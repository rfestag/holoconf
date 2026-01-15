//! SSM Parameter Store resolver
//!
//! Provides the `ssm` resolver for fetching values from AWS Systems Manager Parameter Store.

use std::collections::HashMap;
use std::sync::Arc;

use aws_config::BehaviorVersion;
use aws_sdk_ssm::types::ParameterType;
use holoconf_core::error::{Error, Result};
use holoconf_core::resolver::{register_global, ResolvedValue, Resolver, ResolverContext};
use holoconf_core::Value;
use tokio::runtime::Runtime;

/// SSM Parameter Store resolver.
///
/// Fetches values from AWS Systems Manager Parameter Store.
///
/// ## Usage
///
/// ```yaml
/// database:
///   host: ${ssm:/app/prod/db-host}
///   password: ${ssm:/app/prod/db-password}
/// ```
///
/// ## Parameter Types
///
/// - **String**: Returned as-is
/// - **SecureString**: Automatically marked as sensitive for redaction
/// - **StringList**: Returned as an array (split by comma)
///
/// ## Kwargs
///
/// - `region`: Override the AWS region for this parameter
/// - `profile`: Use a specific AWS profile
/// - `default`: Value to use if parameter not found (framework-handled)
/// - `sensitive`: Override automatic sensitivity detection (framework-handled)
pub struct SsmResolver {
    runtime: Runtime,
}

impl SsmResolver {
    /// Create a new SSM resolver.
    pub fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");
        Self { runtime }
    }

    /// Fetch a parameter from SSM.
    async fn fetch_parameter(
        &self,
        path: &str,
        region: Option<&str>,
        profile: Option<&str>,
    ) -> Result<(String, ParameterType)> {
        // Build AWS config
        let mut config_loader = aws_config::defaults(BehaviorVersion::latest());

        if let Some(region) = region {
            config_loader = config_loader.region(aws_config::Region::new(region.to_string()));
        }

        if let Some(profile) = profile {
            config_loader = config_loader.profile_name(profile);
        }

        let config = config_loader.load().await;
        let client = aws_sdk_ssm::Client::new(&config);

        // Get parameter with decryption
        let response = client
            .get_parameter()
            .name(path)
            .with_decryption(true)
            .send()
            .await
            .map_err(|e| Error::not_found(format!("SSM parameter '{}': {}", path, e), None))?;

        let parameter = response
            .parameter()
            .ok_or_else(|| Error::not_found(format!("SSM parameter '{}'", path), None))?;

        let value = parameter
            .value()
            .ok_or_else(|| {
                Error::not_found(format!("SSM parameter '{}' has no value", path), None)
            })?
            .to_string();

        let param_type = parameter.r#type().cloned().unwrap_or(ParameterType::String);

        Ok((value, param_type))
    }
}

impl Default for SsmResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl Resolver for SsmResolver {
    fn resolve(
        &self,
        args: &[String],
        kwargs: &HashMap<String, String>,
        _ctx: &ResolverContext,
    ) -> Result<ResolvedValue> {
        // Validate args
        if args.is_empty() {
            return Err(Error::resolver_custom(
                "ssm",
                "SSM resolver requires a parameter path argument",
            ));
        }

        let path = &args[0];

        // SSM paths must start with /
        if !path.starts_with('/') {
            return Err(Error::resolver_custom(
                "ssm",
                format!("SSM parameter path must start with /: {}", path),
            ));
        }

        let region = kwargs.get("region").map(|s| s.as_str());
        let profile = kwargs.get("profile").map(|s| s.as_str());

        // Fetch the parameter using the async runtime
        let (value, param_type) = self
            .runtime
            .block_on(self.fetch_parameter(path, region, profile))?;

        // Handle different parameter types
        match param_type {
            ParameterType::SecureString => {
                // SecureString is automatically sensitive
                Ok(ResolvedValue::sensitive(Value::String(value)))
            }
            ParameterType::StringList => {
                // StringList is comma-separated, convert to array
                let values: Vec<Value> = value
                    .split(',')
                    .map(|s| Value::String(s.to_string()))
                    .collect();
                Ok(ResolvedValue::new(Value::Sequence(values)))
            }
            _ => {
                // Regular string
                Ok(ResolvedValue::new(Value::String(value)))
            }
        }
    }

    fn name(&self) -> &str {
        "ssm"
    }
}

/// Register the SSM resolver in the global registry.
pub fn register() {
    let resolver = Arc::new(SsmResolver::new());
    // Use force=true to allow re-registration (e.g., during testing)
    let _ = register_global(resolver, true);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssm_resolver_name() {
        let resolver = SsmResolver::new();
        assert_eq!(resolver.name(), "ssm");
    }

    #[test]
    fn test_ssm_resolver_no_args() {
        let resolver = SsmResolver::new();
        let ctx = ResolverContext::new("test.path");

        let result = resolver.resolve(&[], &HashMap::new(), &ctx);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("requires a parameter path"));
    }

    #[test]
    fn test_ssm_resolver_path_must_start_with_slash() {
        let resolver = SsmResolver::new();
        let ctx = ResolverContext::new("test.path");
        let args = vec!["invalid-path".to_string()];

        let result = resolver.resolve(&args, &HashMap::new(), &ctx);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must start with /"));
    }

    #[test]
    fn test_register_doesnt_panic() {
        // Just verify registration doesn't panic
        register();
    }

    // Note: Integration tests with actual SSM require AWS credentials
    // and should be run separately with appropriate test fixtures
}

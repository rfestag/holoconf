//! CloudFormation outputs resolver
//!
//! Provides the `cfn` resolver for fetching outputs from CloudFormation stacks.

use std::collections::HashMap;
use std::sync::Arc;

use aws_config::BehaviorVersion;
use holoconf_core::error::{Error, Result};
use holoconf_core::resolver::{register_global, ResolvedValue, Resolver, ResolverContext};
use holoconf_core::Value;
use tokio::runtime::Runtime;

/// CloudFormation outputs resolver.
///
/// Fetches outputs from CloudFormation stacks.
///
/// ## Usage
///
/// ```yaml
/// database:
///   endpoint: ${cfn:my-database-stack/DatabaseEndpoint}
///   port: ${cfn:my-database-stack/DatabasePort}
/// ```
///
/// ## Kwargs
///
/// - `region`: Override the AWS region for this lookup
/// - `profile`: Use a specific AWS profile
/// - `default`: Value to use if output not found (framework-handled)
/// - `sensitive`: Mark value as sensitive (framework-handled)
pub struct CfnResolver {
    runtime: Runtime,
}

impl CfnResolver {
    /// Create a new CloudFormation resolver.
    pub fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");
        Self { runtime }
    }

    /// Fetch a stack output from CloudFormation.
    async fn fetch_output(
        &self,
        stack_name: &str,
        output_key: &str,
        region: Option<&str>,
        profile: Option<&str>,
    ) -> Result<String> {
        // Build AWS config
        let mut config_loader = aws_config::defaults(BehaviorVersion::latest());

        if let Some(region) = region {
            config_loader = config_loader.region(aws_config::Region::new(region.to_string()));
        }

        if let Some(profile) = profile {
            config_loader = config_loader.profile_name(profile);
        }

        let config = config_loader.load().await;
        let client = aws_sdk_cloudformation::Client::new(&config);

        // Describe the stack
        let response = client
            .describe_stacks()
            .stack_name(stack_name)
            .send()
            .await
            .map_err(|e| {
                Error::not_found(
                    format!("CloudFormation stack '{}': {}", stack_name, e),
                    None,
                )
            })?;

        // Find the stack
        let stacks = response.stacks();
        let stack = stacks.first().ok_or_else(|| {
            Error::not_found(
                format!("CloudFormation stack '{}' not found", stack_name),
                None,
            )
        })?;

        // Find the output
        let outputs = stack.outputs();
        for output in outputs {
            if output.output_key() == Some(output_key) {
                return output.output_value().map(|s| s.to_string()).ok_or_else(|| {
                    Error::not_found(
                        format!(
                            "CloudFormation output '{}/{}' has no value",
                            stack_name, output_key
                        ),
                        None,
                    )
                });
            }
        }

        Err(Error::not_found(
            format!(
                "CloudFormation output '{}/{}' not found",
                stack_name, output_key
            ),
            None,
        ))
    }
}

impl Default for CfnResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl Resolver for CfnResolver {
    fn resolve(
        &self,
        args: &[String],
        kwargs: &HashMap<String, String>,
        _ctx: &ResolverContext,
    ) -> Result<ResolvedValue> {
        // Validate args
        if args.is_empty() {
            return Err(Error::resolver_custom(
                "cfn",
                "CloudFormation resolver requires a stack-name/OutputKey argument",
            ));
        }

        let arg = &args[0];

        // Parse stack-name/OutputKey format
        let parts: Vec<&str> = arg.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(Error::resolver_custom(
                "cfn",
                format!(
                    "CloudFormation argument must be in stack-name/OutputKey format: {}",
                    arg
                ),
            ));
        }

        let stack_name = parts[0];
        let output_key = parts[1];

        if stack_name.is_empty() || output_key.is_empty() {
            return Err(Error::resolver_custom(
                "cfn",
                format!(
                    "CloudFormation argument must be in stack-name/OutputKey format: {}",
                    arg
                ),
            ));
        }

        let region = kwargs.get("region").map(|s| s.as_str());
        let profile = kwargs.get("profile").map(|s| s.as_str());

        // Fetch the output using the async runtime
        let value = self
            .runtime
            .block_on(self.fetch_output(stack_name, output_key, region, profile))?;

        // CloudFormation outputs are not sensitive by default
        Ok(ResolvedValue::new(Value::String(value)))
    }

    fn name(&self) -> &str {
        "cfn"
    }
}

/// Register the CloudFormation resolver in the global registry.
pub fn register() {
    let resolver = Arc::new(CfnResolver::new());
    // Use force=true to allow re-registration (e.g., during testing)
    let _ = register_global(resolver, true);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cfn_resolver_name() {
        let resolver = CfnResolver::new();
        assert_eq!(resolver.name(), "cfn");
    }

    #[test]
    fn test_cfn_resolver_no_args() {
        let resolver = CfnResolver::new();
        let ctx = ResolverContext::new("test.path");

        let result = resolver.resolve(&[], &HashMap::new(), &ctx);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("stack-name/OutputKey"));
    }

    #[test]
    fn test_cfn_resolver_invalid_format() {
        let resolver = CfnResolver::new();
        let ctx = ResolverContext::new("test.path");
        let args = vec!["invalid-format".to_string()];

        let result = resolver.resolve(&args, &HashMap::new(), &ctx);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("stack-name/OutputKey"));
    }

    #[test]
    fn test_cfn_resolver_empty_stack_name() {
        let resolver = CfnResolver::new();
        let ctx = ResolverContext::new("test.path");
        let args = vec!["/OutputKey".to_string()];

        let result = resolver.resolve(&args, &HashMap::new(), &ctx);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("stack-name/OutputKey"));
    }

    #[test]
    fn test_cfn_resolver_empty_output_key() {
        let resolver = CfnResolver::new();
        let ctx = ResolverContext::new("test.path");
        let args = vec!["stack-name/".to_string()];

        let result = resolver.resolve(&args, &HashMap::new(), &ctx);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("stack-name/OutputKey"));
    }

    #[test]
    fn test_register_doesnt_panic() {
        // Just verify registration doesn't panic
        register();
    }
}

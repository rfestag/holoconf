//! AWS resolvers for holoconf
//!
//! This crate provides AWS-specific resolvers for holoconf configuration.
//!
//! ## SSM Parameter Store Resolver
//!
//! The `ssm` resolver fetches values from AWS Systems Manager Parameter Store.
//!
//! ```yaml
//! database:
//!   password: ${ssm:/app/prod/db-password}
//! ```
//!
//! ## CloudFormation Outputs Resolver
//!
//! The `cfn` resolver fetches outputs from CloudFormation stacks.
//!
//! ```yaml
//! database:
//!   endpoint: ${cfn:my-database-stack/DatabaseEndpoint}
//! ```
//!
//! ## S3 Object Resolver
//!
//! The `s3` resolver fetches objects from Amazon S3.
//!
//! ```yaml
//! config: ${s3:my-bucket/configs/app.yaml}
//! ```

use once_cell::sync::Lazy;
use std::sync::RwLock;

mod client_cache;

#[cfg(feature = "ssm")]
mod ssm;

#[cfg(feature = "cfn")]
mod cfn;

#[cfg(feature = "s3")]
mod s3;

#[cfg(feature = "ssm")]
pub use ssm::SsmResolver;

#[cfg(feature = "cfn")]
pub use cfn::CfnResolver;

#[cfg(feature = "s3")]
pub use s3::S3Resolver;

// =============================================================================
// Configuration
// =============================================================================

/// Global configuration (applies to all AWS services)
#[derive(Clone, Default, Debug)]
struct GlobalConfig {
    region: Option<String>,
    profile: Option<String>,
}

/// S3-specific configuration
#[derive(Clone, Default, Debug)]
pub struct S3Config {
    pub endpoint: Option<String>,
    pub region: Option<String>,
    pub profile: Option<String>,
}

/// SSM-specific configuration
#[derive(Clone, Default, Debug)]
pub struct SsmConfig {
    pub endpoint: Option<String>,
    pub region: Option<String>,
    pub profile: Option<String>,
}

/// CloudFormation-specific configuration
#[derive(Clone, Default, Debug)]
pub struct CfnConfig {
    pub endpoint: Option<String>,
    pub region: Option<String>,
    pub profile: Option<String>,
}

static GLOBAL_CONFIG: Lazy<RwLock<GlobalConfig>> = Lazy::new(Default::default);
static S3_CONFIG: Lazy<RwLock<S3Config>> = Lazy::new(Default::default);
static SSM_CONFIG: Lazy<RwLock<SsmConfig>> = Lazy::new(Default::default);
static CFN_CONFIG: Lazy<RwLock<CfnConfig>> = Lazy::new(Default::default);

/// Configure global defaults for all AWS services.
///
/// Sets default region and profile for all AWS services (S3, SSM, CloudFormation).
/// These can be overridden by service-specific configuration or per-resolver kwargs.
///
/// # Arguments
///
/// * `region` - Default AWS region
/// * `profile` - Default AWS profile name
///
/// # Example
///
/// ```rust,ignore
/// use holoconf_aws;
///
/// // Set global defaults
/// holoconf_aws::configure(Some("us-east-1".to_string()), Some("prod".to_string()));
/// ```
pub fn configure(region: Option<String>, profile: Option<String>) {
    let mut config = GLOBAL_CONFIG.write().unwrap();
    if let Some(r) = region {
        config.region = Some(r);
    }
    if let Some(p) = profile {
        config.profile = Some(p);
    }
}

/// Configure S3-specific defaults.
///
/// Overrides global configuration for the S3 resolver.
///
/// # Arguments
///
/// * `endpoint` - S3 endpoint URL (for moto/LocalStack)
/// * `region` - AWS region
/// * `profile` - AWS profile name
///
/// # Example
///
/// ```rust,ignore
/// use holoconf_aws;
///
/// // Configure S3 for local testing
/// holoconf_aws::configure_s3(
///     Some("http://localhost:5000".to_string()),
///     Some("us-west-2".to_string()),
///     None,
/// );
/// ```
pub fn configure_s3(endpoint: Option<String>, region: Option<String>, profile: Option<String>) {
    let mut config = S3_CONFIG.write().unwrap();
    if let Some(e) = endpoint {
        config.endpoint = Some(e);
    }
    if let Some(r) = region {
        config.region = Some(r);
    }
    if let Some(p) = profile {
        config.profile = Some(p);
    }
}

/// Configure SSM-specific defaults.
///
/// Overrides global configuration for the SSM resolver.
///
/// # Arguments
///
/// * `endpoint` - SSM endpoint URL (for moto/LocalStack)
/// * `region` - AWS region
/// * `profile` - AWS profile name
///
/// # Example
///
/// ```rust,ignore
/// use holoconf_aws;
///
/// // Configure SSM for local testing
/// holoconf_aws::configure_ssm(
///     Some("http://localhost:5001".to_string()),
///     None,
///     None,
/// );
/// ```
pub fn configure_ssm(endpoint: Option<String>, region: Option<String>, profile: Option<String>) {
    let mut config = SSM_CONFIG.write().unwrap();
    if let Some(e) = endpoint {
        config.endpoint = Some(e);
    }
    if let Some(r) = region {
        config.region = Some(r);
    }
    if let Some(p) = profile {
        config.profile = Some(p);
    }
}

/// Configure CloudFormation-specific defaults.
///
/// Overrides global configuration for the CloudFormation resolver.
///
/// # Arguments
///
/// * `endpoint` - CloudFormation endpoint URL (for moto/LocalStack)
/// * `region` - AWS region
/// * `profile` - AWS profile name
///
/// # Example
///
/// ```rust,ignore
/// use holoconf_aws;
///
/// // Configure CloudFormation for local testing
/// holoconf_aws::configure_cfn(
///     Some("http://localhost:5002".to_string()),
///     None,
///     Some("testing".to_string()),
/// );
/// ```
pub fn configure_cfn(endpoint: Option<String>, region: Option<String>, profile: Option<String>) {
    let mut config = CFN_CONFIG.write().unwrap();
    if let Some(e) = endpoint {
        config.endpoint = Some(e);
    }
    if let Some(r) = region {
        config.region = Some(r);
    }
    if let Some(p) = profile {
        config.profile = Some(p);
    }
}

/// Reset all configuration and clear the client cache.
///
/// Clears all global and service-specific configuration, and removes all cached AWS clients.
/// Useful for test isolation.
///
/// # Example
///
/// ```rust,ignore
/// use holoconf_aws;
///
/// // Configure for testing
/// holoconf_aws::configure_s3(Some("http://localhost:5000".to_string()), None, None);
///
/// // Run tests...
///
/// // Reset for next test
/// holoconf_aws::reset();
/// ```
pub fn reset() {
    *GLOBAL_CONFIG.write().unwrap() = Default::default();
    *S3_CONFIG.write().unwrap() = Default::default();
    *SSM_CONFIG.write().unwrap() = Default::default();
    *CFN_CONFIG.write().unwrap() = Default::default();
    client_cache::clear();
}

/// Resolve S3 configuration with precedence.
///
/// Precedence: kwargs > service config > global config
pub(crate) fn resolve_s3_config(
    endpoint_kwarg: Option<&str>,
    region_kwarg: Option<&str>,
    profile_kwarg: Option<&str>,
) -> (Option<String>, Option<String>, Option<String>) {
    let global = GLOBAL_CONFIG.read().unwrap();
    let service = S3_CONFIG.read().unwrap();

    let endpoint = endpoint_kwarg
        .map(String::from)
        .or_else(|| service.endpoint.clone());

    let region = region_kwarg
        .map(String::from)
        .or_else(|| service.region.clone())
        .or_else(|| global.region.clone());

    let profile = profile_kwarg
        .map(String::from)
        .or_else(|| service.profile.clone())
        .or_else(|| global.profile.clone());

    (endpoint, region, profile)
}

/// Resolve SSM configuration with precedence.
///
/// Precedence: kwargs > service config > global config
pub(crate) fn resolve_ssm_config(
    endpoint_kwarg: Option<&str>,
    region_kwarg: Option<&str>,
    profile_kwarg: Option<&str>,
) -> (Option<String>, Option<String>, Option<String>) {
    let global = GLOBAL_CONFIG.read().unwrap();
    let service = SSM_CONFIG.read().unwrap();

    let endpoint = endpoint_kwarg
        .map(String::from)
        .or_else(|| service.endpoint.clone());

    let region = region_kwarg
        .map(String::from)
        .or_else(|| service.region.clone())
        .or_else(|| global.region.clone());

    let profile = profile_kwarg
        .map(String::from)
        .or_else(|| service.profile.clone())
        .or_else(|| global.profile.clone());

    (endpoint, region, profile)
}

/// Resolve CloudFormation configuration with precedence.
///
/// Precedence: kwargs > service config > global config
pub(crate) fn resolve_cfn_config(
    endpoint_kwarg: Option<&str>,
    region_kwarg: Option<&str>,
    profile_kwarg: Option<&str>,
) -> (Option<String>, Option<String>, Option<String>) {
    let global = GLOBAL_CONFIG.read().unwrap();
    let service = CFN_CONFIG.read().unwrap();

    let endpoint = endpoint_kwarg
        .map(String::from)
        .or_else(|| service.endpoint.clone());

    let region = region_kwarg
        .map(String::from)
        .or_else(|| service.region.clone())
        .or_else(|| global.region.clone());

    let profile = profile_kwarg
        .map(String::from)
        .or_else(|| service.profile.clone())
        .or_else(|| global.profile.clone());

    (endpoint, region, profile)
}

// =============================================================================
// Registration
// =============================================================================

/// Register all AWS resolvers in the global registry.
///
/// Call this function at application startup to enable AWS resolvers.
///
/// # Example
///
/// ```rust,ignore
/// holoconf_aws::register_all();
/// ```
pub fn register_all() {
    #[cfg(feature = "ssm")]
    ssm::register();

    #[cfg(feature = "cfn")]
    cfn::register();

    #[cfg(feature = "s3")]
    s3::register();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_all_doesnt_panic() {
        // Just verify registration doesn't panic
        register_all();
    }
}

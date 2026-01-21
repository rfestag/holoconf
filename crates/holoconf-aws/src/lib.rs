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
pub(crate) struct S3Config {
    pub(crate) endpoint: Option<String>,
    pub(crate) region: Option<String>,
    pub(crate) profile: Option<String>,
}

/// SSM-specific configuration
#[derive(Clone, Default, Debug)]
pub(crate) struct SsmConfig {
    pub(crate) endpoint: Option<String>,
    pub(crate) region: Option<String>,
    pub(crate) profile: Option<String>,
}

/// CloudFormation-specific configuration
#[derive(Clone, Default, Debug)]
pub(crate) struct CfnConfig {
    pub(crate) endpoint: Option<String>,
    pub(crate) region: Option<String>,
    pub(crate) profile: Option<String>,
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
/// Configuration precedence (highest to lowest):
/// 1. Resolver kwargs in config file (e.g., `${s3:bucket/file,region=us-east-1}`)
/// 2. Service-specific configuration (`configure_s3()`, `configure_ssm()`, `configure_cfn()`)
/// 3. Global configuration (`configure()`)
/// 4. AWS SDK defaults (environment variables, credentials file)
///
/// # Arguments
///
/// * `region` - Default AWS region (e.g., "us-east-1"). Pass `None` to leave unchanged.
/// * `profile` - Default AWS profile name. Pass `None` to leave unchanged.
///
/// # Example
///
/// ```rust,ignore
/// use holoconf_aws;
///
/// // Set global defaults
/// holoconf_aws::configure(Some("us-east-1".to_string()), Some("prod".to_string()));
///
/// // Update only region, profile remains unchanged
/// holoconf_aws::configure(Some("us-west-2".to_string()), None);
/// ```
pub fn configure(region: Option<String>, profile: Option<String>) {
    // Panic on lock poisoning is acceptable - indicates a panic during config update,
    // which means the program is already in an inconsistent state
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
/// * `endpoint` - S3 endpoint URL (for moto/LocalStack, e.g., "http://localhost:5000"). Pass `None` to leave unchanged.
/// * `region` - AWS region (overrides global region). Pass `None` to leave unchanged.
/// * `profile` - AWS profile name (overrides global profile). Pass `None` to leave unchanged.
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
    // Panic on lock poisoning is acceptable - indicates a panic during config update,
    // which means the program is already in an inconsistent state
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
/// * `endpoint` - SSM endpoint URL (for moto/LocalStack, e.g., "http://localhost:5001"). Pass `None` to leave unchanged.
/// * `region` - AWS region (overrides global region). Pass `None` to leave unchanged.
/// * `profile` - AWS profile name (overrides global profile). Pass `None` to leave unchanged.
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
    // Panic on lock poisoning is acceptable - indicates a panic during config update,
    // which means the program is already in an inconsistent state
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
/// * `endpoint` - CloudFormation endpoint URL (for moto/LocalStack, e.g., "http://localhost:5002"). Pass `None` to leave unchanged.
/// * `region` - AWS region (overrides global region). Pass `None` to leave unchanged.
/// * `profile` - AWS profile name (overrides global profile). Pass `None` to leave unchanged.
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
    // Panic on lock poisoning is acceptable - indicates a panic during config update,
    // which means the program is already in an inconsistent state
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
    // Panic on lock poisoning is acceptable - indicates a panic during config update,
    // which means the program is already in an inconsistent state
    *GLOBAL_CONFIG.write().unwrap() = Default::default();
    *S3_CONFIG.write().unwrap() = Default::default();
    *SSM_CONFIG.write().unwrap() = Default::default();
    *CFN_CONFIG.write().unwrap() = Default::default();
    client_cache::clear();
}

/// Helper trait to access service-specific configuration.
trait ServiceConfig {
    fn endpoint(&self) -> &Option<String>;
    fn region(&self) -> &Option<String>;
    fn profile(&self) -> &Option<String>;
}

impl ServiceConfig for S3Config {
    fn endpoint(&self) -> &Option<String> {
        &self.endpoint
    }
    fn region(&self) -> &Option<String> {
        &self.region
    }
    fn profile(&self) -> &Option<String> {
        &self.profile
    }
}

impl ServiceConfig for SsmConfig {
    fn endpoint(&self) -> &Option<String> {
        &self.endpoint
    }
    fn region(&self) -> &Option<String> {
        &self.region
    }
    fn profile(&self) -> &Option<String> {
        &self.profile
    }
}

impl ServiceConfig for CfnConfig {
    fn endpoint(&self) -> &Option<String> {
        &self.endpoint
    }
    fn region(&self) -> &Option<String> {
        &self.region
    }
    fn profile(&self) -> &Option<String> {
        &self.profile
    }
}

/// Generic resolver for configuration with precedence.
///
/// Precedence: kwargs > service config > global config
///
/// This function minimizes allocations by:
/// 1. Early return if all kwargs provided (no config access needed)
/// 2. Releasing locks before applying precedence logic (reduces lock contention)
/// 3. Only cloning values that will actually be used in the precedence chain
fn resolve_config_with_precedence<T: ServiceConfig>(
    service_config: &RwLock<T>,
    endpoint_kwarg: Option<&str>,
    region_kwarg: Option<&str>,
    profile_kwarg: Option<&str>,
) -> (Option<String>, Option<String>, Option<String>) {
    // Early return if all kwargs provided - no need to read config
    if endpoint_kwarg.is_some() && region_kwarg.is_some() && profile_kwarg.is_some() {
        return (
            endpoint_kwarg.map(String::from),
            region_kwarg.map(String::from),
            profile_kwarg.map(String::from),
        );
    }

    // Acquire locks and extract values, then drop locks immediately
    let (endpoint_service, region_service, profile_service, region_global, profile_global) = {
        let global = GLOBAL_CONFIG.read().unwrap();
        let service = service_config.read().unwrap();

        (
            service.endpoint().clone(),
            service.region().clone(),
            service.profile().clone(),
            global.region.clone(),
            global.profile.clone(),
        )
        // Locks dropped here
    };

    // Apply precedence without holding any locks
    let endpoint = endpoint_kwarg.map(String::from).or(endpoint_service);

    let region = region_kwarg
        .map(String::from)
        .or(region_service)
        .or(region_global);

    let profile = profile_kwarg
        .map(String::from)
        .or(profile_service)
        .or(profile_global);

    (endpoint, region, profile)
}

/// Resolve S3 configuration with precedence.
///
/// Precedence: kwargs > service config > global config
pub(crate) fn resolve_s3_config(
    endpoint_kwarg: Option<&str>,
    region_kwarg: Option<&str>,
    profile_kwarg: Option<&str>,
) -> (Option<String>, Option<String>, Option<String>) {
    resolve_config_with_precedence(&S3_CONFIG, endpoint_kwarg, region_kwarg, profile_kwarg)
}

/// Resolve SSM configuration with precedence.
///
/// Precedence: kwargs > service config > global config
pub(crate) fn resolve_ssm_config(
    endpoint_kwarg: Option<&str>,
    region_kwarg: Option<&str>,
    profile_kwarg: Option<&str>,
) -> (Option<String>, Option<String>, Option<String>) {
    resolve_config_with_precedence(&SSM_CONFIG, endpoint_kwarg, region_kwarg, profile_kwarg)
}

/// Resolve CloudFormation configuration with precedence.
///
/// Precedence: kwargs > service config > global config
pub(crate) fn resolve_cfn_config(
    endpoint_kwarg: Option<&str>,
    region_kwarg: Option<&str>,
    profile_kwarg: Option<&str>,
) -> (Option<String>, Option<String>, Option<String>) {
    resolve_config_with_precedence(&CFN_CONFIG, endpoint_kwarg, region_kwarg, profile_kwarg)
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

    #[test]
    fn test_s3_precedence_kwarg_over_service() {
        // Setup
        configure_s3(
            Some("http://service-endpoint".to_string()),
            Some("us-west-2".to_string()),
            Some("service-profile".to_string()),
        );

        // Test: kwargs should override service config
        let (endpoint, region, profile) = resolve_s3_config(
            Some("http://kwarg-endpoint"),
            Some("eu-west-1"),
            Some("kwarg-profile"),
        );

        assert_eq!(endpoint, Some("http://kwarg-endpoint".to_string()));
        assert_eq!(region, Some("eu-west-1".to_string()));
        assert_eq!(profile, Some("kwarg-profile".to_string()));

        // Cleanup
        reset();
    }

    #[test]
    fn test_s3_precedence_service_over_global() {
        // Setup
        configure(
            Some("us-east-1".to_string()),
            Some("global-profile".to_string()),
        );
        configure_s3(
            None,
            Some("us-west-2".to_string()),
            Some("service-profile".to_string()),
        );

        // Test: service config should override global config
        let (_, region, profile) = resolve_s3_config(None, None, None);

        assert_eq!(region, Some("us-west-2".to_string()));
        assert_eq!(profile, Some("service-profile".to_string()));

        // Cleanup
        reset();
    }

    #[test]
    fn test_s3_precedence_global_fallback() {
        // Setup
        configure(
            Some("us-east-1".to_string()),
            Some("global-profile".to_string()),
        );

        // Test: global config used when no service or kwarg config
        let (endpoint, region, profile) = resolve_s3_config(None, None, None);

        assert_eq!(endpoint, None);
        assert_eq!(region, Some("us-east-1".to_string()));
        assert_eq!(profile, Some("global-profile".to_string()));

        // Cleanup
        reset();
    }

    #[test]
    fn test_s3_precedence_partial_override() {
        // Setup
        configure(
            Some("us-east-1".to_string()),
            Some("global-profile".to_string()),
        );
        configure_s3(None, Some("us-west-2".to_string()), None);

        // Test: partial override - region from service, profile from global
        let (_, region, profile) = resolve_s3_config(None, None, None);

        assert_eq!(region, Some("us-west-2".to_string()));
        assert_eq!(profile, Some("global-profile".to_string()));

        // Cleanup
        reset();
    }

    #[test]
    fn test_ssm_precedence_full_chain() {
        // Setup: all three levels configured
        configure(
            Some("us-east-1".to_string()),
            Some("global-profile".to_string()),
        );
        configure_ssm(
            Some("http://service-endpoint".to_string()),
            Some("us-west-2".to_string()),
            Some("service-profile".to_string()),
        );

        // Test 1: kwarg overrides everything
        let (endpoint, region, profile) = resolve_ssm_config(
            Some("http://kwarg-endpoint"),
            Some("eu-west-1"),
            Some("kwarg-profile"),
        );
        assert_eq!(endpoint, Some("http://kwarg-endpoint".to_string()));
        assert_eq!(region, Some("eu-west-1".to_string()));
        assert_eq!(profile, Some("kwarg-profile".to_string()));

        // Test 2: service overrides global
        let (endpoint, region, profile) = resolve_ssm_config(None, None, None);
        assert_eq!(endpoint, Some("http://service-endpoint".to_string()));
        assert_eq!(region, Some("us-west-2".to_string()));
        assert_eq!(profile, Some("service-profile".to_string()));

        // Cleanup
        reset();
    }

    #[test]
    fn test_cfn_precedence_none_values_preserved() {
        // Setup: configure only region at global level
        configure(Some("us-east-1".to_string()), None);

        // Test: None values remain None through all levels
        let (endpoint, region, profile) = resolve_cfn_config(None, None, None);

        assert_eq!(endpoint, None);
        assert_eq!(region, Some("us-east-1".to_string()));
        assert_eq!(profile, None);

        // Cleanup
        reset();
    }

    #[test]
    fn test_reset_clears_all_config() {
        // Setup: configure everything
        configure(
            Some("us-east-1".to_string()),
            Some("global-profile".to_string()),
        );
        configure_s3(
            Some("http://localhost:5000".to_string()),
            Some("us-west-2".to_string()),
            Some("s3-profile".to_string()),
        );
        configure_ssm(
            Some("http://localhost:5001".to_string()),
            Some("eu-west-1".to_string()),
            None,
        );
        configure_cfn(None, None, Some("cfn-profile".to_string()));

        // Test: reset clears everything
        reset();

        let (s3_endpoint, s3_region, s3_profile) = resolve_s3_config(None, None, None);
        assert_eq!(s3_endpoint, None);
        assert_eq!(s3_region, None);
        assert_eq!(s3_profile, None);

        let (ssm_endpoint, ssm_region, ssm_profile) = resolve_ssm_config(None, None, None);
        assert_eq!(ssm_endpoint, None);
        assert_eq!(ssm_region, None);
        assert_eq!(ssm_profile, None);

        let (cfn_endpoint, cfn_region, cfn_profile) = resolve_cfn_config(None, None, None);
        assert_eq!(cfn_endpoint, None);
        assert_eq!(cfn_region, None);
        assert_eq!(cfn_profile, None);
    }

    #[test]
    fn test_configure_none_leaves_unchanged() {
        // Setup: initial configuration
        configure(
            Some("us-east-1".to_string()),
            Some("initial-profile".to_string()),
        );

        // Test: calling with None leaves values unchanged
        configure(None, None);

        let (_, region, profile) = resolve_s3_config(None, None, None);
        assert_eq!(region, Some("us-east-1".to_string()));
        assert_eq!(profile, Some("initial-profile".to_string()));

        // Cleanup
        reset();
    }

    #[test]
    fn test_service_config_none_leaves_unchanged() {
        // Setup: initial service configuration
        configure_s3(
            Some("http://initial-endpoint".to_string()),
            Some("us-west-2".to_string()),
            Some("initial-profile".to_string()),
        );

        // Test: calling with None leaves values unchanged
        configure_s3(None, None, None);

        let (endpoint, region, profile) = resolve_s3_config(None, None, None);
        assert_eq!(endpoint, Some("http://initial-endpoint".to_string()));
        assert_eq!(region, Some("us-west-2".to_string()));
        assert_eq!(profile, Some("initial-profile".to_string()));

        // Cleanup
        reset();
    }

    #[test]
    fn test_service_config_partial_update() {
        // Setup: initial service configuration
        configure_s3(
            Some("http://initial-endpoint".to_string()),
            Some("us-west-2".to_string()),
            Some("initial-profile".to_string()),
        );

        // Test: update only region, others remain unchanged
        configure_s3(None, Some("eu-west-1".to_string()), None);

        let (endpoint, region, profile) = resolve_s3_config(None, None, None);
        assert_eq!(endpoint, Some("http://initial-endpoint".to_string()));
        assert_eq!(region, Some("eu-west-1".to_string()));
        assert_eq!(profile, Some("initial-profile".to_string()));

        // Cleanup
        reset();
    }
}

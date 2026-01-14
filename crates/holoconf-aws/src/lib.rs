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
//! ### Syntax
//!
//! ```text
//! ${ssm:/path/to/parameter}
//! ${ssm:/path/to/parameter,default=fallback}
//! ${ssm:/path/to/parameter,region=us-west-2}
//! ${ssm:/path/to/parameter,profile=production}
//! ```
//!
//! ### Features
//!
//! - **Automatic sensitivity**: SecureString parameters are automatically marked as sensitive
//! - **StringList support**: StringList parameters are returned as arrays
//! - **Region/Profile**: Override AWS region or profile per-parameter

#[cfg(feature = "ssm")]
mod ssm;

#[cfg(feature = "ssm")]
pub use ssm::SsmResolver;

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

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

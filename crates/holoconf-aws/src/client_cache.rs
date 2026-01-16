//! Shared AWS client cache for all resolvers.
//!
//! Caches actual service clients (not just SdkConfig) to enable connection pool reuse.
//! Each unique (service, region, profile) combination gets its own cached client.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::RwLock;

use aws_config::{BehaviorVersion, SdkConfig};
use once_cell::sync::Lazy;

/// Cache key: (service TypeId, region, profile)
type CacheKey = (TypeId, Option<String>, Option<String>);

/// Global cache storing type-erased AWS clients.
static CLIENT_CACHE: Lazy<RwLock<HashMap<CacheKey, Box<dyn Any + Send + Sync>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Trait for AWS service clients that can be cached.
///
/// Implement this trait for each AWS service client type to enable caching.
pub trait AwsClient: Clone + Send + Sync + 'static {
    /// Create a new client from an AWS SDK config.
    fn from_sdk_config(config: &SdkConfig) -> Self;
}

#[cfg(feature = "ssm")]
impl AwsClient for aws_sdk_ssm::Client {
    fn from_sdk_config(config: &SdkConfig) -> Self {
        Self::new(config)
    }
}

#[cfg(feature = "cfn")]
impl AwsClient for aws_sdk_cloudformation::Client {
    fn from_sdk_config(config: &SdkConfig) -> Self {
        Self::new(config)
    }
}

#[cfg(feature = "s3")]
impl AwsClient for aws_sdk_s3::Client {
    fn from_sdk_config(config: &SdkConfig) -> Self {
        Self::new(config)
    }
}

/// Get or create an AWS client for the given region/profile.
///
/// Clients are cached and reused, including their HTTP connection pools.
/// The client type is inferred from the return type annotation.
///
/// # Example
///
/// ```ignore
/// let client: aws_sdk_ssm::Client = get_client(Some("us-west-2"), None).await;
/// ```
pub async fn get_client<C: AwsClient>(region: Option<&str>, profile: Option<&str>) -> C {
    let key = (
        TypeId::of::<C>(),
        region.map(|s| s.to_string()),
        profile.map(|s| s.to_string()),
    );

    // Try read lock first (fast path for cached clients)
    {
        let cache = CLIENT_CACHE.read().unwrap();
        if let Some(boxed) = cache.get(&key) {
            if let Some(client) = boxed.downcast_ref::<C>() {
                return client.clone();
            }
        }
    }

    // Build new client (slow path - only on first access)
    let mut config_loader = aws_config::defaults(BehaviorVersion::latest());

    if let Some(region) = region {
        config_loader = config_loader.region(aws_config::Region::new(region.to_string()));
    }

    if let Some(profile) = profile {
        config_loader = config_loader.profile_name(profile);
    }

    let sdk_config = config_loader.load().await;
    let client = C::from_sdk_config(&sdk_config);

    // Cache for future use
    {
        let mut cache = CLIENT_CACHE.write().unwrap();
        // Double-check after acquiring write lock (another thread may have inserted)
        cache.entry(key).or_insert_with(|| Box::new(client.clone()));
    }

    client
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ssm_client_cached() {
        // First call creates the client
        let client1: aws_sdk_ssm::Client = get_client(None, None).await;
        // Second call should return cached client
        let client2: aws_sdk_ssm::Client = get_client(None, None).await;

        // Both should succeed - clients are cloned from cache
        // We can't directly compare clients, but we can verify they're valid
        drop(client1);
        drop(client2);
    }

    #[tokio::test]
    async fn test_different_regions_different_clients() {
        let client1: aws_sdk_ssm::Client = get_client(Some("us-east-1"), None).await;
        let client2: aws_sdk_ssm::Client = get_client(Some("us-west-2"), None).await;

        // Each should have the correct region
        assert_eq!(client1.config().region().unwrap().as_ref(), "us-east-1");
        assert_eq!(client2.config().region().unwrap().as_ref(), "us-west-2");
    }

    #[tokio::test]
    async fn test_same_region_same_client() {
        let client1: aws_sdk_ssm::Client = get_client(Some("us-east-1"), None).await;
        let client2: aws_sdk_ssm::Client = get_client(Some("us-east-1"), None).await;

        // Both should have the same region (cached)
        assert_eq!(client1.config().region().unwrap().as_ref(), "us-east-1");
        assert_eq!(client2.config().region().unwrap().as_ref(), "us-east-1");
    }
}

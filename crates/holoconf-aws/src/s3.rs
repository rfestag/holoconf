//! S3 object resolver
//!
//! Provides the `s3` resolver for fetching objects from Amazon S3.

use std::collections::HashMap;
use std::sync::Arc;

use aws_sdk_s3::primitives::ByteStream;
use holoconf_core::error::{Error, Result};
use holoconf_core::resolver::{register_global, ResolvedValue, Resolver, ResolverContext};
use holoconf_core::Value;
use tokio::runtime::Runtime;

use crate::client_cache;

/// S3 object resolver.
///
/// Fetches objects from Amazon S3.
///
/// ## Usage
///
/// ```yaml
/// config: ${s3:my-bucket/configs/app.yaml}
/// readme: ${s3:my-bucket/docs/README.md,parse=text}
/// cert: ${s3:my-bucket/certs/ca.pem,parse=binary}
/// ```
///
/// ## Parse Modes
///
/// - `auto` (default): Detect by object key extension or Content-Type
/// - `yaml`: Parse as YAML
/// - `json`: Parse as JSON
/// - `text`: Return raw text content
/// - `binary`: Return raw bytes (useful for certificates, images, etc.)
///
/// ## Kwargs
///
/// - `parse`: How to interpret content (auto, yaml, json, text, binary)
/// - `region`: Override the AWS region for this lookup
/// - `profile`: Use a specific AWS profile
/// - `default`: Value to use if object not found (framework-handled)
/// - `sensitive`: Mark value as sensitive (framework-handled)
pub struct S3Resolver {
    runtime: Runtime,
}

impl S3Resolver {
    /// Create a new S3 resolver.
    pub fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");
        Self { runtime }
    }

    /// Fetch an object from S3 as raw bytes.
    async fn fetch_object_bytes(
        &self,
        bucket: &str,
        key: &str,
        region: Option<&str>,
        profile: Option<&str>,
    ) -> Result<(Vec<u8>, Option<String>)> {
        // Get cached client (creates one if needed)
        let client: aws_sdk_s3::Client = client_cache::get_client(region, profile).await;

        // Get the object
        let response = client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                Error::not_found(format!("S3 object '{}/{}': {}", bucket, key, e), None)
            })?;

        let content_type = response.content_type().map(|s| s.to_string());

        // Read the body as raw bytes
        let body: ByteStream = response.body;
        let bytes = body.collect().await.map_err(|e| {
            Error::resolver_custom("s3", format!("Failed to read S3 object body: {}", e))
        })?;

        Ok((bytes.into_bytes().to_vec(), content_type))
    }

    /// Determine parse mode from kwargs, key extension, or content type.
    fn determine_parse_mode(
        &self,
        parse_kwarg: Option<&str>,
        key: &str,
        content_type: Option<&str>,
    ) -> ParseMode {
        // Explicit parse mode takes precedence
        if let Some(mode) = parse_kwarg {
            return match mode.to_lowercase().as_str() {
                "yaml" => ParseMode::Yaml,
                "json" => ParseMode::Json,
                "text" => ParseMode::Text,
                "binary" => ParseMode::Binary,
                _ => ParseMode::Auto,
            };
        }

        // Auto-detect from extension
        let extension = key.rsplit('.').next().map(|s| s.to_lowercase());
        match extension.as_deref() {
            Some("yaml" | "yml") => return ParseMode::Yaml,
            Some("json") => return ParseMode::Json,
            _ => {}
        }

        // Auto-detect from content type
        if let Some(ct) = content_type {
            let ct_lower = ct.to_lowercase();
            if ct_lower.contains("yaml") {
                return ParseMode::Yaml;
            }
            if ct_lower.contains("json") {
                return ParseMode::Json;
            }
        }

        // Default to text
        ParseMode::Text
    }

    /// Parse bytes content based on the parse mode.
    fn parse_bytes(&self, bytes: Vec<u8>, mode: ParseMode) -> Result<Value> {
        match mode {
            ParseMode::Binary => Ok(Value::Bytes(bytes)),
            ParseMode::Yaml => {
                let content = String::from_utf8(bytes).map_err(|e| {
                    Error::resolver_custom("s3", format!("S3 object is not valid UTF-8: {}", e))
                })?;
                serde_yaml::from_str(&content).map_err(|e| {
                    Error::resolver_custom("s3", format!("Failed to parse YAML: {}", e))
                })
            }
            ParseMode::Json => {
                let content = String::from_utf8(bytes).map_err(|e| {
                    Error::resolver_custom("s3", format!("S3 object is not valid UTF-8: {}", e))
                })?;
                serde_json::from_str(&content).map_err(|e| {
                    Error::resolver_custom("s3", format!("Failed to parse JSON: {}", e))
                })
            }
            ParseMode::Text | ParseMode::Auto => {
                let content = String::from_utf8(bytes).map_err(|e| {
                    Error::resolver_custom("s3", format!("S3 object is not valid UTF-8: {}", e))
                })?;
                Ok(Value::String(content))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ParseMode {
    Auto,
    Yaml,
    Json,
    Text,
    Binary,
}

impl Default for S3Resolver {
    fn default() -> Self {
        Self::new()
    }
}

impl Resolver for S3Resolver {
    fn resolve(
        &self,
        args: &[String],
        kwargs: &HashMap<String, String>,
        _ctx: &ResolverContext,
    ) -> Result<ResolvedValue> {
        // Validate args
        if args.is_empty() {
            return Err(Error::resolver_custom(
                "s3",
                "S3 resolver requires a bucket/key argument",
            ));
        }

        let arg = &args[0];

        // Parse bucket/key format (first / separates bucket from key)
        let parts: Vec<&str> = arg.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(Error::resolver_custom(
                "s3",
                format!("S3 argument must be in bucket/key format: {}", arg),
            ));
        }

        let bucket = parts[0];
        let key = parts[1];

        if bucket.is_empty() || key.is_empty() {
            return Err(Error::resolver_custom(
                "s3",
                format!("S3 argument must be in bucket/key format: {}", arg),
            ));
        }

        let region = kwargs.get("region").map(|s| s.as_str());
        let profile = kwargs.get("profile").map(|s| s.as_str());
        let parse_kwarg = kwargs.get("parse").map(|s| s.as_str());

        // Fetch the object as raw bytes using the async runtime
        let (bytes, content_type) = self
            .runtime
            .block_on(self.fetch_object_bytes(bucket, key, region, profile))?;

        // Determine parse mode and parse content
        let mode = self.determine_parse_mode(parse_kwarg, key, content_type.as_deref());
        let value = self.parse_bytes(bytes, mode)?;

        // S3 objects are not sensitive by default
        Ok(ResolvedValue::new(value))
    }

    fn name(&self) -> &str {
        "s3"
    }
}

/// Register the S3 resolver in the global registry.
pub fn register() {
    let resolver = Arc::new(S3Resolver::new());
    // Use force=true to allow re-registration (e.g., during testing)
    let _ = register_global(resolver, true);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s3_resolver_name() {
        let resolver = S3Resolver::new();
        assert_eq!(resolver.name(), "s3");
    }

    #[test]
    fn test_s3_resolver_no_args() {
        let resolver = S3Resolver::new();
        let ctx = ResolverContext::new("test.path");

        let result = resolver.resolve(&[], &HashMap::new(), &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("bucket/key"));
    }

    #[test]
    fn test_s3_resolver_invalid_format() {
        let resolver = S3Resolver::new();
        let ctx = ResolverContext::new("test.path");
        let args = vec!["invalid".to_string()];

        let result = resolver.resolve(&args, &HashMap::new(), &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("bucket/key"));
    }

    #[test]
    fn test_s3_resolver_empty_bucket() {
        let resolver = S3Resolver::new();
        let ctx = ResolverContext::new("test.path");
        let args = vec!["/key".to_string()];

        let result = resolver.resolve(&args, &HashMap::new(), &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("bucket/key"));
    }

    #[test]
    fn test_s3_resolver_empty_key() {
        let resolver = S3Resolver::new();
        let ctx = ResolverContext::new("test.path");
        let args = vec!["bucket/".to_string()];

        let result = resolver.resolve(&args, &HashMap::new(), &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("bucket/key"));
    }

    #[test]
    fn test_determine_parse_mode_explicit() {
        let resolver = S3Resolver::new();

        assert_eq!(
            resolver.determine_parse_mode(Some("yaml"), "file.txt", None),
            ParseMode::Yaml
        );
        assert_eq!(
            resolver.determine_parse_mode(Some("json"), "file.txt", None),
            ParseMode::Json
        );
        assert_eq!(
            resolver.determine_parse_mode(Some("text"), "file.yaml", None),
            ParseMode::Text
        );
        assert_eq!(
            resolver.determine_parse_mode(Some("binary"), "file.yaml", None),
            ParseMode::Binary
        );
    }

    #[test]
    fn test_determine_parse_mode_extension() {
        let resolver = S3Resolver::new();

        assert_eq!(
            resolver.determine_parse_mode(None, "config.yaml", None),
            ParseMode::Yaml
        );
        assert_eq!(
            resolver.determine_parse_mode(None, "config.yml", None),
            ParseMode::Yaml
        );
        assert_eq!(
            resolver.determine_parse_mode(None, "config.json", None),
            ParseMode::Json
        );
        assert_eq!(
            resolver.determine_parse_mode(None, "readme.txt", None),
            ParseMode::Text
        );
    }

    #[test]
    fn test_determine_parse_mode_content_type() {
        let resolver = S3Resolver::new();

        assert_eq!(
            resolver.determine_parse_mode(None, "file", Some("application/x-yaml")),
            ParseMode::Yaml
        );
        assert_eq!(
            resolver.determine_parse_mode(None, "file", Some("application/json")),
            ParseMode::Json
        );
        assert_eq!(
            resolver.determine_parse_mode(None, "file", Some("text/plain")),
            ParseMode::Text
        );
    }

    #[test]
    fn test_parse_bytes_yaml() {
        let resolver = S3Resolver::new();
        let bytes = b"key: value".to_vec();
        let result = resolver.parse_bytes(bytes, ParseMode::Yaml).unwrap();

        match result {
            Value::Mapping(map) => {
                assert!(map.contains_key("key"));
                assert_eq!(map.get("key"), Some(&Value::String("value".to_string())));
            }
            _ => panic!("Expected mapping"),
        }
    }

    #[test]
    fn test_parse_bytes_json() {
        let resolver = S3Resolver::new();
        let bytes = br#"{"key": "value"}"#.to_vec();
        let result = resolver.parse_bytes(bytes, ParseMode::Json).unwrap();

        match result {
            Value::Mapping(map) => {
                assert!(map.contains_key("key"));
                assert_eq!(map.get("key"), Some(&Value::String("value".to_string())));
            }
            _ => panic!("Expected mapping"),
        }
    }

    #[test]
    fn test_parse_bytes_text() {
        let resolver = S3Resolver::new();
        let bytes = b"plain text content".to_vec();
        let result = resolver.parse_bytes(bytes, ParseMode::Text).unwrap();

        assert_eq!(result, Value::String("plain text content".to_string()));
    }

    #[test]
    fn test_parse_bytes_binary() {
        let resolver = S3Resolver::new();
        let bytes = vec![0x00, 0x01, 0x02, 0xFF]; // Binary data including non-UTF-8 bytes
        let result = resolver
            .parse_bytes(bytes.clone(), ParseMode::Binary)
            .unwrap();

        assert_eq!(result, Value::Bytes(bytes));
    }

    #[test]
    fn test_register_doesnt_panic() {
        // Just verify registration doesn't panic
        register();
    }
}

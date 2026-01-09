//! Resolver architecture per ADR-002
//!
//! Resolvers are functions or objects that resolve interpolation expressions
//! like `${env:VAR}` to actual values.

use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{Error, Result};
use crate::value::Value;

/// A resolved value with optional sensitivity metadata
#[derive(Debug, Clone)]
pub struct ResolvedValue {
    /// The actual resolved value
    pub value: Value,
    /// Whether this value is sensitive (should be redacted in logs/exports)
    pub sensitive: bool,
}

impl ResolvedValue {
    /// Create a non-sensitive resolved value
    pub fn new(value: impl Into<Value>) -> Self {
        Self {
            value: value.into(),
            sensitive: false,
        }
    }

    /// Create a sensitive resolved value
    pub fn sensitive(value: impl Into<Value>) -> Self {
        Self {
            value: value.into(),
            sensitive: true,
        }
    }
}

impl From<Value> for ResolvedValue {
    fn from(value: Value) -> Self {
        ResolvedValue::new(value)
    }
}

impl From<String> for ResolvedValue {
    fn from(s: String) -> Self {
        ResolvedValue::new(Value::String(s))
    }
}

impl From<&str> for ResolvedValue {
    fn from(s: &str) -> Self {
        ResolvedValue::new(Value::String(s.to_string()))
    }
}

/// Context provided to resolvers during resolution
#[derive(Debug, Clone)]
pub struct ResolverContext {
    /// The path in the config where this resolution is happening
    pub config_path: String,
    /// The config root (for self-references)
    pub config_root: Option<Arc<Value>>,
    /// The base path for relative file paths
    pub base_path: Option<std::path::PathBuf>,
    /// Resolution stack for circular reference detection
    pub resolution_stack: Vec<String>,
}

impl ResolverContext {
    /// Create a new resolver context
    pub fn new(config_path: impl Into<String>) -> Self {
        Self {
            config_path: config_path.into(),
            config_root: None,
            base_path: None,
            resolution_stack: Vec::new(),
        }
    }

    /// Set the config root for self-references
    pub fn with_config_root(mut self, root: Arc<Value>) -> Self {
        self.config_root = Some(root);
        self
    }

    /// Set the base path for file resolution
    pub fn with_base_path(mut self, path: std::path::PathBuf) -> Self {
        self.base_path = Some(path);
        self
    }

    /// Check if resolving a path would cause a circular reference
    pub fn would_cause_cycle(&self, path: &str) -> bool {
        self.resolution_stack.contains(&path.to_string())
    }

    /// Push a path onto the resolution stack
    pub fn push_resolution(&mut self, path: &str) {
        self.resolution_stack.push(path.to_string());
    }

    /// Pop a path from the resolution stack
    pub fn pop_resolution(&mut self) {
        self.resolution_stack.pop();
    }

    /// Get the resolution chain for error reporting
    pub fn get_resolution_chain(&self) -> Vec<String> {
        self.resolution_stack.clone()
    }
}

/// Trait for resolver implementations
pub trait Resolver: Send + Sync {
    /// Resolve an interpolation expression
    ///
    /// # Arguments
    /// * `args` - Positional arguments from the interpolation
    /// * `kwargs` - Keyword arguments from the interpolation
    /// * `ctx` - Resolution context
    fn resolve(
        &self,
        args: &[String],
        kwargs: &HashMap<String, String>,
        ctx: &ResolverContext,
    ) -> Result<ResolvedValue>;

    /// Get the name of this resolver
    fn name(&self) -> &str;
}

/// A simple function-based resolver
pub struct FnResolver<F>
where
    F: Fn(&[String], &HashMap<String, String>, &ResolverContext) -> Result<ResolvedValue>
        + Send
        + Sync,
{
    name: String,
    func: F,
}

impl<F> FnResolver<F>
where
    F: Fn(&[String], &HashMap<String, String>, &ResolverContext) -> Result<ResolvedValue>
        + Send
        + Sync,
{
    /// Create a new function-based resolver
    pub fn new(name: impl Into<String>, func: F) -> Self {
        Self {
            name: name.into(),
            func,
        }
    }
}

impl<F> Resolver for FnResolver<F>
where
    F: Fn(&[String], &HashMap<String, String>, &ResolverContext) -> Result<ResolvedValue>
        + Send
        + Sync,
{
    fn resolve(
        &self,
        args: &[String],
        kwargs: &HashMap<String, String>,
        ctx: &ResolverContext,
    ) -> Result<ResolvedValue> {
        (self.func)(args, kwargs, ctx)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Registry of available resolvers
pub struct ResolverRegistry {
    resolvers: HashMap<String, Arc<dyn Resolver>>,
}

impl Default for ResolverRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ResolverRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            resolvers: HashMap::new(),
        }
    }

    /// Create a registry with the standard built-in resolvers
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        registry.register_builtin_resolvers();
        registry
    }

    /// Register the built-in resolvers (env, file, http)
    fn register_builtin_resolvers(&mut self) {
        // Environment variable resolver
        self.register(Arc::new(FnResolver::new("env", env_resolver)));
        // File resolver
        self.register(Arc::new(FnResolver::new("file", file_resolver)));
        // HTTP resolver (disabled by default for security)
        self.register(Arc::new(FnResolver::new("http", http_resolver)));
    }

    /// Register a resolver
    pub fn register(&mut self, resolver: Arc<dyn Resolver>) {
        self.resolvers.insert(resolver.name().to_string(), resolver);
    }

    /// Register a function as a resolver
    pub fn register_fn<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: Fn(&[String], &HashMap<String, String>, &ResolverContext) -> Result<ResolvedValue>
            + Send
            + Sync
            + 'static,
    {
        let name = name.into();
        self.register(Arc::new(FnResolver::new(name, func)));
    }

    /// Get a resolver by name
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Resolver>> {
        self.resolvers.get(name)
    }

    /// Check if a resolver is registered
    pub fn contains(&self, name: &str) -> bool {
        self.resolvers.contains_key(name)
    }

    /// Resolve an interpolation using the appropriate resolver
    pub fn resolve(
        &self,
        resolver_name: &str,
        args: &[String],
        kwargs: &HashMap<String, String>,
        ctx: &ResolverContext,
    ) -> Result<ResolvedValue> {
        let resolver = self
            .resolvers
            .get(resolver_name)
            .ok_or_else(|| Error::unknown_resolver(resolver_name, Some(ctx.config_path.clone())))?;

        resolver.resolve(args, kwargs, ctx)
    }
}

/// Built-in environment variable resolver
///
/// Usage:
///   ${env:VAR_NAME}                      - Get env var (error if not set)
///   ${env:VAR_NAME,default}              - Get env var with default
///   ${env:VAR_NAME,sensitive=true}       - Mark as sensitive for redaction
///   ${env:VAR_NAME,default,sensitive=true} - Both default and sensitive
fn env_resolver(
    args: &[String],
    kwargs: &HashMap<String, String>,
    ctx: &ResolverContext,
) -> Result<ResolvedValue> {
    if args.is_empty() {
        return Err(Error::parse("env resolver requires a variable name")
            .with_path(ctx.config_path.clone()));
    }

    let var_name = &args[0];
    let default_value = args.get(1);

    // Check if sensitive=true is set in kwargs
    let is_sensitive = kwargs
        .get("sensitive")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    match std::env::var(var_name) {
        Ok(value) => {
            let resolved_value = Value::String(value);
            if is_sensitive {
                Ok(ResolvedValue::sensitive(resolved_value))
            } else {
                Ok(ResolvedValue::new(resolved_value))
            }
        }
        Err(_) => {
            if let Some(default) = default_value {
                let resolved_value = Value::String(default.clone());
                if is_sensitive {
                    Ok(ResolvedValue::sensitive(resolved_value))
                } else {
                    Ok(ResolvedValue::new(resolved_value))
                }
            } else {
                Err(Error::env_not_found(
                    var_name,
                    Some(ctx.config_path.clone()),
                ))
            }
        }
    }
}

/// Built-in file resolver
fn file_resolver(
    args: &[String],
    kwargs: &HashMap<String, String>,
    ctx: &ResolverContext,
) -> Result<ResolvedValue> {
    use std::path::Path;

    if args.is_empty() {
        return Err(
            Error::parse("file resolver requires a file path").with_path(ctx.config_path.clone())
        );
    }

    let file_path_str = &args[0];
    let parse_mode = kwargs.get("parse").map(|s| s.as_str()).unwrap_or("auto");

    // Resolve relative paths based on context base path
    let file_path = if Path::new(file_path_str).is_relative() {
        if let Some(base) = &ctx.base_path {
            base.join(file_path_str)
        } else {
            std::path::PathBuf::from(file_path_str)
        }
    } else {
        std::path::PathBuf::from(file_path_str)
    };

    // Read the file
    let content = std::fs::read_to_string(&file_path)
        .map_err(|_| Error::file_not_found(file_path_str, Some(ctx.config_path.clone())))?;

    // Determine parse mode
    let actual_parse_mode = if parse_mode == "auto" {
        // Detect from extension
        match file_path.extension().and_then(|e| e.to_str()) {
            Some("yaml") | Some("yml") => "yaml",
            Some("json") => "json",
            _ => "text",
        }
    } else {
        parse_mode
    };

    // Parse content based on mode
    match actual_parse_mode {
        "yaml" => {
            let value: Value = serde_yaml::from_str(&content).map_err(|e| {
                Error::parse(format!("Failed to parse YAML: {}", e))
                    .with_path(ctx.config_path.clone())
            })?;
            Ok(ResolvedValue::new(value))
        }
        "json" => {
            let value: Value = serde_json::from_str(&content).map_err(|e| {
                Error::parse(format!("Failed to parse JSON: {}", e))
                    .with_path(ctx.config_path.clone())
            })?;
            Ok(ResolvedValue::new(value))
        }
        _ => {
            // Default to text mode (including explicit "text")
            Ok(ResolvedValue::new(Value::String(content)))
        }
    }
}

/// Built-in HTTP resolver
///
/// This resolver is registered but disabled by default for security.
/// To enable HTTP resolution, set allow_http=true in ConfigOptions.
///
/// When the `http` feature is not enabled, this always returns an error.
fn http_resolver(
    args: &[String],
    _kwargs: &HashMap<String, String>,
    ctx: &ResolverContext,
) -> Result<ResolvedValue> {
    if args.is_empty() {
        return Err(Error::parse("http resolver requires a URL").with_path(ctx.config_path.clone()));
    }

    let url = &args[0];

    // The http resolver is always disabled by default for security
    // Users must enable it explicitly via ConfigOptions.allow_http
    // This is just a placeholder that always returns an error
    // The actual HTTP fetching is done in the Config when allow_http is true

    Err(Error {
        kind: crate::error::ErrorKind::Resolver(crate::error::ResolverErrorKind::HttpDisabled),
        path: Some(ctx.config_path.clone()),
        source_location: None,
        help: Some(format!(
            "Enable HTTP resolver with Config.load(..., allow_http=True)\nURL: {}",
            url
        )),
        cause: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_resolver_with_value() {
        std::env::set_var("HOLOCONF_TEST_VAR", "test_value");

        let ctx = ResolverContext::new("test.path");
        let args = vec!["HOLOCONF_TEST_VAR".to_string()];
        let kwargs = HashMap::new();

        let result = env_resolver(&args, &kwargs, &ctx).unwrap();
        assert_eq!(result.value.as_str(), Some("test_value"));
        assert!(!result.sensitive);

        std::env::remove_var("HOLOCONF_TEST_VAR");
    }

    #[test]
    fn test_env_resolver_with_default() {
        // Make sure the var doesn't exist
        std::env::remove_var("HOLOCONF_NONEXISTENT_VAR");

        let ctx = ResolverContext::new("test.path");
        let args = vec![
            "HOLOCONF_NONEXISTENT_VAR".to_string(),
            "default_value".to_string(),
        ];
        let kwargs = HashMap::new();

        let result = env_resolver(&args, &kwargs, &ctx).unwrap();
        assert_eq!(result.value.as_str(), Some("default_value"));
    }

    #[test]
    fn test_env_resolver_missing_no_default() {
        std::env::remove_var("HOLOCONF_MISSING_VAR");

        let ctx = ResolverContext::new("test.path");
        let args = vec!["HOLOCONF_MISSING_VAR".to_string()];
        let kwargs = HashMap::new();

        let result = env_resolver(&args, &kwargs, &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_env_resolver_sensitive_kwarg() {
        std::env::set_var("HOLOCONF_SENSITIVE_VAR", "secret_value");

        let ctx = ResolverContext::new("test.path");
        let args = vec!["HOLOCONF_SENSITIVE_VAR".to_string()];
        let mut kwargs = HashMap::new();
        kwargs.insert("sensitive".to_string(), "true".to_string());

        let result = env_resolver(&args, &kwargs, &ctx).unwrap();
        assert_eq!(result.value.as_str(), Some("secret_value"));
        assert!(result.sensitive);

        std::env::remove_var("HOLOCONF_SENSITIVE_VAR");
    }

    #[test]
    fn test_env_resolver_sensitive_false() {
        std::env::set_var("HOLOCONF_NON_SENSITIVE", "public_value");

        let ctx = ResolverContext::new("test.path");
        let args = vec!["HOLOCONF_NON_SENSITIVE".to_string()];
        let mut kwargs = HashMap::new();
        kwargs.insert("sensitive".to_string(), "false".to_string());

        let result = env_resolver(&args, &kwargs, &ctx).unwrap();
        assert_eq!(result.value.as_str(), Some("public_value"));
        assert!(!result.sensitive);

        std::env::remove_var("HOLOCONF_NON_SENSITIVE");
    }

    #[test]
    fn test_env_resolver_sensitive_with_default() {
        std::env::remove_var("HOLOCONF_SENSITIVE_DEFAULT");

        let ctx = ResolverContext::new("test.path");
        let args = vec![
            "HOLOCONF_SENSITIVE_DEFAULT".to_string(),
            "default_secret".to_string(),
        ];
        let mut kwargs = HashMap::new();
        kwargs.insert("sensitive".to_string(), "true".to_string());

        let result = env_resolver(&args, &kwargs, &ctx).unwrap();
        assert_eq!(result.value.as_str(), Some("default_secret"));
        assert!(result.sensitive);
    }

    #[test]
    fn test_resolver_registry() {
        let registry = ResolverRegistry::with_builtins();

        assert!(registry.contains("env"));
        assert!(!registry.contains("nonexistent"));
    }

    #[test]
    fn test_custom_resolver() {
        let mut registry = ResolverRegistry::new();

        registry.register_fn("custom", |args, _kwargs, _ctx| {
            let value = args.first().cloned().unwrap_or_default();
            Ok(ResolvedValue::new(Value::String(format!(
                "custom:{}",
                value
            ))))
        });

        let ctx = ResolverContext::new("test");
        let result = registry
            .resolve("custom", &["arg".to_string()], &HashMap::new(), &ctx)
            .unwrap();

        assert_eq!(result.value.as_str(), Some("custom:arg"));
    }

    #[test]
    fn test_resolved_value_sensitivity() {
        let non_sensitive = ResolvedValue::new("public");
        assert!(!non_sensitive.sensitive);

        let sensitive = ResolvedValue::sensitive("secret");
        assert!(sensitive.sensitive);
    }

    #[test]
    fn test_resolver_context_cycle_detection() {
        let mut ctx = ResolverContext::new("root");
        ctx.push_resolution("a");
        ctx.push_resolution("b");

        assert!(ctx.would_cause_cycle("a"));
        assert!(ctx.would_cause_cycle("b"));
        assert!(!ctx.would_cause_cycle("c"));

        ctx.pop_resolution();
        assert!(!ctx.would_cause_cycle("b"));
    }

    #[test]
    fn test_file_resolver() {
        use std::io::Write;

        // Create a temporary file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("holoconf_test_file.txt");
        {
            let mut file = std::fs::File::create(&test_file).unwrap();
            writeln!(file, "test content").unwrap();
        }

        let mut ctx = ResolverContext::new("test.path");
        ctx.base_path = Some(temp_dir.clone());

        let args = vec!["holoconf_test_file.txt".to_string()];
        let mut kwargs = HashMap::new();
        kwargs.insert("parse".to_string(), "text".to_string());

        let result = file_resolver(&args, &kwargs, &ctx).unwrap();
        assert!(result.value.as_str().unwrap().contains("test content"));
        assert!(!result.sensitive);

        // Cleanup
        std::fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_file_resolver_yaml() {
        use std::io::Write;

        // Create a temporary YAML file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("holoconf_test.yaml");
        {
            let mut file = std::fs::File::create(&test_file).unwrap();
            writeln!(file, "key: value").unwrap();
            writeln!(file, "number: 42").unwrap();
        }

        let mut ctx = ResolverContext::new("test.path");
        ctx.base_path = Some(temp_dir.clone());

        let args = vec!["holoconf_test.yaml".to_string()];
        let kwargs = HashMap::new();

        let result = file_resolver(&args, &kwargs, &ctx).unwrap();
        assert!(result.value.is_mapping());

        // Cleanup
        std::fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_file_resolver_not_found() {
        let ctx = ResolverContext::new("test.path");
        let args = vec!["nonexistent_file.txt".to_string()];
        let kwargs = HashMap::new();

        let result = file_resolver(&args, &kwargs, &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_with_file() {
        let registry = ResolverRegistry::with_builtins();
        assert!(registry.contains("file"));
    }

    #[test]
    fn test_http_resolver_disabled() {
        let ctx = ResolverContext::new("test.path");
        let args = vec!["https://example.com/config.yaml".to_string()];
        let kwargs = HashMap::new();

        let result = http_resolver(&args, &kwargs, &ctx);
        assert!(result.is_err());

        let err = result.unwrap_err();
        let display = format!("{}", err);
        assert!(display.contains("HTTP resolver is disabled"));
    }

    #[test]
    fn test_registry_with_http() {
        let registry = ResolverRegistry::with_builtins();
        assert!(registry.contains("http"));
    }

    // Additional edge case tests for improved coverage

    #[test]
    fn test_env_resolver_no_args() {
        let ctx = ResolverContext::new("test.path");
        let args = vec![];
        let kwargs = HashMap::new();

        let result = env_resolver(&args, &kwargs, &ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("requires"));
    }

    #[test]
    fn test_file_resolver_no_args() {
        let ctx = ResolverContext::new("test.path");
        let args = vec![];
        let kwargs = HashMap::new();

        let result = file_resolver(&args, &kwargs, &ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("requires"));
    }

    #[test]
    fn test_http_resolver_no_args() {
        let ctx = ResolverContext::new("test.path");
        let args = vec![];
        let kwargs = HashMap::new();

        let result = http_resolver(&args, &kwargs, &ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("requires"));
    }

    #[test]
    fn test_unknown_resolver() {
        let registry = ResolverRegistry::with_builtins();
        let ctx = ResolverContext::new("test.path");

        let result = registry.resolve("unknown_resolver", &[], &HashMap::new(), &ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("unknown_resolver"));
    }

    #[test]
    fn test_resolved_value_from_traits() {
        let from_value: ResolvedValue = Value::String("test".to_string()).into();
        assert_eq!(from_value.value.as_str(), Some("test"));
        assert!(!from_value.sensitive);

        let from_string: ResolvedValue = "hello".to_string().into();
        assert_eq!(from_string.value.as_str(), Some("hello"));

        let from_str: ResolvedValue = "world".into();
        assert_eq!(from_str.value.as_str(), Some("world"));
    }

    #[test]
    fn test_resolver_context_with_base_path() {
        let ctx = ResolverContext::new("test").with_base_path(std::path::PathBuf::from("/tmp"));
        assert_eq!(ctx.base_path, Some(std::path::PathBuf::from("/tmp")));
    }

    #[test]
    fn test_resolver_context_with_config_root() {
        use std::sync::Arc;
        let root = Arc::new(Value::String("root".to_string()));
        let ctx = ResolverContext::new("test").with_config_root(root.clone());
        assert!(ctx.config_root.is_some());
    }

    #[test]
    fn test_resolver_context_resolution_chain() {
        let mut ctx = ResolverContext::new("root");
        ctx.push_resolution("a");
        ctx.push_resolution("b");
        ctx.push_resolution("c");

        let chain = ctx.get_resolution_chain();
        assert_eq!(chain, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_registry_get_resolver() {
        let registry = ResolverRegistry::with_builtins();

        let env_resolver = registry.get("env");
        assert!(env_resolver.is_some());
        assert_eq!(env_resolver.unwrap().name(), "env");

        let missing = registry.get("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_registry_default() {
        let registry = ResolverRegistry::default();
        // Default registry is empty
        assert!(!registry.contains("env"));
    }

    #[test]
    fn test_fn_resolver_name() {
        let resolver = FnResolver::new("my_resolver", |_, _, _| Ok(ResolvedValue::new("test")));
        assert_eq!(resolver.name(), "my_resolver");
    }

    #[test]
    fn test_file_resolver_json() {
        use std::io::Write;

        // Create a temporary JSON file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("holoconf_test.json");
        {
            let mut file = std::fs::File::create(&test_file).unwrap();
            writeln!(file, r#"{{"key": "value", "number": 42}}"#).unwrap();
        }

        let mut ctx = ResolverContext::new("test.path");
        ctx.base_path = Some(temp_dir.clone());

        let args = vec!["holoconf_test.json".to_string()];
        let kwargs = HashMap::new();

        let result = file_resolver(&args, &kwargs, &ctx).unwrap();
        assert!(result.value.is_mapping());

        // Cleanup
        std::fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_file_resolver_absolute_path() {
        use std::io::Write;

        // Create a temporary file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("holoconf_abs_test.txt");
        {
            let mut file = std::fs::File::create(&test_file).unwrap();
            writeln!(file, "absolute path content").unwrap();
        }

        let ctx = ResolverContext::new("test.path");
        // No base path - using absolute path directly
        let args = vec![test_file.to_string_lossy().to_string()];
        let mut kwargs = HashMap::new();
        kwargs.insert("parse".to_string(), "text".to_string());

        let result = file_resolver(&args, &kwargs, &ctx).unwrap();
        assert!(result
            .value
            .as_str()
            .unwrap()
            .contains("absolute path content"));

        // Cleanup
        std::fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_file_resolver_invalid_yaml() {
        use std::io::Write;

        // Create a temporary file with invalid YAML
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("holoconf_invalid.yaml");
        {
            let mut file = std::fs::File::create(&test_file).unwrap();
            writeln!(file, "key: [invalid").unwrap();
        }

        let mut ctx = ResolverContext::new("test.path");
        ctx.base_path = Some(temp_dir.clone());

        let args = vec!["holoconf_invalid.yaml".to_string()];
        let kwargs = HashMap::new();

        let result = file_resolver(&args, &kwargs, &ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("parse") || err.to_string().contains("YAML"));

        // Cleanup
        std::fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_file_resolver_invalid_json() {
        use std::io::Write;

        // Create a temporary file with invalid JSON
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("holoconf_invalid.json");
        {
            let mut file = std::fs::File::create(&test_file).unwrap();
            writeln!(file, "{{invalid json}}").unwrap();
        }

        let mut ctx = ResolverContext::new("test.path");
        ctx.base_path = Some(temp_dir.clone());

        let args = vec!["holoconf_invalid.json".to_string()];
        let kwargs = HashMap::new();

        let result = file_resolver(&args, &kwargs, &ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("parse") || err.to_string().contains("JSON"));

        // Cleanup
        std::fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_file_resolver_unknown_extension() {
        use std::io::Write;

        // Create a temporary file with unknown extension (treated as text)
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("holoconf_test.xyz");
        {
            let mut file = std::fs::File::create(&test_file).unwrap();
            writeln!(file, "plain text content").unwrap();
        }

        let mut ctx = ResolverContext::new("test.path");
        ctx.base_path = Some(temp_dir.clone());

        let args = vec!["holoconf_test.xyz".to_string()];
        let kwargs = HashMap::new();

        let result = file_resolver(&args, &kwargs, &ctx).unwrap();
        // Unknown extension defaults to text mode
        assert!(result
            .value
            .as_str()
            .unwrap()
            .contains("plain text content"));

        // Cleanup
        std::fs::remove_file(test_file).ok();
    }
}

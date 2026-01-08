//! Main Config type for holoconf
//!
//! The Config type is the primary interface for loading and accessing
//! configuration values with lazy resolution.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::error::{Error, Result};
use crate::interpolation::{self, Interpolation, InterpolationArg};
use crate::resolver::{ResolvedValue, ResolverContext, ResolverRegistry};
use crate::value::Value;

/// Configuration options for loading configs
#[derive(Debug, Clone, Default)]
pub struct ConfigOptions {
    /// Base path for relative file references
    pub base_path: Option<PathBuf>,
    /// Allow HTTP resolver (disabled by default for security)
    pub allow_http: bool,
    /// HTTP URL allowlist (glob patterns)
    pub http_allowlist: Vec<String>,
    /// Additional file roots for file resolver sandboxing
    pub file_roots: Vec<PathBuf>,
}

/// The main configuration container
///
/// Config provides lazy resolution of interpolation expressions
/// and caches resolved values for efficiency.
pub struct Config {
    /// The raw (unresolved) configuration data
    raw: Arc<Value>,
    /// Cache of resolved values
    cache: Arc<RwLock<HashMap<String, ResolvedValue>>>,
    /// Resolver registry
    resolvers: Arc<ResolverRegistry>,
    /// Configuration options
    options: ConfigOptions,
}

impl Config {
    /// Create a new Config from a Value
    pub fn new(value: Value) -> Self {
        Self {
            raw: Arc::new(value),
            cache: Arc::new(RwLock::new(HashMap::new())),
            resolvers: Arc::new(ResolverRegistry::with_builtins()),
            options: ConfigOptions::default(),
        }
    }

    /// Create a Config with custom options
    pub fn with_options(value: Value, options: ConfigOptions) -> Self {
        Self {
            raw: Arc::new(value),
            cache: Arc::new(RwLock::new(HashMap::new())),
            resolvers: Arc::new(ResolverRegistry::with_builtins()),
            options,
        }
    }

    /// Create a Config with a custom resolver registry
    pub fn with_resolvers(value: Value, resolvers: ResolverRegistry) -> Self {
        Self {
            raw: Arc::new(value),
            cache: Arc::new(RwLock::new(HashMap::new())),
            resolvers: Arc::new(resolvers),
            options: ConfigOptions::default(),
        }
    }

    /// Load configuration from a YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let value: Value = serde_yaml::from_str(yaml).map_err(|e| Error::parse(e.to_string()))?;
        Ok(Self::new(value))
    }

    /// Load configuration from a YAML string with options
    pub fn from_yaml_with_options(yaml: &str, options: ConfigOptions) -> Result<Self> {
        let value: Value = serde_yaml::from_str(yaml).map_err(|e| Error::parse(e.to_string()))?;
        Ok(Self::with_options(value, options))
    }

    /// Load configuration from a YAML file
    pub fn from_yaml_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| {
            Error::parse(format!("Failed to read file '{}': {}", path.display(), e))
        })?;

        let value: Value =
            serde_yaml::from_str(&content).map_err(|e| Error::parse(e.to_string()))?;

        let mut options = ConfigOptions::default();
        options.base_path = path.parent().map(|p| p.to_path_buf());

        Ok(Self::with_options(value, options))
    }

    /// Load and merge multiple YAML files
    ///
    /// Files are merged in order, with later files overriding earlier ones.
    /// Per ADR-004:
    /// - Mappings are deep-merged
    /// - Scalars use last-writer-wins
    /// - Arrays are replaced (not concatenated)
    /// - Null values remove keys
    pub fn load_merged<P: AsRef<Path>>(paths: &[P]) -> Result<Self> {
        if paths.is_empty() {
            return Ok(Self::new(Value::Mapping(indexmap::IndexMap::new())));
        }

        let mut merged_value: Option<Value> = None;
        let mut last_base_path: Option<PathBuf> = None;

        for path in paths {
            let path = path.as_ref();
            let content = std::fs::read_to_string(path)
                .map_err(|_e| Error::file_not_found(path.display().to_string(), None))?;

            let value: Value =
                serde_yaml::from_str(&content).map_err(|e| Error::parse(e.to_string()))?;

            last_base_path = path.parent().map(|p| p.to_path_buf());

            match &mut merged_value {
                Some(base) => base.merge(value),
                None => merged_value = Some(value),
            }
        }

        let mut options = ConfigOptions::default();
        options.base_path = last_base_path;

        Ok(Self::with_options(
            merged_value.unwrap_or(Value::Mapping(indexmap::IndexMap::new())),
            options,
        ))
    }

    /// Merge another config into this one
    ///
    /// The other config's values override this config's values per ADR-004 merge semantics.
    pub fn merge(&mut self, other: Config) {
        // Get a mutable reference to our raw value
        if let Some(raw) = Arc::get_mut(&mut self.raw) {
            raw.merge((*other.raw).clone());
        } else {
            // Need to clone and replace
            let mut new_raw = (*self.raw).clone();
            new_raw.merge((*other.raw).clone());
            self.raw = Arc::new(new_raw);
        }
        // Clear the cache since values may have changed
        self.clear_cache();
    }

    /// Load configuration from a JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        let value: Value = serde_json::from_str(json).map_err(|e| Error::parse(e.to_string()))?;
        Ok(Self::new(value))
    }

    /// Get the raw (unresolved) value at a path
    pub fn get_raw(&self, path: &str) -> Result<&Value> {
        self.raw.get_path(path)
    }

    /// Get a resolved value at a path
    ///
    /// This resolves any interpolation expressions in the value.
    /// Resolved values are cached for subsequent accesses.
    pub fn get(&self, path: &str) -> Result<Value> {
        // Check cache first
        {
            let cache = self.cache.read().unwrap();
            if let Some(cached) = cache.get(path) {
                return Ok(cached.value.clone());
            }
        }

        // Get raw value
        let raw_value = self.raw.get_path(path)?;

        // Resolve the value
        let resolved = self.resolve_value(raw_value, path)?;

        // Cache the result
        {
            let mut cache = self.cache.write().unwrap();
            cache.insert(path.to_string(), resolved.clone());
        }

        Ok(resolved.value)
    }

    /// Get a resolved string value, with type coercion if needed
    pub fn get_string(&self, path: &str) -> Result<String> {
        let value = self.get(path)?;
        match value {
            Value::String(s) => Ok(s),
            Value::Integer(i) => Ok(i.to_string()),
            Value::Float(f) => Ok(f.to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            Value::Null => Ok("null".to_string()),
            _ => Err(Error::type_coercion(path, "string", value.type_name())),
        }
    }

    /// Get a resolved integer value, with type coercion if needed
    pub fn get_i64(&self, path: &str) -> Result<i64> {
        let value = self.get(path)?;
        match value {
            Value::Integer(i) => Ok(i),
            Value::String(s) => s
                .parse()
                .map_err(|_| Error::type_coercion(path, "integer", format!("string (\"{}\")", s))),
            _ => Err(Error::type_coercion(path, "integer", value.type_name())),
        }
    }

    /// Get a resolved float value, with type coercion if needed
    pub fn get_f64(&self, path: &str) -> Result<f64> {
        let value = self.get(path)?;
        match value {
            Value::Float(f) => Ok(f),
            Value::Integer(i) => Ok(i as f64),
            Value::String(s) => s
                .parse()
                .map_err(|_| Error::type_coercion(path, "float", format!("string (\"{}\")", s))),
            _ => Err(Error::type_coercion(path, "float", value.type_name())),
        }
    }

    /// Get a resolved boolean value, with strict coercion per ADR-012
    pub fn get_bool(&self, path: &str) -> Result<bool> {
        let value = self.get(path)?;
        match value {
            Value::Bool(b) => Ok(b),
            Value::String(s) => {
                // Strict boolean coercion: only "true" and "false"
                match s.to_lowercase().as_str() {
                    "true" => Ok(true),
                    "false" => Ok(false),
                    _ => Err(Error::type_coercion(
                        path,
                        "boolean",
                        format!("string (\"{}\") - only \"true\" or \"false\" allowed", s),
                    )),
                }
            }
            _ => Err(Error::type_coercion(path, "boolean", value.type_name())),
        }
    }

    /// Resolve all values in the configuration eagerly
    pub fn resolve_all(&self) -> Result<()> {
        self.resolve_value_recursive(&self.raw, "")?;
        Ok(())
    }

    /// Export the configuration as a resolved Value
    pub fn to_value(&self) -> Result<Value> {
        self.resolve_value_to_value(&self.raw, "")
    }

    /// Export the raw (unresolved) configuration as a Value
    ///
    /// This shows the configuration with interpolation placeholders (${...})
    pub fn to_value_raw(&self) -> Value {
        (*self.raw).clone()
    }

    /// Export the resolved configuration with optional redaction
    ///
    /// When redact=true, sensitive values are replaced with "[REDACTED]"
    pub fn to_value_redacted(&self, redact: bool) -> Result<Value> {
        if redact {
            self.resolve_value_to_value_redacted(&self.raw, "")
        } else {
            self.resolve_value_to_value(&self.raw, "")
        }
    }

    /// Export the configuration as YAML
    ///
    /// By default, resolves all values. Use to_yaml_raw() for unresolved output.
    pub fn to_yaml(&self) -> Result<String> {
        let value = self.to_value()?;
        serde_yaml::to_string(&value).map_err(|e| Error::parse(e.to_string()))
    }

    /// Export the raw (unresolved) configuration as YAML
    ///
    /// Shows interpolation placeholders (${...}) without resolution.
    pub fn to_yaml_raw(&self) -> Result<String> {
        serde_yaml::to_string(&*self.raw).map_err(|e| Error::parse(e.to_string()))
    }

    /// Export the resolved configuration as YAML with optional redaction
    ///
    /// When redact=true, sensitive values are replaced with "[REDACTED]"
    pub fn to_yaml_redacted(&self, redact: bool) -> Result<String> {
        let value = self.to_value_redacted(redact)?;
        serde_yaml::to_string(&value).map_err(|e| Error::parse(e.to_string()))
    }

    /// Export the configuration as JSON
    ///
    /// By default, resolves all values. Use to_json_raw() for unresolved output.
    pub fn to_json(&self) -> Result<String> {
        let value = self.to_value()?;
        serde_json::to_string_pretty(&value).map_err(|e| Error::parse(e.to_string()))
    }

    /// Export the raw (unresolved) configuration as JSON
    ///
    /// Shows interpolation placeholders (${...}) without resolution.
    pub fn to_json_raw(&self) -> Result<String> {
        serde_json::to_string_pretty(&*self.raw).map_err(|e| Error::parse(e.to_string()))
    }

    /// Export the resolved configuration as JSON with optional redaction
    ///
    /// When redact=true, sensitive values are replaced with "[REDACTED]"
    pub fn to_json_redacted(&self, redact: bool) -> Result<String> {
        let value = self.to_value_redacted(redact)?;
        serde_json::to_string_pretty(&value).map_err(|e| Error::parse(e.to_string()))
    }

    /// Clear the resolution cache
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }

    /// Register a custom resolver
    pub fn register_resolver(&mut self, resolver: Arc<dyn crate::resolver::Resolver>) {
        // We need to get mutable access to the registry
        // This is safe because we're the only owner at this point
        if let Some(registry) = Arc::get_mut(&mut self.resolvers) {
            registry.register(resolver);
        }
    }

    /// Validate the raw (unresolved) configuration against a schema
    ///
    /// This performs structural validation (Phase 1 per ADR-007):
    /// - Required keys are present
    /// - Object/array structure matches
    /// - Interpolations (${...}) are allowed as placeholders
    pub fn validate_raw(&self, schema: &crate::schema::Schema) -> Result<()> {
        schema.validate(&self.raw)
    }

    /// Validate the resolved configuration against a schema
    ///
    /// This performs type/value validation (Phase 2 per ADR-007):
    /// - Resolved values match expected types
    /// - Constraints (min, max, pattern, enum) are checked
    pub fn validate(&self, schema: &crate::schema::Schema) -> Result<()> {
        let resolved = self.to_value()?;
        schema.validate(&resolved)
    }

    /// Validate and collect all errors (instead of failing on first)
    pub fn validate_collect(
        &self,
        schema: &crate::schema::Schema,
    ) -> Vec<crate::schema::ValidationError> {
        match self.to_value() {
            Ok(resolved) => schema.validate_collect(&resolved),
            Err(e) => vec![crate::schema::ValidationError {
                path: String::new(),
                message: e.to_string(),
            }],
        }
    }

    /// Resolve a single value
    fn resolve_value(&self, value: &Value, path: &str) -> Result<ResolvedValue> {
        match value {
            Value::String(s) => {
                // Use needs_processing to handle both interpolations AND escape sequences
                if interpolation::needs_processing(s) {
                    let parsed = interpolation::parse(s)?;
                    self.resolve_interpolation(&parsed, path)
                } else {
                    Ok(ResolvedValue::new(value.clone()))
                }
            }
            _ => Ok(ResolvedValue::new(value.clone())),
        }
    }

    /// Resolve an interpolation expression
    fn resolve_interpolation(&self, interp: &Interpolation, path: &str) -> Result<ResolvedValue> {
        match interp {
            Interpolation::Literal(s) => Ok(ResolvedValue::new(Value::String(s.clone()))),

            Interpolation::Resolver { name, args, kwargs } => {
                // Create resolver context
                let mut ctx = ResolverContext::new(path);
                ctx.config_root = Some(Arc::clone(&self.raw));
                if let Some(base) = &self.options.base_path {
                    ctx.base_path = Some(base.clone());
                }

                // Resolve arguments
                let resolved_args: Vec<String> = args
                    .iter()
                    .map(|arg| self.resolve_arg(arg, path))
                    .collect::<Result<Vec<_>>>()?;

                let resolved_kwargs: HashMap<String, String> = kwargs
                    .iter()
                    .map(|(k, v)| Ok((k.clone(), self.resolve_arg(v, path)?)))
                    .collect::<Result<HashMap<_, _>>>()?;

                // Call the resolver
                self.resolvers
                    .resolve(name, &resolved_args, &resolved_kwargs, &ctx)
            }

            Interpolation::SelfRef {
                path: ref_path,
                relative,
            } => {
                let full_path = if *relative {
                    self.resolve_relative_path(path, ref_path)
                } else {
                    ref_path.clone()
                };

                // Check for circular reference
                // For now, simple implementation - full cycle detection would need context tracking
                if full_path == path {
                    return Err(Error::circular_reference(
                        path,
                        vec![path.to_string(), full_path],
                    ));
                }

                // Get the referenced value
                let ref_value = self
                    .raw
                    .get_path(&full_path)
                    .map_err(|_| Error::ref_not_found(&full_path, Some(path.to_string())))?;

                // Resolve it recursively
                self.resolve_value(ref_value, &full_path)
            }

            Interpolation::Concat(parts) => {
                let mut result = String::new();
                let mut any_sensitive = false;

                for part in parts {
                    let resolved = self.resolve_interpolation(part, path)?;
                    any_sensitive = any_sensitive || resolved.sensitive;

                    match resolved.value {
                        Value::String(s) => result.push_str(&s),
                        other => result.push_str(&other.to_string()),
                    }
                }

                if any_sensitive {
                    Ok(ResolvedValue::sensitive(Value::String(result)))
                } else {
                    Ok(ResolvedValue::new(Value::String(result)))
                }
            }
        }
    }

    /// Resolve an interpolation argument
    fn resolve_arg(&self, arg: &InterpolationArg, path: &str) -> Result<String> {
        match arg {
            InterpolationArg::Literal(s) => Ok(s.clone()),
            InterpolationArg::Nested(interp) => {
                let resolved = self.resolve_interpolation(interp, path)?;
                match resolved.value {
                    Value::String(s) => Ok(s),
                    other => Ok(other.to_string()),
                }
            }
        }
    }

    /// Resolve a relative path reference
    fn resolve_relative_path(&self, current_path: &str, ref_path: &str) -> String {
        let mut ref_chars = ref_path.chars().peekable();
        let mut levels_up = 0;

        // Count leading dots for parent references
        while ref_chars.peek() == Some(&'.') {
            ref_chars.next();
            levels_up += 1;
        }

        // Get the remaining path
        let remaining: String = ref_chars.collect();

        if levels_up == 0 {
            // No dots - shouldn't happen for relative paths
            return ref_path.to_string();
        }

        // Split current path into segments
        let mut segments: Vec<&str> = current_path.split('.').collect();

        // Remove segments based on levels up
        // levels_up = 1 means sibling (remove last segment)
        // levels_up = 2 means parent's sibling (remove last 2 segments)
        for _ in 0..levels_up {
            segments.pop();
        }

        // Append the remaining path
        if remaining.is_empty() {
            segments.join(".")
        } else if segments.is_empty() {
            remaining
        } else {
            format!("{}.{}", segments.join("."), remaining)
        }
    }

    /// Recursively resolve all values
    fn resolve_value_recursive(&self, value: &Value, path: &str) -> Result<ResolvedValue> {
        match value {
            Value::String(s) => {
                if interpolation::needs_processing(s) {
                    let parsed = interpolation::parse(s)?;
                    let resolved = self.resolve_interpolation(&parsed, path)?;

                    // Cache the result
                    let mut cache = self.cache.write().unwrap();
                    cache.insert(path.to_string(), resolved.clone());

                    Ok(resolved)
                } else {
                    Ok(ResolvedValue::new(value.clone()))
                }
            }
            Value::Sequence(seq) => {
                for (i, item) in seq.iter().enumerate() {
                    let item_path = format!("{}[{}]", path, i);
                    self.resolve_value_recursive(item, &item_path)?;
                }
                Ok(ResolvedValue::new(value.clone()))
            }
            Value::Mapping(map) => {
                for (key, val) in map {
                    let key_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };
                    self.resolve_value_recursive(val, &key_path)?;
                }
                Ok(ResolvedValue::new(value.clone()))
            }
            _ => Ok(ResolvedValue::new(value.clone())),
        }
    }

    /// Resolve a value tree to a new Value
    fn resolve_value_to_value(&self, value: &Value, path: &str) -> Result<Value> {
        match value {
            Value::String(s) => {
                if interpolation::needs_processing(s) {
                    let parsed = interpolation::parse(s)?;
                    let resolved = self.resolve_interpolation(&parsed, path)?;
                    Ok(resolved.value)
                } else {
                    Ok(value.clone())
                }
            }
            Value::Sequence(seq) => {
                let resolved: Result<Vec<Value>> = seq
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let item_path = format!("{}[{}]", path, i);
                        self.resolve_value_to_value(item, &item_path)
                    })
                    .collect();
                Ok(Value::Sequence(resolved?))
            }
            Value::Mapping(map) => {
                let mut resolved = indexmap::IndexMap::new();
                for (key, val) in map {
                    let key_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };
                    resolved.insert(key.clone(), self.resolve_value_to_value(val, &key_path)?);
                }
                Ok(Value::Mapping(resolved))
            }
            _ => Ok(value.clone()),
        }
    }

    /// Resolve a value tree to a new Value with sensitive value redaction
    fn resolve_value_to_value_redacted(&self, value: &Value, path: &str) -> Result<Value> {
        const REDACTED: &str = "[REDACTED]";

        match value {
            Value::String(s) => {
                if interpolation::needs_processing(s) {
                    let parsed = interpolation::parse(s)?;
                    let resolved = self.resolve_interpolation(&parsed, path)?;
                    if resolved.sensitive {
                        Ok(Value::String(REDACTED.to_string()))
                    } else {
                        Ok(resolved.value)
                    }
                } else {
                    Ok(value.clone())
                }
            }
            Value::Sequence(seq) => {
                let resolved: Result<Vec<Value>> = seq
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let item_path = format!("{}[{}]", path, i);
                        self.resolve_value_to_value_redacted(item, &item_path)
                    })
                    .collect();
                Ok(Value::Sequence(resolved?))
            }
            Value::Mapping(map) => {
                let mut resolved = indexmap::IndexMap::new();
                for (key, val) in map {
                    let key_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };
                    resolved.insert(
                        key.clone(),
                        self.resolve_value_to_value_redacted(val, &key_path)?,
                    );
                }
                Ok(Value::Mapping(resolved))
            }
            _ => Ok(value.clone()),
        }
    }
}

impl Clone for Config {
    fn clone(&self) -> Self {
        Self {
            raw: Arc::clone(&self.raw),
            cache: Arc::new(RwLock::new(HashMap::new())), // Fresh cache per ADR-010
            resolvers: Arc::clone(&self.resolvers),
            options: self.options.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_yaml() {
        let yaml = r#"
database:
  host: localhost
  port: 5432
"#;
        let config = Config::from_yaml(yaml).unwrap();

        assert_eq!(
            config.get("database.host").unwrap().as_str(),
            Some("localhost")
        );
        assert_eq!(config.get("database.port").unwrap().as_i64(), Some(5432));
    }

    #[test]
    fn test_env_resolver() {
        std::env::set_var("HOLOCONF_TEST_HOST", "prod-server");

        let yaml = r#"
server:
  host: ${env:HOLOCONF_TEST_HOST}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        assert_eq!(
            config.get("server.host").unwrap().as_str(),
            Some("prod-server")
        );

        std::env::remove_var("HOLOCONF_TEST_HOST");
    }

    #[test]
    fn test_env_resolver_with_default() {
        std::env::remove_var("HOLOCONF_MISSING_VAR");

        let yaml = r#"
server:
  host: ${env:HOLOCONF_MISSING_VAR,default-host}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        assert_eq!(
            config.get("server.host").unwrap().as_str(),
            Some("default-host")
        );
    }

    #[test]
    fn test_self_reference() {
        let yaml = r#"
defaults:
  host: localhost
database:
  host: ${defaults.host}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        assert_eq!(
            config.get("database.host").unwrap().as_str(),
            Some("localhost")
        );
    }

    #[test]
    fn test_string_concatenation() {
        std::env::set_var("HOLOCONF_PREFIX", "prod");

        let yaml = r#"
bucket: myapp-${env:HOLOCONF_PREFIX}-data
"#;
        let config = Config::from_yaml(yaml).unwrap();

        assert_eq!(
            config.get("bucket").unwrap().as_str(),
            Some("myapp-prod-data")
        );

        std::env::remove_var("HOLOCONF_PREFIX");
    }

    #[test]
    fn test_escaped_interpolation() {
        // In YAML, we need to quote the value to preserve the backslash properly
        // Or the backslash escapes the $
        let yaml = r#"
literal: '\${not_resolved}'
"#;
        let config = Config::from_yaml(yaml).unwrap();

        // After parsing, the backslash-$ sequence becomes just ${
        assert_eq!(
            config.get("literal").unwrap().as_str(),
            Some("${not_resolved}")
        );
    }

    #[test]
    fn test_type_coercion_string_to_int() {
        std::env::set_var("HOLOCONF_PORT", "8080");

        let yaml = r#"
port: ${env:HOLOCONF_PORT}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        // get_i64 should coerce the string to integer
        assert_eq!(config.get_i64("port").unwrap(), 8080);

        std::env::remove_var("HOLOCONF_PORT");
    }

    #[test]
    fn test_strict_boolean_coercion() {
        std::env::set_var("HOLOCONF_ENABLED", "true");
        std::env::set_var("HOLOCONF_INVALID", "1");

        let yaml = r#"
enabled: ${env:HOLOCONF_ENABLED}
invalid: ${env:HOLOCONF_INVALID}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        // "true" should work
        assert!(config.get_bool("enabled").unwrap());

        // "1" should NOT work per ADR-012
        assert!(config.get_bool("invalid").is_err());

        std::env::remove_var("HOLOCONF_ENABLED");
        std::env::remove_var("HOLOCONF_INVALID");
    }

    #[test]
    fn test_caching() {
        std::env::set_var("HOLOCONF_CACHED", "initial");

        let yaml = r#"
value: ${env:HOLOCONF_CACHED}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        // First access resolves and caches
        assert_eq!(config.get("value").unwrap().as_str(), Some("initial"));

        // Change the env var
        std::env::set_var("HOLOCONF_CACHED", "changed");

        // Second access returns cached value
        assert_eq!(config.get("value").unwrap().as_str(), Some("initial"));

        // Clear cache
        config.clear_cache();

        // Now returns new value
        assert_eq!(config.get("value").unwrap().as_str(), Some("changed"));

        std::env::remove_var("HOLOCONF_CACHED");
    }

    #[test]
    fn test_path_not_found() {
        let yaml = r#"
database:
  host: localhost
"#;
        let config = Config::from_yaml(yaml).unwrap();

        let result = config.get("database.nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_to_yaml() {
        std::env::set_var("HOLOCONF_EXPORT_HOST", "exported-host");

        let yaml = r#"
server:
  host: ${env:HOLOCONF_EXPORT_HOST}
  port: 8080
"#;
        let config = Config::from_yaml(yaml).unwrap();

        let exported = config.to_yaml().unwrap();
        assert!(exported.contains("exported-host"));
        assert!(exported.contains("8080"));

        std::env::remove_var("HOLOCONF_EXPORT_HOST");
    }

    #[test]
    fn test_relative_path_sibling() {
        let yaml = r#"
database:
  host: localhost
  url: postgres://${.host}:5432/db
"#;
        let config = Config::from_yaml(yaml).unwrap();

        assert_eq!(
            config.get("database.url").unwrap().as_str(),
            Some("postgres://localhost:5432/db")
        );
    }

    #[test]
    fn test_array_access() {
        let yaml = r#"
servers:
  - host: server1
  - host: server2
primary: ${servers[0].host}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        assert_eq!(config.get("primary").unwrap().as_str(), Some("server1"));
    }

    #[test]
    fn test_nested_interpolation() {
        std::env::set_var("HOLOCONF_DEFAULT_HOST", "fallback-host");

        let yaml = r#"
host: ${env:UNDEFINED_HOST,${env:HOLOCONF_DEFAULT_HOST}}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        assert_eq!(config.get("host").unwrap().as_str(), Some("fallback-host"));

        std::env::remove_var("HOLOCONF_DEFAULT_HOST");
    }

    #[test]
    fn test_to_yaml_raw() {
        let yaml = r#"
server:
  host: ${env:MY_HOST}
  port: 8080
"#;
        let config = Config::from_yaml(yaml).unwrap();

        let raw = config.to_yaml_raw().unwrap();
        // Should contain the placeholder, not a resolved value
        assert!(raw.contains("${env:MY_HOST}"));
        assert!(raw.contains("8080"));
    }

    #[test]
    fn test_to_json_raw() {
        let yaml = r#"
database:
  url: ${env:DATABASE_URL}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        let raw = config.to_json_raw().unwrap();
        // Should contain the placeholder
        assert!(raw.contains("${env:DATABASE_URL}"));
    }

    #[test]
    fn test_to_value_raw() {
        let yaml = r#"
key: ${env:SOME_VAR}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        let raw = config.to_value_raw();
        assert_eq!(
            raw.get_path("key").unwrap().as_str(),
            Some("${env:SOME_VAR}")
        );
    }

    #[test]
    fn test_to_yaml_redacted_no_sensitive() {
        std::env::set_var("HOLOCONF_NON_SENSITIVE", "public-value");

        let yaml = r#"
value: ${env:HOLOCONF_NON_SENSITIVE}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        // With redact=true, but no sensitive values, should show real values
        let output = config.to_yaml_redacted(true).unwrap();
        assert!(output.contains("public-value"));

        std::env::remove_var("HOLOCONF_NON_SENSITIVE");
    }
}

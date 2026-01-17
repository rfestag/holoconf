//! Main Config type for holoconf
//!
//! The Config type is the primary interface for loading and accessing
//! configuration values with lazy resolution.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::error::{Error, Result};
use crate::interpolation::{self, Interpolation, InterpolationArg};
use crate::resolver::{global_registry, ResolvedValue, ResolverContext, ResolverRegistry};
use crate::value::Value;

/// Check if a path string contains glob metacharacters
fn is_glob_pattern(path: &str) -> bool {
    path.contains('*') || path.contains('?') || path.contains('[')
}

/// Expand a glob pattern to matching paths, sorted alphabetically
fn expand_glob(pattern: &str) -> Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = glob::glob(pattern)
        .map_err(|e| Error::parse(format!("Invalid glob pattern '{}': {}", pattern, e)))?
        .filter_map(|r| r.ok())
        .collect();
    paths.sort();
    Ok(paths)
}

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

    // --- TLS/Proxy Options ---
    /// HTTP proxy URL (e.g., "http://proxy:8080" or "socks5://proxy:1080")
    pub http_proxy: Option<String>,
    /// Whether to auto-detect proxy from environment variables (HTTP_PROXY, HTTPS_PROXY)
    pub http_proxy_from_env: bool,
    /// Path to CA bundle PEM file (replaces default webpki-roots)
    pub http_ca_bundle: Option<PathBuf>,
    /// Path to extra CA bundle PEM file (appends to webpki-roots)
    pub http_extra_ca_bundle: Option<PathBuf>,
    /// Path to client certificate PEM or P12/PFX file (for mTLS)
    pub http_client_cert: Option<PathBuf>,
    /// Path to client private key PEM file (for mTLS, not needed for P12/PFX)
    pub http_client_key: Option<PathBuf>,
    /// Password for encrypted private key or P12/PFX file
    pub http_client_key_password: Option<String>,
    /// DANGEROUS: Skip TLS certificate verification (dev only)
    pub http_insecure: bool,
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
    /// Source file for each config path (tracks which file a value came from)
    source_map: Arc<HashMap<String, String>>,
    /// Resolver registry
    resolvers: Arc<ResolverRegistry>,
    /// Configuration options
    options: ConfigOptions,
    /// Optional schema for default value lookup
    schema: Option<Arc<crate::schema::Schema>>,
}

/// Clone the global registry for use in a Config instance
fn clone_global_registry() -> Arc<ResolverRegistry> {
    let global = global_registry()
        .read()
        .expect("Global registry lock poisoned");
    Arc::new(global.clone())
}

impl Config {
    /// Create a new Config from a Value
    ///
    /// The config will use resolvers from the global registry.
    pub fn new(value: Value) -> Self {
        Self {
            raw: Arc::new(value),
            cache: Arc::new(RwLock::new(HashMap::new())),
            source_map: Arc::new(HashMap::new()),
            resolvers: clone_global_registry(),
            options: ConfigOptions::default(),
            schema: None,
        }
    }

    /// Create a Config with custom options
    ///
    /// The config will use resolvers from the global registry.
    pub fn with_options(value: Value, options: ConfigOptions) -> Self {
        Self {
            raw: Arc::new(value),
            cache: Arc::new(RwLock::new(HashMap::new())),
            source_map: Arc::new(HashMap::new()),
            resolvers: clone_global_registry(),
            options,
            schema: None,
        }
    }

    /// Create a Config with options and source map
    fn with_options_and_sources(
        value: Value,
        options: ConfigOptions,
        source_map: HashMap<String, String>,
    ) -> Self {
        Self {
            raw: Arc::new(value),
            cache: Arc::new(RwLock::new(HashMap::new())),
            source_map: Arc::new(source_map),
            resolvers: clone_global_registry(),
            options,
            schema: None,
        }
    }

    /// Create a Config with a custom resolver registry
    pub fn with_resolvers(value: Value, resolvers: ResolverRegistry) -> Self {
        Self {
            raw: Arc::new(value),
            cache: Arc::new(RwLock::new(HashMap::new())),
            source_map: Arc::new(HashMap::new()),
            resolvers: Arc::new(resolvers),
            options: ConfigOptions::default(),
            schema: None,
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

    /// Load configuration from a YAML file (required - errors if missing)
    ///
    /// This is the primary way to load configuration. Use `Config::optional()`
    /// for files that may not exist.
    ///
    /// Supports glob patterns like `config/*.yaml` or `config/**/*.yaml`.
    /// When a glob pattern is used, matching files are sorted alphabetically
    /// and merged in order (later files override earlier ones).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = Config::load("config.yaml")?;
    /// let merged = Config::load("config/*.yaml")?;
    /// ```
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy();

        if is_glob_pattern(&path_str) {
            let paths = expand_glob(&path_str)?;
            if paths.is_empty() {
                return Err(Error::file_not_found(
                    format!("No files matched glob pattern '{}'", path_str),
                    None,
                ));
            }
            // Load first file, then merge the rest
            let mut config = Self::from_yaml_file(&paths[0])?;
            for p in &paths[1..] {
                let other = Self::from_yaml_file(p)?;
                config.merge(other);
            }
            Ok(config)
        } else {
            Self::from_yaml_file(path)
        }
    }

    /// Load a configuration file with custom options
    ///
    /// This is the main entry point for loading config with HTTP/TLS/proxy options.
    /// Supports glob patterns like `config/*.yaml` or `config/**/*.yaml`.
    /// When a glob pattern is used, matching files are sorted alphabetically
    /// and merged in order.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut options = ConfigOptions::default();
    /// options.allow_http = true;
    /// options.http_proxy = Some("http://proxy:8080".into());
    /// let config = Config::load_with_options("config.yaml", options)?;
    /// ```
    pub fn load_with_options(path: impl AsRef<Path>, options: ConfigOptions) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy();

        if is_glob_pattern(&path_str) {
            let paths = expand_glob(&path_str)?;
            if paths.is_empty() {
                return Err(Error::file_not_found(
                    format!("No files matched glob pattern '{}'", path_str),
                    None,
                ));
            }
            // Load first file with options, then merge the rest
            let mut config = Self::from_yaml_file_with_options(&paths[0], options.clone())?;
            for p in &paths[1..] {
                let other = Self::from_yaml_file_with_options(p, options.clone())?;
                config.merge(other);
            }
            Ok(config)
        } else {
            Self::from_yaml_file_with_options(path, options)
        }
    }

    /// Alias for `load()` - load a required config file
    ///
    /// Provided for symmetry with `Config::optional()`.
    pub fn required(path: impl AsRef<Path>) -> Result<Self> {
        Self::load(path)
    }

    /// Load a required config file with custom options
    pub fn required_with_options(path: impl AsRef<Path>, options: ConfigOptions) -> Result<Self> {
        Self::load_with_options(path, options)
    }

    /// Load an optional configuration file
    ///
    /// Returns an empty Config if the file doesn't exist.
    /// Use this for configuration files that may or may not be present,
    /// such as local overrides.
    ///
    /// Supports glob patterns like `config/*.yaml` or `config/**/*.yaml`.
    /// When a glob pattern is used, matching files are sorted alphabetically
    /// and merged in order. Returns empty config if no files match.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let base = Config::load("base.yaml")?;
    /// let local = Config::optional("local.yaml")?;
    /// let overrides = Config::optional("config/*.yaml")?;
    /// base.merge(&local);
    /// ```
    pub fn optional(path: impl AsRef<Path>) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy();

        if is_glob_pattern(&path_str) {
            let paths = expand_glob(&path_str)?;
            if paths.is_empty() {
                // No files matched - return empty config (this is optional)
                return Ok(Self::new(Value::Mapping(indexmap::IndexMap::new())));
            }
            // Load first file, then merge the rest
            let mut config = Self::from_yaml_file(&paths[0])?;
            for p in &paths[1..] {
                let other = Self::from_yaml_file(p)?;
                config.merge(other);
            }
            Ok(config)
        } else {
            Self::optional_single_file(path)
        }
    }

    /// Load a single optional file (non-glob)
    fn optional_single_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let value: Value =
                    serde_yaml::from_str(&content).map_err(|e| Error::parse(e.to_string()))?;

                let filename = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_string();
                let mut source_map = HashMap::new();
                value.collect_leaf_paths("", &filename, &mut source_map);

                let mut options = ConfigOptions::default();
                options.base_path = path.parent().map(|p| p.to_path_buf());

                Ok(Self::with_options_and_sources(value, options, source_map))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // File doesn't exist - return empty config
                Ok(Self::new(Value::Mapping(indexmap::IndexMap::new())))
            }
            Err(e) => Err(Error::parse(format!(
                "Failed to read file '{}': {}",
                path.display(),
                e
            ))),
        }
    }

    /// Load configuration from a YAML file
    fn from_yaml_file(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_yaml_file_with_options(path, ConfigOptions::default())
    }

    /// Load configuration from a YAML file with custom options
    fn from_yaml_file_with_options(
        path: impl AsRef<Path>,
        mut options: ConfigOptions,
    ) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::file_not_found(path.display().to_string(), None)
            } else {
                Error::parse(format!("Failed to read file '{}': {}", path.display(), e))
            }
        })?;

        let value: Value =
            serde_yaml::from_str(&content).map_err(|e| Error::parse(e.to_string()))?;

        // Track source for all leaf paths
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        let mut source_map = HashMap::new();
        value.collect_leaf_paths("", &filename, &mut source_map);

        // Set base_path from file location if not already set
        if options.base_path.is_none() {
            options.base_path = path.parent().map(|p| p.to_path_buf());
        }

        Ok(Self::with_options_and_sources(value, options, source_map))
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

    /// Set or replace the schema for default value lookup
    ///
    /// When a schema is attached, `get()` will return schema defaults for
    /// missing paths instead of raising `PathNotFoundError`.
    ///
    /// Note: Setting a schema clears the value cache since defaults may
    /// now affect lookups.
    pub fn set_schema(&mut self, schema: crate::schema::Schema) {
        self.schema = Some(Arc::new(schema));
        self.clear_cache();
    }

    /// Get a reference to the attached schema, if any
    pub fn get_schema(&self) -> Option<&crate::schema::Schema> {
        self.schema.as_ref().map(|s| s.as_ref())
    }

    /// Get the raw (unresolved) value at a path
    pub fn get_raw(&self, path: &str) -> Result<&Value> {
        self.raw.get_path(path)
    }

    /// Get a resolved value at a path
    ///
    /// This resolves any interpolation expressions in the value.
    /// Resolved values are cached for subsequent accesses.
    ///
    /// If a schema is attached and the path is not found (or is null when null
    /// is not allowed), the schema default is returned instead.
    pub fn get(&self, path: &str) -> Result<Value> {
        // Check cache first
        {
            let cache = self.cache.read().unwrap();
            if let Some(cached) = cache.get(path) {
                return Ok(cached.value.clone());
            }
        }

        // Try to get raw value
        let raw_result = self.raw.get_path(path);

        match raw_result {
            Ok(raw_value) => {
                // Resolve the value with an empty resolution stack
                let mut resolution_stack = Vec::new();
                let resolved = self.resolve_value(raw_value, path, &mut resolution_stack)?;

                // Check for null value that should use schema default
                if resolved.value.is_null() {
                    if let Some(schema) = &self.schema {
                        // If schema doesn't allow null but has a default, use the default
                        if !schema.allows_null(path) {
                            if let Some(default_value) = schema.get_default(path) {
                                let resolved_default = ResolvedValue::new(default_value.clone());
                                // Cache the default
                                {
                                    let mut cache = self.cache.write().unwrap();
                                    cache.insert(path.to_string(), resolved_default);
                                }
                                return Ok(default_value);
                            }
                        }
                    }
                }

                // Cache the result
                {
                    let mut cache = self.cache.write().unwrap();
                    cache.insert(path.to_string(), resolved.clone());
                }

                Ok(resolved.value)
            }
            Err(e) if matches!(e.kind, crate::error::ErrorKind::PathNotFound) => {
                // Path not found - check schema defaults
                if let Some(schema) = &self.schema {
                    if let Some(default_value) = schema.get_default(path) {
                        let resolved_default = ResolvedValue::new(default_value.clone());
                        // Cache the default
                        {
                            let mut cache = self.cache.write().unwrap();
                            cache.insert(path.to_string(), resolved_default);
                        }
                        return Ok(default_value);
                    }
                }
                // No schema or no default - propagate error
                Err(e)
            }
            Err(e) => Err(e),
        }
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
        let mut resolution_stack = Vec::new();
        self.resolve_value_recursive(&self.raw, "", &mut resolution_stack)?;
        Ok(())
    }

    /// Export the configuration as a Value
    ///
    /// # Arguments
    /// * `resolve` - If true, resolve interpolations (${...}). If false, show placeholders.
    /// * `redact` - If true, replace sensitive values with "[REDACTED]". Only applies when resolve=true.
    ///
    /// # Examples
    /// ```ignore
    /// // Show raw config with placeholders (safest, fastest)
    /// let raw = config.to_value(false, false)?;
    ///
    /// // Resolved with secrets redacted (safe for logs)
    /// let safe = config.to_value(true, true)?;
    ///
    /// // Resolved with secrets visible (use with caution)
    /// let full = config.to_value(true, false)?;
    /// ```
    pub fn to_value(&self, resolve: bool, redact: bool) -> Result<Value> {
        if !resolve {
            return Ok((*self.raw).clone());
        }
        let mut resolution_stack = Vec::new();
        if redact {
            self.resolve_value_to_value_redacted(&self.raw, "", &mut resolution_stack)
        } else {
            self.resolve_value_to_value(&self.raw, "", &mut resolution_stack)
        }
    }

    /// Export the configuration as YAML
    ///
    /// # Arguments
    /// * `resolve` - If true, resolve interpolations (${...}). If false, show placeholders.
    /// * `redact` - If true, replace sensitive values with "[REDACTED]". Only applies when resolve=true.
    ///
    /// # Examples
    /// ```ignore
    /// // Show raw config with placeholders
    /// let yaml = config.to_yaml(false, false)?;
    ///
    /// // Resolved with secrets redacted
    /// let yaml = config.to_yaml(true, true)?;
    /// ```
    pub fn to_yaml(&self, resolve: bool, redact: bool) -> Result<String> {
        let value = self.to_value(resolve, redact)?;
        serde_yaml::to_string(&value).map_err(|e| Error::parse(e.to_string()))
    }

    /// Export the configuration as JSON
    ///
    /// # Arguments
    /// * `resolve` - If true, resolve interpolations (${...}). If false, show placeholders.
    /// * `redact` - If true, replace sensitive values with "[REDACTED]". Only applies when resolve=true.
    ///
    /// # Examples
    /// ```ignore
    /// // Show raw config with placeholders
    /// let json = config.to_json(false, false)?;
    ///
    /// // Resolved with secrets redacted
    /// let json = config.to_json(true, true)?;
    /// ```
    pub fn to_json(&self, resolve: bool, redact: bool) -> Result<String> {
        let value = self.to_value(resolve, redact)?;
        serde_json::to_string_pretty(&value).map_err(|e| Error::parse(e.to_string()))
    }

    /// Clear the resolution cache
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }

    /// Get the source file for a config path
    ///
    /// Returns the filename of the config file that provided this value.
    /// For merged configs, this returns the file that "won" for this path.
    pub fn get_source(&self, path: &str) -> Option<&str> {
        self.source_map.get(path).map(|s| s.as_str())
    }

    /// Get all source mappings
    ///
    /// Returns a map of config paths to their source filenames.
    /// Useful for debugging which file each value came from.
    pub fn dump_sources(&self) -> &HashMap<String, String> {
        &self.source_map
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
    ///
    /// If `schema` is None, uses the attached schema (set via `set_schema()`).
    /// Returns an error if no schema is provided and none is attached.
    pub fn validate_raw(&self, schema: Option<&crate::schema::Schema>) -> Result<()> {
        let schema = self.resolve_schema(schema)?;
        schema.validate(&self.raw)
    }

    /// Validate the resolved configuration against a schema
    ///
    /// This performs type/value validation (Phase 2 per ADR-007):
    /// - Resolved values match expected types
    /// - Constraints (min, max, pattern, enum) are checked
    ///
    /// If `schema` is None, uses the attached schema (set via `set_schema()`).
    /// Returns an error if no schema is provided and none is attached.
    pub fn validate(&self, schema: Option<&crate::schema::Schema>) -> Result<()> {
        let schema = self.resolve_schema(schema)?;
        let resolved = self.to_value(true, false)?;
        schema.validate(&resolved)
    }

    /// Validate and collect all errors (instead of failing on first)
    ///
    /// If `schema` is None, uses the attached schema (set via `set_schema()`).
    /// Returns a single error if no schema is provided and none is attached.
    pub fn validate_collect(
        &self,
        schema: Option<&crate::schema::Schema>,
    ) -> Vec<crate::schema::ValidationError> {
        let schema = match self.resolve_schema(schema) {
            Ok(s) => s,
            Err(e) => {
                return vec![crate::schema::ValidationError {
                    path: String::new(),
                    message: e.to_string(),
                }]
            }
        };
        match self.to_value(true, false) {
            Ok(resolved) => schema.validate_collect(&resolved),
            Err(e) => vec![crate::schema::ValidationError {
                path: String::new(),
                message: e.to_string(),
            }],
        }
    }

    /// Helper to resolve which schema to use (provided or attached)
    fn resolve_schema<'a>(
        &'a self,
        schema: Option<&'a crate::schema::Schema>,
    ) -> Result<&'a crate::schema::Schema> {
        schema
            .or_else(|| self.schema.as_ref().map(|s| s.as_ref()))
            .ok_or_else(|| Error::validation("<root>", "No schema provided and none attached"))
    }

    /// Resolve a single value
    fn resolve_value(
        &self,
        value: &Value,
        path: &str,
        resolution_stack: &mut Vec<String>,
    ) -> Result<ResolvedValue> {
        match value {
            Value::String(s) => {
                // Use needs_processing to handle both interpolations AND escape sequences
                if interpolation::needs_processing(s) {
                    let parsed = interpolation::parse(s)?;
                    self.resolve_interpolation(&parsed, path, resolution_stack)
                } else {
                    Ok(ResolvedValue::new(value.clone()))
                }
            }
            _ => Ok(ResolvedValue::new(value.clone())),
        }
    }

    /// Resolve an interpolation expression
    fn resolve_interpolation(
        &self,
        interp: &Interpolation,
        path: &str,
        resolution_stack: &mut Vec<String>,
    ) -> Result<ResolvedValue> {
        match interp {
            Interpolation::Literal(s) => Ok(ResolvedValue::new(Value::String(s.clone()))),

            Interpolation::Resolver { name, args, kwargs } => {
                // Create resolver context with all options
                let mut ctx = ResolverContext::new(path);
                ctx.config_root = Some(Arc::clone(&self.raw));
                if let Some(base) = &self.options.base_path {
                    ctx.base_path = Some(base.clone());
                }
                // HTTP options
                ctx.allow_http = self.options.allow_http;
                ctx.http_allowlist = self.options.http_allowlist.clone();
                // TLS/Proxy options
                ctx.http_proxy = self.options.http_proxy.clone();
                ctx.http_proxy_from_env = self.options.http_proxy_from_env;
                ctx.http_ca_bundle = self.options.http_ca_bundle.clone();
                ctx.http_extra_ca_bundle = self.options.http_extra_ca_bundle.clone();
                ctx.http_client_cert = self.options.http_client_cert.clone();
                ctx.http_client_key = self.options.http_client_key.clone();
                ctx.http_client_key_password = self.options.http_client_key_password.clone();
                ctx.http_insecure = self.options.http_insecure;

                // Resolve arguments
                let resolved_args: Vec<String> = args
                    .iter()
                    .map(|arg| self.resolve_arg(arg, path, resolution_stack))
                    .collect::<Result<Vec<_>>>()?;

                // Extract and defer `default` kwarg for lazy resolution
                // Resolve all other kwargs eagerly
                let default_arg = kwargs.get("default");
                let resolved_kwargs: HashMap<String, String> = kwargs
                    .iter()
                    .filter(|(k, _)| *k != "default") // Don't resolve default yet
                    .map(|(k, v)| Ok((k.clone(), self.resolve_arg(v, path, resolution_stack)?)))
                    .collect::<Result<HashMap<_, _>>>()?;

                // Call the resolver (without `default` in kwargs)
                let result = self
                    .resolvers
                    .resolve(name, &resolved_args, &resolved_kwargs, &ctx);

                // Handle NotFound errors with lazy default resolution
                match result {
                    Ok(value) => Ok(value),
                    Err(e) => {
                        // Check if this is a "not found" type error that should use default
                        let should_use_default = matches!(
                            &e.kind,
                            crate::error::ErrorKind::Resolver(
                                crate::error::ResolverErrorKind::NotFound { .. }
                            ) | crate::error::ErrorKind::Resolver(
                                crate::error::ResolverErrorKind::EnvNotFound { .. }
                            ) | crate::error::ErrorKind::Resolver(
                                crate::error::ResolverErrorKind::FileNotFound { .. }
                            )
                        );

                        if should_use_default {
                            if let Some(default_arg) = default_arg {
                                // Lazily resolve the default value now
                                let default_str =
                                    self.resolve_arg(default_arg, path, resolution_stack)?;

                                // Apply sensitivity override if present
                                let is_sensitive = resolved_kwargs
                                    .get("sensitive")
                                    .map(|v| v.eq_ignore_ascii_case("true"))
                                    .unwrap_or(false);

                                return if is_sensitive {
                                    Ok(ResolvedValue::sensitive(Value::String(default_str)))
                                } else {
                                    Ok(ResolvedValue::new(Value::String(default_str)))
                                };
                            }
                        }
                        Err(e)
                    }
                }
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

                // Check for circular reference using the resolution stack
                if resolution_stack.contains(&full_path) {
                    // Build the cycle chain for the error message
                    let mut chain = resolution_stack.clone();
                    chain.push(full_path.clone());
                    return Err(Error::circular_reference(path, chain));
                }

                // Get the referenced value
                let ref_value = self
                    .raw
                    .get_path(&full_path)
                    .map_err(|_| Error::ref_not_found(&full_path, Some(path.to_string())))?;

                // Push onto the resolution stack before resolving
                resolution_stack.push(full_path.clone());

                // Resolve it recursively
                let result = self.resolve_value(ref_value, &full_path, resolution_stack);

                // Pop from the resolution stack after resolving
                resolution_stack.pop();

                result
            }

            Interpolation::Concat(parts) => {
                let mut result = String::new();
                let mut any_sensitive = false;

                for part in parts {
                    let resolved = self.resolve_interpolation(part, path, resolution_stack)?;
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
    fn resolve_arg(
        &self,
        arg: &InterpolationArg,
        path: &str,
        resolution_stack: &mut Vec<String>,
    ) -> Result<String> {
        match arg {
            InterpolationArg::Literal(s) => Ok(s.clone()),
            InterpolationArg::Nested(interp) => {
                let resolved = self.resolve_interpolation(interp, path, resolution_stack)?;
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
    fn resolve_value_recursive(
        &self,
        value: &Value,
        path: &str,
        resolution_stack: &mut Vec<String>,
    ) -> Result<ResolvedValue> {
        match value {
            Value::String(s) => {
                if interpolation::needs_processing(s) {
                    let parsed = interpolation::parse(s)?;
                    let resolved = self.resolve_interpolation(&parsed, path, resolution_stack)?;

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
                    self.resolve_value_recursive(item, &item_path, resolution_stack)?;
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
                    self.resolve_value_recursive(val, &key_path, resolution_stack)?;
                }
                Ok(ResolvedValue::new(value.clone()))
            }
            _ => Ok(ResolvedValue::new(value.clone())),
        }
    }

    /// Resolve a value tree to a new Value
    fn resolve_value_to_value(
        &self,
        value: &Value,
        path: &str,
        resolution_stack: &mut Vec<String>,
    ) -> Result<Value> {
        match value {
            Value::String(s) => {
                if interpolation::needs_processing(s) {
                    let parsed = interpolation::parse(s)?;
                    let resolved = self.resolve_interpolation(&parsed, path, resolution_stack)?;
                    Ok(resolved.value)
                } else {
                    Ok(value.clone())
                }
            }
            Value::Sequence(seq) => {
                let mut resolved_seq = Vec::new();
                for (i, item) in seq.iter().enumerate() {
                    let item_path = format!("{}[{}]", path, i);
                    resolved_seq.push(self.resolve_value_to_value(
                        item,
                        &item_path,
                        resolution_stack,
                    )?);
                }
                Ok(Value::Sequence(resolved_seq))
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
                        self.resolve_value_to_value(val, &key_path, resolution_stack)?,
                    );
                }
                Ok(Value::Mapping(resolved))
            }
            _ => Ok(value.clone()),
        }
    }

    /// Resolve a value tree to a new Value with sensitive value redaction
    fn resolve_value_to_value_redacted(
        &self,
        value: &Value,
        path: &str,
        resolution_stack: &mut Vec<String>,
    ) -> Result<Value> {
        const REDACTED: &str = "[REDACTED]";

        match value {
            Value::String(s) => {
                if interpolation::needs_processing(s) {
                    let parsed = interpolation::parse(s)?;
                    let resolved = self.resolve_interpolation(&parsed, path, resolution_stack)?;
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
                let mut resolved_seq = Vec::new();
                for (i, item) in seq.iter().enumerate() {
                    let item_path = format!("{}[{}]", path, i);
                    resolved_seq.push(self.resolve_value_to_value_redacted(
                        item,
                        &item_path,
                        resolution_stack,
                    )?);
                }
                Ok(Value::Sequence(resolved_seq))
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
                        self.resolve_value_to_value_redacted(val, &key_path, resolution_stack)?,
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
            source_map: Arc::clone(&self.source_map),
            resolvers: Arc::clone(&self.resolvers),
            options: self.options.clone(),
            schema: self.schema.clone(),
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
  host: ${env:HOLOCONF_MISSING_VAR,default=default-host}
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
    fn test_boolean_coercion_case_insensitive() {
        // Test case-insensitive boolean coercion per ADR-012
        let yaml = r#"
lower_true: "true"
upper_true: "TRUE"
mixed_true: "True"
lower_false: "false"
upper_false: "FALSE"
mixed_false: "False"
"#;
        let config = Config::from_yaml(yaml).unwrap();

        // All variations of "true" should work
        assert!(config.get_bool("lower_true").unwrap());
        assert!(config.get_bool("upper_true").unwrap());
        assert!(config.get_bool("mixed_true").unwrap());

        // All variations of "false" should work
        assert!(!config.get_bool("lower_false").unwrap());
        assert!(!config.get_bool("upper_false").unwrap());
        assert!(!config.get_bool("mixed_false").unwrap());
    }

    #[test]
    fn test_boolean_coercion_rejects_invalid() {
        // Test that invalid boolean strings are rejected per ADR-012
        let yaml = r#"
yes_value: "yes"
no_value: "no"
one_value: "1"
zero_value: "0"
on_value: "on"
off_value: "off"
"#;
        let config = Config::from_yaml(yaml).unwrap();

        // None of these should work
        assert!(config.get_bool("yes_value").is_err());
        assert!(config.get_bool("no_value").is_err());
        assert!(config.get_bool("one_value").is_err());
        assert!(config.get_bool("zero_value").is_err());
        assert!(config.get_bool("on_value").is_err());
        assert!(config.get_bool("off_value").is_err());
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
    fn test_to_yaml_resolved() {
        std::env::set_var("HOLOCONF_EXPORT_HOST", "exported-host");

        let yaml = r#"
server:
  host: ${env:HOLOCONF_EXPORT_HOST}
  port: 8080
"#;
        let config = Config::from_yaml(yaml).unwrap();

        let exported = config.to_yaml(true, false).unwrap();
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
host: ${env:UNDEFINED_HOST,default=${env:HOLOCONF_DEFAULT_HOST}}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        assert_eq!(config.get("host").unwrap().as_str(), Some("fallback-host"));

        std::env::remove_var("HOLOCONF_DEFAULT_HOST");
    }

    #[test]
    fn test_to_yaml_unresolved() {
        let yaml = r#"
server:
  host: ${env:MY_HOST}
  port: 8080
"#;
        let config = Config::from_yaml(yaml).unwrap();

        let raw = config.to_yaml(false, false).unwrap();
        // Should contain the placeholder, not a resolved value
        assert!(raw.contains("${env:MY_HOST}"));
        assert!(raw.contains("8080"));
    }

    #[test]
    fn test_to_json_unresolved() {
        let yaml = r#"
database:
  url: ${env:DATABASE_URL}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        let raw = config.to_json(false, false).unwrap();
        // Should contain the placeholder
        assert!(raw.contains("${env:DATABASE_URL}"));
    }

    #[test]
    fn test_to_value_unresolved() {
        let yaml = r#"
key: ${env:SOME_VAR}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        let raw = config.to_value(false, false).unwrap();
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
        let output = config.to_yaml(true, true).unwrap();
        assert!(output.contains("public-value"));

        std::env::remove_var("HOLOCONF_NON_SENSITIVE");
    }

    #[test]
    fn test_circular_reference_direct() {
        // Direct circular reference: a -> b -> a
        let yaml = r#"
a: ${b}
b: ${a}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        // Accessing 'a' should detect the circular reference
        let result = config.get("a");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("circular"),
            "Error should mention 'circular': {}",
            err
        );
    }

    #[test]
    fn test_circular_reference_chain() {
        // Chain circular reference: first -> second -> third -> first
        let yaml = r#"
first: ${second}
second: ${third}
third: ${first}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        // Accessing 'first' should detect the circular reference chain
        let result = config.get("first");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("circular"),
            "Error should mention 'circular': {}",
            err
        );
    }

    #[test]
    fn test_circular_reference_self() {
        // Self-referential: value references itself
        let yaml = r#"
value: ${value}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        let result = config.get("value");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("circular"),
            "Error should mention 'circular': {}",
            err
        );
    }

    #[test]
    fn test_circular_reference_nested() {
        // Circular reference in nested structure
        let yaml = r#"
database:
  primary: ${database.secondary}
  secondary: ${database.primary}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        let result = config.get("database.primary");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("circular"),
            "Error should mention 'circular': {}",
            err
        );
    }

    // Source tracking tests

    #[test]
    fn test_get_source_from_yaml_string() {
        // Config loaded from YAML string has no source tracking
        let yaml = r#"
database:
  host: localhost
"#;
        let config = Config::from_yaml(yaml).unwrap();

        // No source info for YAML string (no filename)
        assert!(config.get_source("database.host").is_none());
        assert!(config.dump_sources().is_empty());
    }

    #[test]
    fn test_source_tracking_load_and_merge() {
        // Create temp files for testing
        let temp_dir = std::env::temp_dir().join("holoconf_test_sources");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let base_path = temp_dir.join("base.yaml");
        let override_path = temp_dir.join("override.yaml");

        std::fs::write(
            &base_path,
            r#"
database:
  host: localhost
  port: 5432
api:
  url: http://localhost
"#,
        )
        .unwrap();

        std::fs::write(
            &override_path,
            r#"
database:
  host: prod-db.example.com
api:
  key: secret123
"#,
        )
        .unwrap();

        // Use the new API: load and merge
        let mut config = Config::load(&base_path).unwrap();
        let override_config = Config::load(&override_path).unwrap();
        config.merge(override_config);

        // Verify merged values (source tracking currently doesn't persist through merge)
        assert_eq!(
            config.get("database.host").unwrap().as_str(),
            Some("prod-db.example.com")
        );
        assert_eq!(config.get("database.port").unwrap().as_i64(), Some(5432));
        assert_eq!(
            config.get("api.url").unwrap().as_str(),
            Some("http://localhost")
        );
        assert_eq!(config.get("api.key").unwrap().as_str(), Some("secret123"));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_source_tracking_single_file() {
        let temp_dir = std::env::temp_dir().join("holoconf_test_single");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let config_path = temp_dir.join("config.yaml");
        std::fs::write(
            &config_path,
            r#"
database:
  host: localhost
  port: 5432
"#,
        )
        .unwrap();

        let config = Config::load(&config_path).unwrap();

        // All values should come from config.yaml
        assert_eq!(config.get_source("database.host"), Some("config.yaml"));
        assert_eq!(config.get_source("database.port"), Some("config.yaml"));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_null_removes_values_on_merge() {
        let temp_dir = std::env::temp_dir().join("holoconf_test_null");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let base_path = temp_dir.join("base.yaml");
        let override_path = temp_dir.join("override.yaml");

        std::fs::write(
            &base_path,
            r#"
database:
  host: localhost
  port: 5432
  debug: true
"#,
        )
        .unwrap();

        std::fs::write(
            &override_path,
            r#"
database:
  debug: null
"#,
        )
        .unwrap();

        let mut config = Config::load(&base_path).unwrap();
        let override_config = Config::load(&override_path).unwrap();
        config.merge(override_config);

        // debug should be removed
        assert!(config.get("database.debug").is_err());
        // Others should remain
        assert_eq!(
            config.get("database.host").unwrap().as_str(),
            Some("localhost")
        );
        assert_eq!(config.get("database.port").unwrap().as_i64(), Some(5432));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_array_replacement_on_merge() {
        let temp_dir = std::env::temp_dir().join("holoconf_test_array");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let base_path = temp_dir.join("base.yaml");
        let override_path = temp_dir.join("override.yaml");

        std::fs::write(
            &base_path,
            r#"
servers:
  - host: server1
  - host: server2
"#,
        )
        .unwrap();

        std::fs::write(
            &override_path,
            r#"
servers:
  - host: prod-server
"#,
        )
        .unwrap();

        let mut config = Config::load(&base_path).unwrap();
        let override_config = Config::load(&override_path).unwrap();
        config.merge(override_config);

        // Array is replaced, so only one item
        assert_eq!(
            config.get("servers[0].host").unwrap().as_str(),
            Some("prod-server")
        );
        // server2 no longer exists
        assert!(config.get("servers[1].host").is_err());

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    // Optional file tests

    #[test]
    fn test_optional_file_missing() {
        let temp_dir = std::env::temp_dir().join("holoconf_test_optional_missing");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let base_path = temp_dir.join("base.yaml");
        let optional_path = temp_dir.join("optional.yaml"); // Does not exist

        std::fs::write(
            &base_path,
            r#"
database:
  host: localhost
  port: 5432
"#,
        )
        .unwrap();

        // Use the new API: Config::optional() returns empty config if missing
        let mut config = Config::load(&base_path).unwrap();
        let optional_config = Config::optional(&optional_path).unwrap();
        config.merge(optional_config);

        // Base values should be present
        assert_eq!(
            config.get("database.host").unwrap().as_str(),
            Some("localhost")
        );
        assert_eq!(config.get("database.port").unwrap().as_i64(), Some(5432));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_optional_file_exists() {
        let temp_dir = std::env::temp_dir().join("holoconf_test_optional_exists");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let base_path = temp_dir.join("base.yaml");
        let optional_path = temp_dir.join("optional.yaml");

        std::fs::write(
            &base_path,
            r#"
database:
  host: localhost
  port: 5432
"#,
        )
        .unwrap();

        std::fs::write(
            &optional_path,
            r#"
database:
  host: prod-db
"#,
        )
        .unwrap();

        // Use the new API
        let mut config = Config::load(&base_path).unwrap();
        let optional_config = Config::optional(&optional_path).unwrap();
        config.merge(optional_config);

        // Optional file should override base
        assert_eq!(
            config.get("database.host").unwrap().as_str(),
            Some("prod-db")
        );
        assert_eq!(config.get("database.port").unwrap().as_i64(), Some(5432));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_required_file_missing_errors() {
        let temp_dir = std::env::temp_dir().join("holoconf_test_required_missing");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let missing_path = temp_dir.join("missing.yaml"); // Does not exist

        // Config::load() errors on missing file
        let result = Config::load(&missing_path);

        match result {
            Ok(_) => panic!("Expected error for missing required file"),
            Err(err) => {
                assert!(
                    err.to_string().contains("File not found"),
                    "Error should mention file not found: {}",
                    err
                );
            }
        }

        // Config::required() is an alias and should also error
        let result2 = Config::required(&missing_path);
        assert!(result2.is_err());

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_all_optional_files_missing() {
        let temp_dir = std::env::temp_dir().join("holoconf_test_all_optional_missing");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let optional1 = temp_dir.join("optional1.yaml");
        let optional2 = temp_dir.join("optional2.yaml");

        // Both files don't exist - Config::optional() returns empty config
        let mut config = Config::optional(&optional1).unwrap();
        let config2 = Config::optional(&optional2).unwrap();
        config.merge(config2);

        // Should return empty config
        let value = config.to_value(false, false).unwrap();
        assert!(value.as_mapping().unwrap().is_empty());

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_mixed_required_and_optional() {
        let temp_dir = std::env::temp_dir().join("holoconf_test_mixed_req_opt");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let required1 = temp_dir.join("required1.yaml");
        let optional1 = temp_dir.join("optional1.yaml"); // Missing
        let required2 = temp_dir.join("required2.yaml");
        let optional2 = temp_dir.join("optional2.yaml");

        std::fs::write(
            &required1,
            r#"
app:
  name: myapp
  debug: false
"#,
        )
        .unwrap();

        std::fs::write(
            &required2,
            r#"
database:
  host: localhost
"#,
        )
        .unwrap();

        std::fs::write(
            &optional2,
            r#"
app:
  debug: true
database:
  port: 5432
"#,
        )
        .unwrap();

        // Use new API: load required files with load(), optional with optional(), then merge
        let mut config = Config::load(&required1).unwrap();
        let opt1 = Config::optional(&optional1).unwrap(); // Missing, returns empty
        config.merge(opt1);
        let req2 = Config::load(&required2).unwrap();
        config.merge(req2);
        let opt2 = Config::optional(&optional2).unwrap(); // Exists
        config.merge(opt2);

        // Check merged values
        assert_eq!(config.get("app.name").unwrap().as_str(), Some("myapp"));
        assert_eq!(config.get("app.debug").unwrap().as_bool(), Some(true)); // From optional2
        assert_eq!(
            config.get("database.host").unwrap().as_str(),
            Some("localhost")
        );
        assert_eq!(config.get("database.port").unwrap().as_i64(), Some(5432)); // From optional2

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_from_json() {
        let json = r#"{"database": {"host": "localhost", "port": 5432}}"#;
        let config = Config::from_json(json).unwrap();

        assert_eq!(
            config.get("database.host").unwrap().as_str(),
            Some("localhost")
        );
        assert_eq!(config.get("database.port").unwrap().as_i64(), Some(5432));
    }

    #[test]
    fn test_from_json_invalid() {
        let json = r#"{"unclosed": "#;
        let result = Config::from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_raw() {
        let yaml = r#"
key: ${env:SOME_VAR,default=fallback}
literal: plain_value
"#;
        let config = Config::from_yaml(yaml).unwrap();

        // get_raw returns the unresolved value
        let raw = config.get_raw("key").unwrap();
        assert!(raw.as_str().unwrap().contains("${env:"));

        // literal values are unchanged
        let literal = config.get_raw("literal").unwrap();
        assert_eq!(literal.as_str(), Some("plain_value"));
    }

    #[test]
    fn test_get_string() {
        std::env::set_var("HOLOCONF_TEST_STRING", "hello_world");

        let yaml = r#"
plain: "plain_string"
env_var: ${env:HOLOCONF_TEST_STRING}
number: 42
"#;
        let config = Config::from_yaml(yaml).unwrap();

        assert_eq!(config.get_string("plain").unwrap(), "plain_string");
        assert_eq!(config.get_string("env_var").unwrap(), "hello_world");

        // Numbers get coerced to string
        assert_eq!(config.get_string("number").unwrap(), "42");

        std::env::remove_var("HOLOCONF_TEST_STRING");
    }

    #[test]
    fn test_get_f64() {
        let yaml = r#"
float: 1.23
int: 42
string_num: "4.56"
string_bad: "not_a_number"
"#;
        let config = Config::from_yaml(yaml).unwrap();

        assert!((config.get_f64("float").unwrap() - 1.23).abs() < 0.001);
        assert!((config.get_f64("int").unwrap() - 42.0).abs() < 0.001);
        // Strings that look like numbers ARE coerced
        assert!((config.get_f64("string_num").unwrap() - 4.56).abs() < 0.001);
        // Strings that don't parse as numbers should error
        assert!(config.get_f64("string_bad").is_err());
    }

    #[test]
    fn test_config_merge() {
        let yaml1 = r#"
database:
  host: localhost
  port: 5432
app:
  name: myapp
"#;
        let yaml2 = r#"
database:
  port: 3306
  user: admin
app:
  debug: true
"#;
        let mut config1 = Config::from_yaml(yaml1).unwrap();
        let config2 = Config::from_yaml(yaml2).unwrap();

        config1.merge(config2);

        // Merged values
        assert_eq!(
            config1.get("database.host").unwrap().as_str(),
            Some("localhost")
        );
        assert_eq!(config1.get("database.port").unwrap().as_i64(), Some(3306)); // Overwritten
        assert_eq!(
            config1.get("database.user").unwrap().as_str(),
            Some("admin")
        ); // Added
        assert_eq!(config1.get("app.name").unwrap().as_str(), Some("myapp"));
        assert_eq!(config1.get("app.debug").unwrap().as_bool(), Some(true)); // Added
    }

    #[test]
    fn test_config_clone() {
        let yaml = r#"
key: value
nested:
  a: 1
  b: 2
"#;
        let config = Config::from_yaml(yaml).unwrap();
        let cloned = config.clone();

        assert_eq!(cloned.get("key").unwrap().as_str(), Some("value"));
        assert_eq!(cloned.get("nested.a").unwrap().as_i64(), Some(1));
    }

    #[test]
    fn test_with_options() {
        use indexmap::IndexMap;
        let mut map = IndexMap::new();
        map.insert("key".to_string(), crate::Value::String("value".to_string()));
        let value = crate::Value::Mapping(map);
        let options = ConfigOptions {
            base_path: None,
            allow_http: true,
            http_allowlist: vec![],
            file_roots: vec!["/custom/path".into()],
            ..Default::default()
        };
        let config = Config::with_options(value, options);

        assert_eq!(config.get("key").unwrap().as_str(), Some("value"));
    }

    #[test]
    fn test_from_yaml_with_options() {
        let yaml = "key: value";
        let options = ConfigOptions {
            base_path: None,
            allow_http: true,
            http_allowlist: vec![],
            file_roots: vec![],
            ..Default::default()
        };
        let config = Config::from_yaml_with_options(yaml, options).unwrap();

        assert_eq!(config.get("key").unwrap().as_str(), Some("value"));
    }

    #[test]
    fn test_resolve_all() {
        std::env::set_var("HOLOCONF_RESOLVE_ALL_TEST", "resolved");

        let yaml = r#"
a: ${env:HOLOCONF_RESOLVE_ALL_TEST}
b: static_value
c:
  nested: ${env:HOLOCONF_RESOLVE_ALL_TEST}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        // resolve_all should resolve all values without errors
        config.resolve_all().unwrap();

        // All values should be cached now
        assert_eq!(config.get("a").unwrap().as_str(), Some("resolved"));
        assert_eq!(config.get("b").unwrap().as_str(), Some("static_value"));
        assert_eq!(config.get("c.nested").unwrap().as_str(), Some("resolved"));

        std::env::remove_var("HOLOCONF_RESOLVE_ALL_TEST");
    }

    #[test]
    fn test_resolve_all_with_errors() {
        let yaml = r#"
valid: static
invalid: ${env:HOLOCONF_NONEXISTENT_RESOLVE_VAR}
"#;
        std::env::remove_var("HOLOCONF_NONEXISTENT_RESOLVE_VAR");

        let config = Config::from_yaml(yaml).unwrap();
        let result = config.resolve_all();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_self_reference_basic() {
        // Test basic self-references (without default kwargs - kwargs not yet implemented for self-ref)
        let yaml = r#"
settings:
  timeout: 30
app:
  timeout: ${settings.timeout}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        assert_eq!(config.get("app.timeout").unwrap().as_i64(), Some(30));
    }

    #[test]
    fn test_self_reference_missing_errors() {
        let yaml = r#"
app:
  timeout: ${settings.missing_timeout}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        // Should error when path doesn't exist (no default support for self-refs yet)
        let result = config.get("app.timeout");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_self_reference_sensitivity_inheritance() {
        std::env::set_var("HOLOCONF_INHERITED_SECRET", "secret_value");

        let yaml = r#"
secrets:
  api_key: ${env:HOLOCONF_INHERITED_SECRET,sensitive=true}
derived: ${secrets.api_key}
"#;
        let config = Config::from_yaml(yaml).unwrap();

        // Access the values to ensure resolution works
        assert_eq!(
            config.get("secrets.api_key").unwrap().as_str(),
            Some("secret_value")
        );
        assert_eq!(
            config.get("derived").unwrap().as_str(),
            Some("secret_value")
        );

        // Check that serialization redacts sensitive values
        let yaml_output = config.to_yaml(true, true).unwrap();
        assert!(yaml_output.contains("[REDACTED]"));
        assert!(!yaml_output.contains("secret_value"));

        std::env::remove_var("HOLOCONF_INHERITED_SECRET");
    }

    #[test]
    fn test_non_notfound_error_does_not_use_default() {
        // Register a resolver that returns a non-NotFound error
        use crate::resolver::FnResolver;
        use std::sync::Arc;

        let yaml = r#"
value: ${failing:arg,default=should_not_be_used}
"#;
        let mut config = Config::from_yaml(yaml).unwrap();

        // Register a resolver that fails with a custom error (not NotFound)
        config.register_resolver(Arc::new(FnResolver::new(
            "failing",
            |_args, _kwargs, ctx| {
                Err(
                    crate::error::Error::resolver_custom("failing", "Network timeout")
                        .with_path(ctx.config_path.clone()),
                )
            },
        )));

        // The default should NOT be used because the error is not a NotFound type
        let result = config.get("value");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Network timeout"));
    }

    // Tests for schema default integration

    #[test]
    fn test_get_returns_schema_default() {
        use crate::schema::Schema;

        let yaml = r#"
database:
  host: localhost
"#;
        let schema_yaml = r#"
type: object
properties:
  database:
    type: object
    properties:
      host:
        type: string
      port:
        type: integer
        default: 5432
      pool_size:
        type: integer
        default: 10
"#;
        let mut config = Config::from_yaml(yaml).unwrap();
        let schema = Schema::from_yaml(schema_yaml).unwrap();
        config.set_schema(schema);

        // Existing value should work
        assert_eq!(
            config.get("database.host").unwrap(),
            Value::String("localhost".into())
        );

        // Missing value with schema default should return default
        assert_eq!(config.get("database.port").unwrap(), Value::Integer(5432));
        assert_eq!(
            config.get("database.pool_size").unwrap(),
            Value::Integer(10)
        );
    }

    #[test]
    fn test_config_value_overrides_schema_default() {
        use crate::schema::Schema;

        let yaml = r#"
port: 3000
"#;
        let schema_yaml = r#"
type: object
properties:
  port:
    type: integer
    default: 8080
"#;
        let mut config = Config::from_yaml(yaml).unwrap();
        let schema = Schema::from_yaml(schema_yaml).unwrap();
        config.set_schema(schema);

        // Config value should win over schema default
        assert_eq!(config.get("port").unwrap(), Value::Integer(3000));
    }

    #[test]
    fn test_no_schema_raises_path_not_found() {
        let yaml = r#"
existing: value
"#;
        let config = Config::from_yaml(yaml).unwrap();

        // Without schema, missing path should error
        let result = config.get("missing");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind,
            crate::error::ErrorKind::PathNotFound
        ));
    }

    #[test]
    fn test_missing_path_no_default_raises_error() {
        use crate::schema::Schema;

        let yaml = r#"
existing: value
"#;
        let schema_yaml = r#"
type: object
properties:
  existing:
    type: string
  no_default:
    type: string
"#;
        let mut config = Config::from_yaml(yaml).unwrap();
        let schema = Schema::from_yaml(schema_yaml).unwrap();
        config.set_schema(schema);

        // Path exists in schema but no default, should error
        let result = config.get("no_default");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind,
            crate::error::ErrorKind::PathNotFound
        ));
    }

    #[test]
    fn test_validate_uses_attached_schema() {
        use crate::schema::Schema;

        let yaml = r#"
name: test
port: 8080
"#;
        let schema_yaml = r#"
type: object
required:
  - name
  - port
properties:
  name:
    type: string
  port:
    type: integer
"#;
        let mut config = Config::from_yaml(yaml).unwrap();
        let schema = Schema::from_yaml(schema_yaml).unwrap();
        config.set_schema(schema);

        // validate() with no arg should use attached schema
        assert!(config.validate(None).is_ok());
    }

    #[test]
    fn test_validate_no_schema_errors() {
        let yaml = r#"
name: test
"#;
        let config = Config::from_yaml(yaml).unwrap();

        // validate() with no arg and no attached schema should error
        let result = config.validate(None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("No schema"));
    }

    #[test]
    fn test_null_value_uses_default_when_null_disallowed() {
        use crate::schema::Schema;

        let yaml = r#"
value: null
"#;
        let schema_yaml = r#"
type: object
properties:
  value:
    type: string
    default: "fallback"
"#;
        let mut config = Config::from_yaml(yaml).unwrap();
        let schema = Schema::from_yaml(schema_yaml).unwrap();
        config.set_schema(schema);

        // Null value with non-nullable schema type should use default
        assert_eq!(
            config.get("value").unwrap(),
            Value::String("fallback".into())
        );
    }

    #[test]
    fn test_null_value_preserved_when_null_allowed() {
        use crate::schema::Schema;

        let yaml = r#"
value: null
"#;
        let schema_yaml = r#"
type: object
properties:
  value:
    type:
      - string
      - "null"
    default: "fallback"
"#;
        let mut config = Config::from_yaml(yaml).unwrap();
        let schema = Schema::from_yaml(schema_yaml).unwrap();
        config.set_schema(schema);

        // Null value with nullable schema type should preserve null
        assert_eq!(config.get("value").unwrap(), Value::Null);
    }

    #[test]
    fn test_set_and_get_schema() {
        use crate::schema::Schema;

        let yaml = r#"
name: test
"#;
        let mut config = Config::from_yaml(yaml).unwrap();

        // No schema initially
        assert!(config.get_schema().is_none());

        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  name:
    type: string
"#,
        )
        .unwrap();
        config.set_schema(schema);

        // Schema should now be attached
        assert!(config.get_schema().is_some());
    }
}

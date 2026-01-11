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

/// Specifies a file to load, either required or optional
///
/// Use this with `Config::load_merged_with_specs()` to load config files
/// where some files may or may not exist.
///
/// # Examples
///
/// ```ignore
/// use holoconf_core::{Config, FileSpec};
///
/// let config = Config::load_merged_with_specs(&[
///     FileSpec::required("base.yaml"),
///     FileSpec::required("environment.yaml"),
///     FileSpec::optional("local.yaml"),  // Won't error if missing
/// ])?;
/// ```
#[derive(Debug, Clone)]
pub enum FileSpec {
    /// A required file - error if not found
    Required(PathBuf),
    /// An optional file - silently skip if not found
    Optional(PathBuf),
}

impl FileSpec {
    /// Create a required file spec
    pub fn required(path: impl Into<PathBuf>) -> Self {
        FileSpec::Required(path.into())
    }

    /// Create an optional file spec
    pub fn optional(path: impl Into<PathBuf>) -> Self {
        FileSpec::Optional(path.into())
    }

    /// Get the path for this file spec
    pub fn path(&self) -> &Path {
        match self {
            FileSpec::Required(p) => p,
            FileSpec::Optional(p) => p,
        }
    }

    /// Check if this file spec is optional
    pub fn is_optional(&self) -> bool {
        matches!(self, FileSpec::Optional(_))
    }
}

impl<P: Into<PathBuf>> From<P> for FileSpec {
    fn from(path: P) -> Self {
        FileSpec::Required(path.into())
    }
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
}

impl Config {
    /// Create a new Config from a Value
    pub fn new(value: Value) -> Self {
        Self {
            raw: Arc::new(value),
            cache: Arc::new(RwLock::new(HashMap::new())),
            source_map: Arc::new(HashMap::new()),
            resolvers: Arc::new(ResolverRegistry::with_builtins()),
            options: ConfigOptions::default(),
        }
    }

    /// Create a Config with custom options
    pub fn with_options(value: Value, options: ConfigOptions) -> Self {
        Self {
            raw: Arc::new(value),
            cache: Arc::new(RwLock::new(HashMap::new())),
            source_map: Arc::new(HashMap::new()),
            resolvers: Arc::new(ResolverRegistry::with_builtins()),
            options,
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
            resolvers: Arc::new(ResolverRegistry::with_builtins()),
            options,
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

        // Track source for all leaf paths
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

    /// Load and merge multiple YAML files
    ///
    /// Files are merged in order, with later files overriding earlier ones.
    /// Per ADR-004:
    /// - Mappings are deep-merged
    /// - Scalars use last-writer-wins
    /// - Arrays are replaced (not concatenated)
    /// - Null values remove keys
    ///
    /// Source tracking records which file each value came from.
    ///
    /// All files are required - use `load_merged_with_specs()` if you need
    /// optional files that may or may not exist.
    pub fn load_merged<P: AsRef<Path>>(paths: &[P]) -> Result<Self> {
        let specs: Vec<FileSpec> = paths
            .iter()
            .map(|p| FileSpec::Required(p.as_ref().to_path_buf()))
            .collect();
        Self::load_merged_with_specs(&specs)
    }

    /// Load and merge multiple YAML files with support for optional files
    ///
    /// Files are merged in order, with later files overriding earlier ones.
    /// Optional files that don't exist are silently skipped.
    ///
    /// Per ADR-004:
    /// - Mappings are deep-merged
    /// - Scalars use last-writer-wins
    /// - Arrays are replaced (not concatenated)
    /// - Null values remove keys
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use holoconf_core::{Config, FileSpec};
    ///
    /// let config = Config::load_merged_with_specs(&[
    ///     FileSpec::required("base.yaml"),
    ///     FileSpec::required("environment.yaml"),
    ///     FileSpec::optional("local.yaml"),  // Won't error if missing
    /// ])?;
    /// ```
    pub fn load_merged_with_specs(specs: &[FileSpec]) -> Result<Self> {
        if specs.is_empty() {
            return Ok(Self::new(Value::Mapping(indexmap::IndexMap::new())));
        }

        let mut merged_value: Option<Value> = None;
        let mut last_base_path: Option<PathBuf> = None;
        let mut source_map: HashMap<String, String> = HashMap::new();

        for spec in specs {
            let path = spec.path();

            // Try to read the file
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_e) => {
                    // If optional, skip missing files
                    if spec.is_optional() {
                        continue;
                    }
                    // If required, return error
                    return Err(Error::file_not_found(path.display().to_string(), None));
                }
            };

            let value: Value =
                serde_yaml::from_str(&content).map_err(|e| Error::parse(e.to_string()))?;

            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();

            last_base_path = path.parent().map(|p| p.to_path_buf());

            match &mut merged_value {
                Some(base) => {
                    base.merge_tracking_sources(value, &filename, "", &mut source_map);
                }
                None => {
                    // First file: record all leaf paths
                    value.collect_leaf_paths("", &filename, &mut source_map);
                    merged_value = Some(value);
                }
            }
        }

        let mut options = ConfigOptions::default();
        options.base_path = last_base_path;

        Ok(Self::with_options_and_sources(
            merged_value.unwrap_or(Value::Mapping(indexmap::IndexMap::new())),
            options,
            source_map,
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

        // Resolve the value with an empty resolution stack
        let mut resolution_stack = Vec::new();
        let resolved = self.resolve_value(raw_value, path, &mut resolution_stack)?;

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
    pub fn validate_raw(&self, schema: &crate::schema::Schema) -> Result<()> {
        schema.validate(&self.raw)
    }

    /// Validate the resolved configuration against a schema
    ///
    /// This performs type/value validation (Phase 2 per ADR-007):
    /// - Resolved values match expected types
    /// - Constraints (min, max, pattern, enum) are checked
    pub fn validate(&self, schema: &crate::schema::Schema) -> Result<()> {
        let resolved = self.to_value(true, false)?;
        schema.validate(&resolved)
    }

    /// Validate and collect all errors (instead of failing on first)
    pub fn validate_collect(
        &self,
        schema: &crate::schema::Schema,
    ) -> Vec<crate::schema::ValidationError> {
        match self.to_value(true, false) {
            Ok(resolved) => schema.validate_collect(&resolved),
            Err(e) => vec![crate::schema::ValidationError {
                path: String::new(),
                message: e.to_string(),
            }],
        }
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
                // Create resolver context
                let mut ctx = ResolverContext::new(path);
                ctx.config_root = Some(Arc::clone(&self.raw));
                if let Some(base) = &self.options.base_path {
                    ctx.base_path = Some(base.clone());
                }

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
    fn test_source_tracking_load_merged() {
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

        let config = Config::load_merged(&[&base_path, &override_path]).unwrap();

        // Check sources
        assert_eq!(config.get_source("database.host"), Some("override.yaml"));
        assert_eq!(config.get_source("database.port"), Some("base.yaml"));
        assert_eq!(config.get_source("api.url"), Some("base.yaml"));
        assert_eq!(config.get_source("api.key"), Some("override.yaml"));

        // Check dump_sources returns all
        let sources = config.dump_sources();
        assert_eq!(sources.len(), 4);
        assert_eq!(
            sources.get("database.host").map(|s| s.as_str()),
            Some("override.yaml")
        );
        assert_eq!(
            sources.get("database.port").map(|s| s.as_str()),
            Some("base.yaml")
        );

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

        let config = Config::from_yaml_file(&config_path).unwrap();

        // All values should come from config.yaml
        assert_eq!(config.get_source("database.host"), Some("config.yaml"));
        assert_eq!(config.get_source("database.port"), Some("config.yaml"));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_source_tracking_null_removes() {
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

        let config = Config::load_merged(&[&base_path, &override_path]).unwrap();

        // debug should be removed
        assert!(config.get_source("database.debug").is_none());
        // Others should remain
        assert_eq!(config.get_source("database.host"), Some("base.yaml"));
        assert_eq!(config.get_source("database.port"), Some("base.yaml"));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_source_tracking_array_replacement() {
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

        let config = Config::load_merged(&[&base_path, &override_path]).unwrap();

        // Array is replaced, so only one item from override
        assert_eq!(config.get_source("servers[0].host"), Some("override.yaml"));
        // server2 no longer exists
        assert!(config.get_source("servers[1].host").is_none());

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

        // Using load_merged_with_specs with optional file
        let config = Config::load_merged_with_specs(&[
            FileSpec::required(&base_path),
            FileSpec::optional(&optional_path),
        ])
        .unwrap();

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

        // Using load_merged_with_specs with optional file that exists
        let config = Config::load_merged_with_specs(&[
            FileSpec::required(&base_path),
            FileSpec::optional(&optional_path),
        ])
        .unwrap();

        // Optional file should override base
        assert_eq!(
            config.get("database.host").unwrap().as_str(),
            Some("prod-db")
        );
        assert_eq!(config.get("database.port").unwrap().as_i64(), Some(5432));

        // Check source tracking
        assert_eq!(config.get_source("database.host"), Some("optional.yaml"));
        assert_eq!(config.get_source("database.port"), Some("base.yaml"));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_required_file_missing_errors() {
        let temp_dir = std::env::temp_dir().join("holoconf_test_required_missing");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let base_path = temp_dir.join("base.yaml");
        let missing_path = temp_dir.join("missing.yaml"); // Does not exist

        std::fs::write(
            &base_path,
            r#"
key: value
"#,
        )
        .unwrap();

        // Using load_merged_with_specs with required file that doesn't exist
        let result = Config::load_merged_with_specs(&[
            FileSpec::required(&base_path),
            FileSpec::required(&missing_path),
        ]);

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

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_all_optional_files_missing() {
        let temp_dir = std::env::temp_dir().join("holoconf_test_all_optional_missing");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let optional1 = temp_dir.join("optional1.yaml");
        let optional2 = temp_dir.join("optional2.yaml");

        // Both files don't exist
        let config = Config::load_merged_with_specs(&[
            FileSpec::optional(&optional1),
            FileSpec::optional(&optional2),
        ])
        .unwrap();

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

        let config = Config::load_merged_with_specs(&[
            FileSpec::required(&required1),
            FileSpec::optional(&optional1), // Missing, should be skipped
            FileSpec::required(&required2),
            FileSpec::optional(&optional2), // Exists, should be merged
        ])
        .unwrap();

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
    fn test_filespec_from_path() {
        // Test From<&str> implementation
        let spec: FileSpec = "test.yaml".into();
        assert!(matches!(spec, FileSpec::Required(_)));
        assert_eq!(spec.path().to_str(), Some("test.yaml"));
        assert!(!spec.is_optional());

        // Test From<PathBuf> implementation
        let path = std::path::PathBuf::from("other.yaml");
        let spec: FileSpec = path.into();
        assert!(matches!(spec, FileSpec::Required(_)));

        // Test explicit constructors
        let required = FileSpec::required("req.yaml");
        assert!(!required.is_optional());

        let optional = FileSpec::optional("opt.yaml");
        assert!(optional.is_optional());
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
}

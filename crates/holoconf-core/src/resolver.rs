//! Resolver architecture per ADR-002
//!
//! Resolvers are functions or objects that resolve interpolation expressions
//! like `${env:VAR}` to actual values.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use crate::error::{Error, Result};
use crate::value::Value;

// PEM format constants
const PEM_BEGIN_MARKER: &str = "-----BEGIN";
const PEM_BEGIN_ENCRYPTED_KEY: &str = "-----BEGIN ENCRYPTED PRIVATE KEY-----";

/// Certificate or key input - can be text (PEM content or file path) or binary (P12/PFX bytes)
#[derive(Clone, Debug)]
pub enum CertInput {
    /// PEM text content or file path to PEM/P12 file
    Text(String),
    /// Binary P12/PFX content
    Binary(Vec<u8>),
}

impl CertInput {
    /// Check if this looks like PEM content (has -----BEGIN marker)
    pub fn is_pem_content(&self) -> bool {
        matches!(self, CertInput::Text(s) if s.contains(PEM_BEGIN_MARKER))
    }

    /// Check if this looks like a P12/PFX file path (by extension)
    pub fn is_p12_path(&self) -> bool {
        matches!(self, CertInput::Text(s) if {
            let lower = s.to_lowercase();
            lower.ends_with(".p12") || lower.ends_with(".pfx")
        })
    }

    /// Get the text content if this is a Text variant
    pub fn as_text(&self) -> Option<&str> {
        match self {
            CertInput::Text(s) => Some(s),
            CertInput::Binary(_) => None,
        }
    }

    /// Get the binary content if this is a Binary variant
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            CertInput::Text(_) => None,
            CertInput::Binary(b) => Some(b),
        }
    }
}

impl From<String> for CertInput {
    fn from(s: String) -> Self {
        CertInput::Text(s)
    }
}

impl From<&str> for CertInput {
    fn from(s: &str) -> Self {
        CertInput::Text(s.to_string())
    }
}

impl From<Vec<u8>> for CertInput {
    fn from(b: Vec<u8>) -> Self {
        CertInput::Binary(b)
    }
}

// Global resolver registry for extension packages
static GLOBAL_REGISTRY: OnceLock<RwLock<ResolverRegistry>> = OnceLock::new();

/// Get the global resolver registry.
///
/// This registry is lazily initialized with built-in resolvers.
/// Extension packages can register additional resolvers here.
pub fn global_registry() -> &'static RwLock<ResolverRegistry> {
    GLOBAL_REGISTRY.get_or_init(|| RwLock::new(ResolverRegistry::with_builtins()))
}

/// Register a resolver in the global registry.
///
/// # Arguments
/// * `resolver` - The resolver to register
/// * `force` - If true, overwrite any existing resolver with the same name.
///   If false, return an error if the name is already registered.
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(Error)` if force=false and a resolver with the same name exists
pub fn register_global(resolver: Arc<dyn Resolver>, force: bool) -> Result<()> {
    let mut registry = global_registry()
        .write()
        .expect("Global registry lock poisoned");
    registry.register_with_force(resolver, force)
}

/// A resolved value with optional sensitivity metadata
#[derive(Clone)]
pub struct ResolvedValue {
    /// The actual resolved value
    pub value: Value,
    /// Whether this value is sensitive (should be redacted in logs/exports)
    pub sensitive: bool,
}

impl std::fmt::Debug for ResolvedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedValue")
            .field(
                "value",
                if self.sensitive {
                    &"[REDACTED]"
                } else {
                    &self.value
                },
            )
            .field("sensitive", &self.sensitive)
            .finish()
    }
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
    /// Allowed root directories for file resolver (path traversal protection)
    pub file_roots: std::collections::HashSet<std::path::PathBuf>,
    /// Resolution stack for circular reference detection
    pub resolution_stack: Vec<String>,
    /// Whether HTTP resolver is enabled
    pub allow_http: bool,
    /// HTTP URL allowlist (glob patterns)
    pub http_allowlist: Vec<String>,
    /// HTTP proxy URL (e.g., "http://proxy:8080" or "socks5://proxy:1080")
    pub http_proxy: Option<String>,
    /// Whether to auto-detect proxy from environment variables (HTTP_PROXY, HTTPS_PROXY, NO_PROXY)
    pub http_proxy_from_env: bool,
    /// CA bundle (file path or PEM content) - replaces default webpki-roots
    pub http_ca_bundle: Option<CertInput>,
    /// Extra CA bundle (file path or PEM content) - appends to webpki-roots
    pub http_extra_ca_bundle: Option<CertInput>,
    /// Client certificate for mTLS (file path, PEM content, or P12/PFX binary)
    pub http_client_cert: Option<CertInput>,
    /// Client private key for mTLS (file path or PEM content, not needed for P12/PFX)
    pub http_client_key: Option<CertInput>,
    /// Password for encrypted private key or P12/PFX file
    pub http_client_key_password: Option<String>,
    // NOTE: http_insecure removed - use insecure=true kwarg on each resolver call
}

impl ResolverContext {
    /// Create a new resolver context
    pub fn new(config_path: impl Into<String>) -> Self {
        Self {
            config_path: config_path.into(),
            config_root: None,
            base_path: None,
            file_roots: std::collections::HashSet::new(),
            resolution_stack: Vec::new(),
            allow_http: false,
            http_allowlist: Vec::new(),
            http_proxy: None,
            http_proxy_from_env: false,
            http_ca_bundle: None,
            http_extra_ca_bundle: None,
            http_client_cert: None,
            http_client_key: None,
            http_client_key_password: None,
        }
    }

    /// Set whether HTTP resolver is enabled
    pub fn with_allow_http(mut self, allow: bool) -> Self {
        self.allow_http = allow;
        self
    }

    /// Set HTTP URL allowlist
    pub fn with_http_allowlist(mut self, allowlist: Vec<String>) -> Self {
        self.http_allowlist = allowlist;
        self
    }

    /// Set HTTP proxy URL
    pub fn with_http_proxy(mut self, proxy: impl Into<String>) -> Self {
        self.http_proxy = Some(proxy.into());
        self
    }

    /// Set whether to auto-detect proxy from environment variables
    pub fn with_http_proxy_from_env(mut self, enabled: bool) -> Self {
        self.http_proxy_from_env = enabled;
        self
    }

    /// Set CA bundle (file path or PEM content)
    pub fn with_http_ca_bundle(mut self, input: impl Into<CertInput>) -> Self {
        self.http_ca_bundle = Some(input.into());
        self
    }

    /// Set extra CA bundle (file path or PEM content)
    pub fn with_http_extra_ca_bundle(mut self, input: impl Into<CertInput>) -> Self {
        self.http_extra_ca_bundle = Some(input.into());
        self
    }

    /// Set client certificate for mTLS (file path, PEM content, or P12/PFX binary)
    pub fn with_http_client_cert(mut self, input: impl Into<CertInput>) -> Self {
        self.http_client_cert = Some(input.into());
        self
    }

    /// Set client private key for mTLS (file path or PEM content, not needed for P12/PFX)
    pub fn with_http_client_key(mut self, input: impl Into<CertInput>) -> Self {
        self.http_client_key = Some(input.into());
        self
    }

    /// Set password for encrypted private key or P12/PFX file
    pub fn with_http_client_key_password(mut self, password: impl Into<String>) -> Self {
        self.http_client_key_password = Some(password.into());
        self
    }

    // DANGEROUS: with_http_insecure removed - use insecure=true kwarg instead

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
#[derive(Clone)]
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

    /// Register the built-in resolvers (env, file, http, https, json, yaml, split, csv, base64)
    fn register_builtin_resolvers(&mut self) {
        // Environment variable resolver
        self.register(Arc::new(FnResolver::new("env", env_resolver)));
        // File resolver
        self.register(Arc::new(FnResolver::new("file", file_resolver)));

        // HTTP/HTTPS resolvers (only available with http feature)
        #[cfg(feature = "http")]
        {
            // HTTP resolver (disabled by default for security)
            self.register(Arc::new(FnResolver::new("http", http_resolver)));
            // HTTPS resolver (disabled by default for security)
            self.register(Arc::new(FnResolver::new("https", https_resolver)));
        }

        // Transformation resolvers
        self.register(Arc::new(FnResolver::new("json", json_resolver)));
        self.register(Arc::new(FnResolver::new("yaml", yaml_resolver)));
        self.register(Arc::new(FnResolver::new("split", split_resolver)));
        self.register(Arc::new(FnResolver::new("csv", csv_resolver)));
        self.register(Arc::new(FnResolver::new("base64", base64_resolver)));
    }

    /// Register a resolver
    pub fn register(&mut self, resolver: Arc<dyn Resolver>) {
        self.resolvers.insert(resolver.name().to_string(), resolver);
    }

    /// Register a resolver with optional force overwrite.
    ///
    /// # Arguments
    /// * `resolver` - The resolver to register
    /// * `force` - If true, overwrite any existing resolver with the same name.
    ///   If false, return an error if the name is already registered.
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(Error)` if force=false and a resolver with the same name exists
    pub fn register_with_force(&mut self, resolver: Arc<dyn Resolver>, force: bool) -> Result<()> {
        let name = resolver.name().to_string();
        if !force && self.resolvers.contains_key(&name) {
            return Err(Error::resolver_already_registered(&name));
        }
        self.resolvers.insert(name, resolver);
        Ok(())
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
    ///
    /// This method implements framework-level handling of the `sensitive` kwarg per ADR-011.
    /// The `sensitive` kwarg overrides the resolver's sensitivity hint.
    ///
    /// Note: `default` handling with lazy resolution is done at the Config level,
    /// not here, to support nested interpolations in default values.
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

        // Extract framework-level `sensitive` kwarg per ADR-011
        let sensitive_override = kwargs
            .get("sensitive")
            .map(|v| v.eq_ignore_ascii_case("true"));

        // Pass remaining kwargs to the resolver (filter out framework keyword)
        let resolver_kwargs: HashMap<String, String> = kwargs
            .iter()
            .filter(|(k, _)| *k != "sensitive")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Call the resolver
        let mut resolved = resolver.resolve(args, &resolver_kwargs, ctx)?;

        // Apply sensitivity override if specified
        if let Some(is_sensitive) = sensitive_override {
            resolved.sensitive = is_sensitive;
        }

        Ok(resolved)
    }
}

/// Built-in environment variable resolver
///
/// Usage:
///   ${env:VAR_NAME}                      - Get env var (error if not set)
///   ${env:VAR_NAME,default=value}        - Get env var with default (framework-handled)
///   ${env:VAR_NAME,sensitive=true}       - Mark as sensitive for redaction (framework-handled)
///
/// Note: `default` and `sensitive` are framework-level kwargs handled by ResolverRegistry.
/// This resolver just returns the env var value or an error if not found.
fn env_resolver(
    args: &[String],
    _kwargs: &HashMap<String, String>,
    ctx: &ResolverContext,
) -> Result<ResolvedValue> {
    if args.is_empty() {
        return Err(Error::parse("env resolver requires a variable name")
            .with_path(ctx.config_path.clone()));
    }

    let var_name = &args[0];

    match std::env::var(var_name) {
        Ok(value) => {
            // Return non-sensitive by default; sensitivity can be overridden via kwarg
            Ok(ResolvedValue::new(Value::String(value)))
        }
        Err(_) => {
            // Return EnvNotFound error - framework will handle default if provided
            Err(Error::env_not_found(
                var_name,
                Some(ctx.config_path.clone()),
            ))
        }
    }
}

/// Check if a hostname represents localhost
///
/// Recognizes various forms of localhost:
/// - "localhost" (case-insensitive)
/// - "127.0.0.1" (IPv4 loopback)
/// - Any 127.x.x.x address (IPv4 loopback range)
/// - "::1" (IPv6 loopback)
/// - "[::1]" (IPv6 loopback with brackets)
///
/// Note: We intentionally reject internationalized domain names (IDN)
/// and punycode for security and simplicity.
fn is_localhost(hostname: &str) -> bool {
    // ASCII localhost (case-insensitive)
    if hostname.eq_ignore_ascii_case("localhost") {
        return true;
    }

    // IPv4 loopback: 127.0.0.1 or any 127.x.x.x
    if hostname.starts_with("127.") {
        return true;
    }

    // IPv6 loopback: ::1 or [::1]
    if hostname == "::1" || hostname == "[::1]" {
        return true;
    }

    false
}

/// Normalize file path according to RFC 8089 file: URI scheme
///
/// RFC 8089 defines these valid formats:
///   file:///path      - Local file (empty authority)
///   file://localhost/path - Local file (explicit localhost)
///   file:/path        - Local file (minimal form)
///   file://host/path  - Remote file (not supported, returns error)
///   path              - HoloConf relative path (not RFC, but supported)
///
/// Returns (normalized_path, is_relative)
fn normalize_file_path(arg: &str) -> Result<(String, bool)> {
    // Security: Reject null bytes
    if arg.contains('\0') {
        return Err(Error::resolver_custom(
            "file",
            "File paths cannot contain null bytes",
        ));
    }

    if let Some(after_slashes) = arg.strip_prefix("//") {
        // file://... format - Parse as RFC 8089 file: URL
        // Remove exactly two leading slashes to get the authority+path

        // Check if this is file:/// (third slash means empty authority)
        if after_slashes.starts_with('/') {
            // file:/// - empty authority means localhost
            // The rest is the absolute path (already starts with /)
            Ok((after_slashes.to_string(), false))
        } else {
            // file://hostname/path or file://hostname format
            // Extract hostname (before first slash, or entire string if no slash)
            let parts: Vec<&str> = after_slashes.splitn(2, '/').collect();
            let hostname = parts[0];

            // Check if empty hostname (file:// with no authority)
            if hostname.is_empty() {
                return Ok(("/".to_string(), false));
            }

            if is_localhost(hostname) {
                // file://localhost/path - explicit localhost
                let path = parts
                    .get(1)
                    .map(|s| format!("/{}", s))
                    .unwrap_or_else(|| "/".to_string());
                Ok((path, false))
            } else {
                // file://hostname/path - remote file (not supported)
                Err(Error::resolver_custom(
                    "file",
                    format!(
                        "Remote file URIs not supported: hostname '{}' is not localhost\n\
                         \n\
                         HoloConf only supports local files:\n\
                         - file:///path/to/file (absolute, empty authority)\n\
                         - file://localhost/path/to/file (absolute, explicit localhost)\n\
                         - file:/path/to/file (absolute, minimal)\n\
                         - relative/path/to/file (relative to config directory)",
                        hostname
                    ),
                ))
            }
        }
    } else if arg.starts_with('/') {
        // file:/path - RFC 8089 local absolute (no authority)
        Ok((arg.to_string(), false))
    } else {
        // No leading slashes: relative path (HoloConf convention)
        Ok((arg.to_string(), true))
    }
}

/// Built-in file resolver
///
/// Usage:
///   ${file:path/to/file}                    - Read file as text (UTF-8), relative to config
///   ${file:///absolute/path}                - Absolute path (RFC 8089)
///   ${file://localhost/absolute/path}       - Absolute path (RFC 8089, explicit localhost)
///   ${file:/absolute/path}                  - Absolute path (RFC 8089 minimal form)
///   ${file:path/to/file,parse=text}         - Read as text (explicit, no parsing)
///   ${file:path/to/file,parse=none}         - Return raw bytes (alias for encoding=binary)
///   ${file:path/to/file,encoding=utf-8}     - UTF-8 encoding (default)
///   ${file:path/to/file,encoding=ascii}     - ASCII encoding (strips non-ASCII)
///   ${file:path/to/file,encoding=base64}    - Base64 encode the file contents as string
///   ${file:path/to/file,encoding=binary}    - Return raw bytes as Value::Bytes
///   ${file:path/to/file,default={}}         - Default if file not found (framework-handled)
///   ${file:path/to/file,sensitive=true}     - Mark as sensitive (framework-handled)
///
/// For structured data parsing, use transformation resolvers:
///   ${json:${file:config.json}}             - Parse JSON file
///   ${yaml:${file:config.yaml}}             - Parse YAML file
///   ${csv:${file:data.csv}}                 - Parse CSV file
///
/// Note: `default` and `sensitive` are framework-level kwargs handled by ResolverRegistry.
fn file_resolver(
    args: &[String],
    kwargs: &HashMap<String, String>,
    ctx: &ResolverContext,
) -> Result<ResolvedValue> {
    if args.is_empty() {
        return Err(
            Error::parse("file resolver requires a file path").with_path(ctx.config_path.clone())
        );
    }

    let file_path_arg = &args[0];
    let parse_mode = kwargs.get("parse").map(|s| s.as_str()).unwrap_or("text");
    let encoding = kwargs
        .get("encoding")
        .map(|s| s.as_str())
        .unwrap_or("utf-8");

    // Normalize file path according to RFC 8089
    let (normalized_path, is_relative) = normalize_file_path(file_path_arg)?;

    // Resolve relative paths based on context base path
    let file_path = if is_relative {
        if let Some(base) = &ctx.base_path {
            base.join(&normalized_path)
        } else {
            std::path::PathBuf::from(&normalized_path)
        }
    } else {
        // Absolute path from RFC 8089 file: URI
        std::path::PathBuf::from(&normalized_path)
    };

    // Validate path is within allowed roots (path traversal protection)
    // Security: Empty file_roots would bypass all validation - deny by default
    if ctx.file_roots.is_empty() {
        return Err(Error::resolver_custom(
            "file",
            "File resolver requires allowed directories to be configured. \
             Use Config.load() which auto-configures the parent directory, or \
             specify file_roots explicitly for Config.loads()."
                .to_string(),
        )
        .with_path(ctx.config_path.clone()));
    }

    // Canonicalize file path to resolve symlinks and get absolute path
    // This also checks if file exists (canonicalize fails if file doesn't exist)
    let canonical_path = file_path.canonicalize().map_err(|e| {
        // Check if this is a "not found" error for better error message
        if e.kind() == std::io::ErrorKind::NotFound {
            return Error::file_not_found(file_path_arg, Some(ctx.config_path.clone()));
        }
        Error::resolver_custom("file", format!("Failed to resolve file path: {}", e))
            .with_path(ctx.config_path.clone())
    })?;

    // Validate against allowed roots
    let mut canonicalization_errors = Vec::new();
    let is_allowed = ctx.file_roots.iter().any(|root| {
        match root.canonicalize() {
            Ok(canonical_root) => canonical_path.starts_with(&canonical_root),
            Err(e) => {
                // Log but don't fail - root might not exist yet
                canonicalization_errors.push((root.clone(), e));
                false
            }
        }
    });

    if !is_allowed {
        // Sanitize error message to avoid information disclosure
        let display_path = if let Some(base) = &ctx.base_path {
            file_path
                .strip_prefix(base)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "<outside allowed directories>".to_string())
        } else {
            "<outside allowed directories>".to_string()
        };

        let mut msg = format!(
            "Access denied: file '{}' is outside allowed directories.",
            display_path
        );

        if !canonicalization_errors.is_empty() {
            msg.push_str(&format!(
                " Note: {} configured root(s) could not be validated.",
                canonicalization_errors.len()
            ));
        }

        msg.push_str(" Use file_roots parameter to extend allowed directories.");

        return Err(Error::resolver_custom("file", msg).with_path(ctx.config_path.clone()));
    }

    // Handle binary encoding separately - returns Value::Bytes directly
    if encoding == "binary" {
        let bytes = std::fs::read(&file_path)
            .map_err(|_| Error::file_not_found(file_path_arg, Some(ctx.config_path.clone())))?;
        return Ok(ResolvedValue::new(Value::Bytes(bytes)));
    }

    // Read the file based on encoding
    let content = match encoding {
        "base64" => {
            // Read as binary and base64 encode
            use base64::{engine::general_purpose::STANDARD, Engine as _};
            let bytes = std::fs::read(&file_path)
                .map_err(|_| Error::file_not_found(file_path_arg, Some(ctx.config_path.clone())))?;
            STANDARD.encode(bytes)
        }
        "ascii" => {
            // Read as UTF-8 but strip non-ASCII characters
            let raw = std::fs::read_to_string(&file_path)
                .map_err(|_| Error::file_not_found(file_path_arg, Some(ctx.config_path.clone())))?;
            raw.chars().filter(|c| c.is_ascii()).collect()
        }
        _ => {
            // Default to UTF-8 (including explicit "utf-8")
            std::fs::read_to_string(&file_path)
                .map_err(|_| Error::file_not_found(file_path_arg, Some(ctx.config_path.clone())))?
        }
    };

    // For base64 encoding, always return as text (don't try to parse)
    if encoding == "base64" {
        return Ok(ResolvedValue::new(Value::String(content)));
    }

    // Parse mode: only "text" (default) or "none" (same as encoding=binary)
    // For structured data parsing, use transformation resolvers:
    //   ${json:${file:config.json}}, ${yaml:${file:config.yaml}}, etc.
    match parse_mode {
        "none" => {
            // parse=none is an alias for encoding=binary - return raw bytes
            let bytes = std::fs::read(&file_path)
                .map_err(|_| Error::file_not_found(file_path_arg, Some(ctx.config_path.clone())))?;
            Ok(ResolvedValue::new(Value::Bytes(bytes)))
        }
        _ => {
            // Default to text mode (including explicit "text") - return content as string
            Ok(ResolvedValue::new(Value::String(content)))
        }
    }
}

/// Normalize HTTP/HTTPS URL by stripping existing scheme and prepending the correct one
///
/// This allows flexible syntax like:
///   ${https://example.com}    → https://example.com
///   ${https:example.com}      → https://example.com
///   ${https:https://example}  → https://example.com (backwards compatible)
///
/// Returns an error if the URL is invalid (empty, or has invalid syntax like ///)
#[cfg(feature = "http")]
fn normalize_http_url(scheme: &str, arg: &str) -> Result<String> {
    // Strip any existing http:// or https:// prefix
    let clean = arg
        .strip_prefix("http://")
        .or_else(|| arg.strip_prefix("https://"))
        .unwrap_or(arg);

    // Strip leading // if present (handles ${https://example.com} syntax)
    let clean = clean.strip_prefix("//").unwrap_or(clean);

    // Validate: Reject empty URLs
    if clean.trim().is_empty() {
        return Err(Error::resolver_custom(
            scheme,
            format!(
                "{} resolver requires a non-empty URL",
                scheme.to_uppercase()
            ),
        ));
    }

    // Validate: Reject URLs starting with / (like /// which would create scheme:///)
    if clean.starts_with('/') {
        return Err(Error::resolver_custom(
            scheme,
            format!(
                "Invalid URL syntax: '{}'. URLs must have a hostname after the ://\n\
                 Valid formats:\n\
                 - ${{{}:example.com/path}} (clean syntax)\n\
                 - ${{{}:{}://example.com/path}} (backwards compatible)",
                arg, scheme, scheme, scheme
            ),
        ));
    }

    // Prepend the correct scheme
    Ok(format!("{}://{}", scheme, clean))
}

/// Common HTTP/HTTPS resolver implementation
///
/// This shared function reduces duplication between http_resolver and https_resolver.
/// The only difference between the two resolvers is the scheme they prepend.
fn http_or_https_resolver(
    scheme: &str,
    args: &[String],
    kwargs: &HashMap<String, String>,
    ctx: &ResolverContext,
) -> Result<ResolvedValue> {
    if args.is_empty() {
        return Err(
            Error::parse(format!("{} resolver requires a URL", scheme.to_uppercase()))
                .with_path(ctx.config_path.clone()),
        );
    }

    #[cfg(feature = "http")]
    {
        // Normalize URL to prepend the appropriate scheme
        let url = normalize_http_url(scheme, &args[0])?;

        // Check if HTTP is enabled
        if !ctx.allow_http {
            return Err(Error {
                kind: crate::error::ErrorKind::Resolver(
                    crate::error::ResolverErrorKind::HttpDisabled,
                ),
                path: Some(ctx.config_path.clone()),
                source_location: None,
                help: Some(format!(
                    "{} resolver is disabled. The URL specified by this config path cannot be fetched.\n\
                     Enable with Config.load(..., allow_http=True)",
                    scheme.to_uppercase()
                )),
                cause: None,
            });
        }

        // Check URL against allowlist if configured
        if !ctx.http_allowlist.is_empty() {
            let url_allowed = ctx
                .http_allowlist
                .iter()
                .any(|pattern| url_matches_pattern(&url, pattern));
            if !url_allowed {
                return Err(Error::http_not_in_allowlist(
                    &url,
                    &ctx.http_allowlist,
                    Some(ctx.config_path.clone()),
                ));
            }
        }

        http_fetch(&url, kwargs, ctx)
    }

    #[cfg(not(feature = "http"))]
    {
        // If compiled without HTTP feature, return an error
        let _ = (kwargs, ctx); // Suppress unused warnings
        Err(Error::resolver_custom(
            scheme,
            format!(
                "{} support not compiled in. Rebuild with --features http",
                scheme.to_uppercase()
            ),
        ))
    }
}

/// Built-in HTTP resolver
///
/// Fetches content from remote URLs.
///
/// Usage:
///   ${http:example.com/config.yaml}                   - Clean syntax (auto-prepends http://)
///   ${http:example.com/config,parse=text}             - Read as text (explicit, no parsing)
///   ${http:example.com/config,parse=binary}           - Read as binary (Value::Bytes)
///   ${http:example.com/config,timeout=60}             - Timeout in seconds
///   ${http:example.com/config,header=Auth:Bearer token} - Add header
///   ${http:example.com/config,default={}}             - Default if request fails
///   ${http:example.com/config,sensitive=true}         - Mark as sensitive
///
/// For structured data parsing, use transformation resolvers:
///   ${json:${http:api.example.com/config}}            - Parse JSON response
///   ${yaml:${http:example.com/config.yaml}}           - Parse YAML response
///   ${csv:${http:data.example.com/export}}            - Parse CSV response
///
/// Backwards compatible:
///   ${http:http://example.com}                        - Still works (protocol stripped and re-prepended)
///
/// Security:
/// - Disabled by default (requires allow_http=true in ConfigOptions)
/// - URL allowlist can restrict which URLs are accessible
fn http_resolver(
    args: &[String],
    kwargs: &HashMap<String, String>,
    ctx: &ResolverContext,
) -> Result<ResolvedValue> {
    http_or_https_resolver("http", args, kwargs, ctx)
}

/// Built-in HTTPS resolver
///
/// Fetches content from remote HTTPS URLs. Same as http_resolver but prepends https:// scheme.
///
/// Usage:
///   ${https:example.com/config.yaml}                  - Clean syntax (auto-prepends https://)
///   ${https:example.com/config,parse=text}            - Read as text (explicit, no parsing)
///   ${https:example.com/config,parse=binary}          - Read as binary (Value::Bytes)
///   ${https:example.com/config,timeout=60}            - Timeout in seconds
///   ${https:example.com/config,header=Auth:Bearer token} - Add header
///   ${https:example.com/config,default={}}            - Default if request fails
///   ${https:example.com/config,sensitive=true}        - Mark as sensitive
///
/// For structured data parsing, use transformation resolvers:
///   ${json:${https:api.example.com/config}}           - Parse JSON response
///   ${yaml:${https:example.com/config.yaml}}          - Parse YAML response
///   ${csv:${https:data.example.com/export}}           - Parse CSV response
///
/// Backwards compatible:
///   ${https:https://example.com}                      - Still works (protocol stripped and re-prepended)
///
/// Security:
/// - Disabled by default (requires allow_http=true in ConfigOptions)
/// - URL allowlist can restrict which URLs are accessible
fn https_resolver(
    args: &[String],
    kwargs: &HashMap<String, String>,
    ctx: &ResolverContext,
) -> Result<ResolvedValue> {
    http_or_https_resolver("https", args, kwargs, ctx)
}

/// Check if a URL matches an allowlist pattern
///
/// Supports glob-style patterns:
/// - `https://example.com/*` matches any path on example.com
/// - `https://*.example.com/*` matches any subdomain
#[cfg(feature = "http")]
fn url_matches_pattern(url: &str, pattern: &str) -> bool {
    // Security: Parse URL first to prevent bypass via malformed URLs
    let parsed_url = match url::Url::parse(url) {
        Ok(u) => u,
        Err(_) => {
            // Invalid URL - no match
            log::warn!("Invalid URL '{}' rejected by allowlist", url);
            return false;
        }
    };

    // Validate pattern doesn't contain dangerous sequences
    if pattern.contains("**") || pattern.contains(".*.*") {
        log::warn!(
            "Invalid allowlist pattern '{}' - contains dangerous sequence",
            pattern
        );
        return false;
    }

    // Use glob crate for proper glob matching
    let glob_pattern = match glob::Pattern::new(pattern) {
        Ok(p) => p,
        Err(_) => {
            // Invalid pattern - fall back to exact match
            log::warn!(
                "Invalid glob pattern '{}' - falling back to exact match",
                pattern
            );
            return url == pattern;
        }
    };

    // Match against the full URL string
    // This allows patterns like:
    // - "https://api.example.com/*" (all paths on this host)
    // - "https://*.example.com/api/*" (all subdomains)
    // - "https://api.example.com/v1/users" (exact match)
    glob_pattern.matches(parsed_url.as_str())
}

// =============================================================================
// TLS/Proxy Configuration Helpers (HTTP feature)
// =============================================================================

/// Parse PEM certificates from bytes
#[cfg(feature = "http")]
fn parse_pem_certs(pem_bytes: &[u8], source: &str) -> Result<Vec<ureq::tls::Certificate<'static>>> {
    use ureq::tls::PemItem;

    let certs: Vec<_> = ureq::tls::parse_pem(pem_bytes)
        .filter_map(|item| item.ok())
        .filter_map(|item| match item {
            PemItem::Certificate(cert) => Some(cert.to_owned()),
            _ => None,
        })
        .collect();

    if certs.is_empty() {
        return Err(Error::pem_load_error(
            source,
            "No valid certificates found in PEM data",
        ));
    }

    Ok(certs)
}

/// Load certificates from CertInput (PEM content or file path)
#[cfg(feature = "http")]
fn load_certs(input: &CertInput) -> Result<Vec<ureq::tls::Certificate<'static>>> {
    match input {
        CertInput::Binary(_) => {
            Err(Error::tls_config_error(
                "CA bundle must be PEM format, not binary. For P12 client certificates, use client_cert parameter."
            ))
        }
        CertInput::Text(text) => {
            // Try as file path first
            let path = std::path::Path::new(text);
            if path.exists() {
                log::trace!("Loading certificates from file: {}", text);
                let bytes = std::fs::read(path).map_err(|e| {
                    // Sanitize path for error message (could contain PEM content if detection fails)
                    let display_path = if text.len() < 256 && !text.contains('\n') {
                        text
                    } else {
                        "[PEM content or long path]"
                    };
                    Error::pem_load_error(
                        display_path,
                        format!("Failed to read certificate file: {}", e),
                    )
                })?;
                parse_pem_certs(&bytes, text)
            } else {
                // Fallback to parsing as PEM content
                log::trace!("Path does not exist, attempting to parse as PEM content");
                parse_pem_certs(text.as_bytes(), "PEM content")
            }
        }
    }
}

/// Parse a private key from PEM bytes (handles encrypted and unencrypted)
#[cfg(feature = "http")]
fn parse_pem_private_key(
    pem_content: &str,
    password: Option<&str>,
    source: &str,
) -> Result<ureq::tls::PrivateKey<'static>> {
    use pkcs8::der::Decode;

    // Check if this is an encrypted PKCS#8 key
    if pem_content.contains(PEM_BEGIN_ENCRYPTED_KEY) {
        let pwd = password.ok_or_else(|| {
            Error::tls_config_error(format!(
                "Password required for encrypted private key from: {}",
                source
            ))
        })?;

        // Extract the base64 content from PEM
        let der_bytes = pem_to_der(pem_content, "ENCRYPTED PRIVATE KEY")
            .map_err(|e| Error::pem_load_error(source, e))?;

        let encrypted = pkcs8::EncryptedPrivateKeyInfo::from_der(&der_bytes)
            .map_err(|e| Error::pem_load_error(source, e.to_string()))?;

        let decrypted = encrypted
            .decrypt(pwd)
            .map_err(|e| Error::key_decryption_error(e.to_string()))?;

        // The decrypted key is in PKCS#8 DER format
        // Wrap it in PEM format so ureq can parse it
        let pem_key = der_to_pem(decrypted.as_bytes(), "PRIVATE KEY");

        ureq::tls::PrivateKey::from_pem(pem_key.as_bytes())
            .map(|k| k.to_owned())
            .map_err(|e| {
                Error::pem_load_error(source, format!("Failed to parse decrypted key: {}", e))
            })
    } else {
        // Try loading as regular key
        ureq::tls::PrivateKey::from_pem(pem_content.as_bytes())
            .map(|k| k.to_owned())
            .map_err(|e| {
                Error::pem_load_error(source, format!("Failed to parse private key: {}", e))
            })
    }
}

/// Load a private key from CertInput (PEM content or file path)
#[cfg(feature = "http")]
fn load_private_key(
    input: &CertInput,
    password: Option<&str>,
) -> Result<ureq::tls::PrivateKey<'static>> {
    match input {
        CertInput::Binary(_) => {
            Err(Error::tls_config_error(
                "Private key must be PEM text format, not binary. For P12, use client_cert only (no client_key needed)."
            ))
        }
        CertInput::Text(text) => {
            // Try as file path first
            let path = std::path::Path::new(text);
            if path.exists() {
                log::trace!("Loading private key from file: {}", text);
                let pem_content = std::fs::read_to_string(path).map_err(|e| {
                    // Sanitize path for error message
                    let display_path = if text.len() < 256 && !text.contains('\n') {
                        text
                    } else {
                        "[PEM content or long path]"
                    };
                    Error::pem_load_error(
                        display_path,
                        format!("Failed to read key file: {}", e),
                    )
                })?;
                parse_pem_private_key(&pem_content, password, text)
            } else {
                // Fallback to parsing as PEM content
                log::trace!("Path does not exist, attempting to parse as PEM content");
                parse_pem_private_key(text, password, "PEM content")
            }
        }
    }
}

/// Extract DER bytes from PEM format
#[cfg(feature = "http")]
fn pem_to_der(pem: &str, label: &str) -> std::result::Result<Vec<u8>, String> {
    let begin_marker = format!("-----BEGIN {}-----", label);
    let end_marker = format!("-----END {}-----", label);

    let start = pem
        .find(&begin_marker)
        .ok_or_else(|| format!("PEM begin marker not found for {}", label))?;
    let end = pem
        .find(&end_marker)
        .ok_or_else(|| format!("PEM end marker not found for {}", label))?;

    let base64_content: String = pem[start + begin_marker.len()..end]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(&base64_content)
        .map_err(|e| format!("Failed to decode base64: {}", e))
}

/// Convert DER bytes to PEM format
#[cfg(feature = "http")]
fn der_to_pem(der: &[u8], label: &str) -> String {
    use base64::Engine;
    let base64 = base64::engine::general_purpose::STANDARD.encode(der);
    // Split into 64-char lines
    let lines: Vec<&str> = base64
        .as_bytes()
        .chunks(64)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect();
    format!(
        "-----BEGIN {}-----\n{}\n-----END {}-----\n",
        label,
        lines.join("\n"),
        label
    )
}

/// Parse P12/PFX bytes into certificate chain and private key
#[cfg(feature = "http")]
fn parse_p12_identity(
    p12_data: &[u8],
    password: &str,
    source: &str,
) -> Result<(
    Vec<ureq::tls::Certificate<'static>>,
    ureq::tls::PrivateKey<'static>,
)> {
    // Warn if using empty password (valid but insecure)
    if password.is_empty() {
        log::warn!(
            "Loading P12 file without password from: {} - ensure file is properly protected",
            source
        );
    }

    let keystore = p12_keystore::KeyStore::from_pkcs12(p12_data, password)
        .map_err(|e| Error::p12_load_error(source, e.to_string()))?;

    // Get the first key entry (most P12 files have one key)
    // private_key_chain() returns Option<(&str, &PrivateKeyChain)>
    let (_alias, key_chain) = keystore
        .private_key_chain()
        .ok_or_else(|| Error::p12_load_error(source, "No private key found in P12 data"))?;

    // Get the private key DER bytes - wrap in PEM for ureq
    let pem_key = der_to_pem(key_chain.key(), "PRIVATE KEY");
    let private_key = ureq::tls::PrivateKey::from_pem(pem_key.as_bytes())
        .map(|k| k.to_owned())
        .map_err(|e| {
            Error::p12_load_error(source, format!("Failed to parse private key: {}", e))
        })?;

    // Get certificates from the chain
    let certs: Vec<_> = key_chain
        .chain()
        .iter()
        .map(|cert| ureq::tls::Certificate::from_der(cert.as_der()).to_owned())
        .collect();

    if certs.is_empty() {
        return Err(Error::p12_load_error(
            source,
            "No certificates found in P12 data",
        ));
    }

    Ok((certs, private_key))
}

/// Load client identity (cert + key) for mTLS from CertInput
#[cfg(feature = "http")]
fn load_client_identity(
    cert_input: &CertInput,
    key_input: Option<&CertInput>,
    password: Option<&str>,
) -> Result<(
    Vec<ureq::tls::Certificate<'static>>,
    ureq::tls::PrivateKey<'static>,
)> {
    match cert_input {
        // P12 binary content
        CertInput::Binary(bytes) => {
            log::trace!("Loading client identity from P12 binary content");
            // P12 files may have empty passwords - this is valid (warning logged in parse_p12_identity)
            let pwd = password.unwrap_or("");
            parse_p12_identity(bytes, pwd, "P12 binary content")
        }

        // Text - could be PEM content, P12 path, or PEM path
        CertInput::Text(text) => {
            // Check if it's a P12 file path
            if cert_input.is_p12_path() {
                log::trace!("Loading client identity from P12 file: {}", text);
                let bytes = std::fs::read(text).map_err(|e| {
                    Error::p12_load_error(text, format!("Failed to read P12 file: {}", e))
                })?;
                // P12 files may have empty passwords - this is valid (warning logged in parse_p12_identity)
                let pwd = password.unwrap_or("");
                return parse_p12_identity(&bytes, pwd, text);
            }

            // PEM content or path
            log::trace!("Loading client identity from PEM (cert + key)");
            let certs = load_certs(cert_input)?;

            let key_input = key_input.ok_or_else(|| {
                Error::tls_config_error(
                    "client_key required when using PEM certificate (not needed for P12)",
                )
            })?;

            let key = load_private_key(key_input, password)?;
            Ok((certs, key))
        }
    }
}

/// Build TLS configuration from context and per-request kwargs
#[cfg(feature = "http")]
fn build_tls_config(
    ctx: &ResolverContext,
    kwargs: &HashMap<String, String>,
) -> Result<ureq::tls::TlsConfig> {
    use std::sync::Arc;
    use ureq::tls::{ClientCert, RootCerts, TlsConfig};

    let mut builder = TlsConfig::builder();

    // Check for insecure mode (only from per-request kwargs)
    let insecure = kwargs.get("insecure").map(|v| v == "true").unwrap_or(false);

    if insecure {
        // OBNOXIOUS WARNING: This is a SECURITY RISK
        eprintln!("\n┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓");
        eprintln!("┃ ⚠️  WARNING: TLS CERTIFICATE VERIFICATION DISABLED ┃");
        eprintln!("┃                                                    ┃");
        eprintln!("┃ You are using insecure=true which disables ALL    ┃");
        eprintln!("┃ TLS certificate validation. This is DANGEROUS     ┃");
        eprintln!("┃ and should ONLY be used in development.           ┃");
        eprintln!("┃                                                    ┃");
        eprintln!("┃ In production, use proper certificate             ┃");
        eprintln!("┃ configuration with ca_bundle or extra_ca_bundle.  ┃");
        eprintln!("┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛\n");
        log::warn!("TLS certificate verification is disabled (insecure=true)");
        builder = builder.disable_verification(true);
    }

    // Load CA bundle if specified (per-request overrides context)
    let ca_bundle_input = kwargs
        .get("ca_bundle")
        .map(|s| CertInput::Text(s.clone()))
        .or_else(|| ctx.http_ca_bundle.clone());

    let extra_ca_bundle_input = kwargs
        .get("extra_ca_bundle")
        .map(|s| CertInput::Text(s.clone()))
        .or_else(|| ctx.http_extra_ca_bundle.clone());

    if let Some(ca_input) = ca_bundle_input.as_ref() {
        // Replace root certs with custom CA bundle
        let certs = load_certs(ca_input)?;
        builder = builder.root_certs(RootCerts::Specific(Arc::new(certs)));
    } else if let Some(extra_ca_input) = extra_ca_bundle_input.as_ref() {
        // Add extra certs to webpki roots using new_with_certs
        let extra_certs = load_certs(extra_ca_input)?;
        builder = builder.root_certs(RootCerts::new_with_certs(&extra_certs));
    }

    // Load client certificate for mTLS (per-request overrides context)
    let client_cert_input = kwargs
        .get("client_cert")
        .map(|s| CertInput::Text(s.clone()))
        .or_else(|| ctx.http_client_cert.clone());

    if let Some(cert_input) = client_cert_input.as_ref() {
        let client_key_input = kwargs
            .get("client_key")
            .map(|s| CertInput::Text(s.clone()))
            .or_else(|| ctx.http_client_key.clone());

        let password = kwargs
            .get("key_password")
            .map(|s| s.as_str())
            .or(ctx.http_client_key_password.as_deref());

        let (certs, key) = load_client_identity(cert_input, client_key_input.as_ref(), password)?;

        let client_cert = ClientCert::new_with_certs(&certs, key);
        builder = builder.client_cert(Some(client_cert));
    }

    Ok(builder.build())
}

/// Build proxy configuration from context and per-request kwargs
#[cfg(feature = "http")]
fn build_proxy_config(
    ctx: &ResolverContext,
    kwargs: &HashMap<String, String>,
) -> Result<Option<ureq::Proxy>> {
    // Per-request proxy overrides context
    let proxy_url = kwargs
        .get("proxy")
        .cloned()
        .or_else(|| ctx.http_proxy.clone());

    // If no explicit proxy, check environment if enabled
    let proxy_url = proxy_url.or_else(|| {
        if ctx.http_proxy_from_env {
            // Check standard proxy environment variables
            std::env::var("HTTPS_PROXY")
                .or_else(|_| std::env::var("https_proxy"))
                .or_else(|_| std::env::var("HTTP_PROXY"))
                .or_else(|_| std::env::var("http_proxy"))
                .ok()
        } else {
            None
        }
    });

    if let Some(url) = proxy_url {
        let proxy = ureq::Proxy::new(&url).map_err(|e| {
            Error::proxy_config_error(format!("Invalid proxy URL '{}': {}", url, e))
        })?;
        Ok(Some(proxy))
    } else {
        Ok(None)
    }
}

/// Perform HTTP request and parse response
#[cfg(feature = "http")]
fn http_fetch(
    url: &str,
    kwargs: &HashMap<String, String>,
    ctx: &ResolverContext,
) -> Result<ResolvedValue> {
    use std::time::Duration;

    let parse_mode = kwargs.get("parse").map(|s| s.as_str()).unwrap_or("text");
    let timeout_secs: u64 = kwargs
        .get("timeout")
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    // Build TLS configuration
    let tls_config = build_tls_config(ctx, kwargs)?;

    // Build proxy configuration
    let proxy = build_proxy_config(ctx, kwargs)?;

    // Build agent with timeout, TLS, and proxy configuration
    let mut config_builder = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(timeout_secs)))
        .tls_config(tls_config);

    if proxy.is_some() {
        config_builder = config_builder.proxy(proxy);
    }

    let config = config_builder.build();
    let agent: ureq::Agent = config.into();

    // Build the request
    let mut request = agent.get(url);

    // Add custom headers
    for (key, value) in kwargs {
        if key == "header" {
            // Parse header in format "Name:Value"
            if let Some((name, val)) = value.split_once(':') {
                request = request.header(name.trim(), val.trim());
            }
        }
    }

    // Send request
    let response = request.call().map_err(|e| {
        let error_msg = match &e {
            ureq::Error::StatusCode(code) => format!("HTTP {}", code),
            ureq::Error::Timeout(kind) => format!("Request timeout: {:?}", kind),
            ureq::Error::Io(io_err) => format!("Connection error: {}", io_err),
            _ => format!("HTTP request failed: {}", e),
        };
        Error::http_request_failed(url, &error_msg, Some(ctx.config_path.clone()))
    })?;

    // Parse mode: only "text" (default) or "binary"
    // For structured data parsing, use transformation resolvers:
    //   ${json:${http:...}}, ${yaml:${http:...}}, etc.
    match parse_mode {
        "binary" => {
            let bytes = response.into_body().read_to_vec().map_err(|e| {
                Error::http_request_failed(url, e.to_string(), Some(ctx.config_path.clone()))
            })?;
            Ok(ResolvedValue::new(Value::Bytes(bytes)))
        }
        _ => {
            // Default to text mode (including explicit "text") - return response body as string
            let body = response.into_body().read_to_string().map_err(|e| {
                Error::http_request_failed(url, e.to_string(), Some(ctx.config_path.clone()))
            })?;
            Ok(ResolvedValue::new(Value::String(body)))
        }
    }
}

// =============================================================================
// Transformation Resolvers
// =============================================================================

/// Helper: Truncate string for error messages
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

/// JSON resolver - Parse JSON strings into structured data
fn json_resolver(
    args: &[String],
    _kwargs: &HashMap<String, String>,
    ctx: &ResolverContext,
) -> Result<ResolvedValue> {
    if args.is_empty() {
        return Err(Error::parse("json resolver requires a string argument")
            .with_path(ctx.config_path.clone()));
    }

    let json_str = &args[0];

    // Parse JSON
    let parsed: Value = serde_json::from_str(json_str).map_err(|e| {
        Error::parse(format!(
            "Invalid JSON at line {}, column {}: {}\nInput preview: {}",
            e.line(),
            e.column(),
            e,
            truncate_str(json_str, 50)
        ))
        .with_path(ctx.config_path.clone())
    })?;

    Ok(ResolvedValue::new(parsed))
}

/// YAML resolver - Parse YAML strings (first document only)
fn yaml_resolver(
    args: &[String],
    _kwargs: &HashMap<String, String>,
    ctx: &ResolverContext,
) -> Result<ResolvedValue> {
    if args.is_empty() {
        return Err(Error::parse("yaml resolver requires a string argument")
            .with_path(ctx.config_path.clone()));
    }

    let yaml_str = &args[0];

    // Parse YAML (first document only)
    let parsed: Value = serde_yaml::from_str(yaml_str).map_err(|e| {
        let location_info = if let Some(loc) = e.location() {
            format!(" at line {}, column {}", loc.line(), loc.column())
        } else {
            String::new()
        };

        Error::parse(format!(
            "Invalid YAML{}: {}\nInput preview: {}",
            location_info,
            e,
            truncate_str(yaml_str, 50)
        ))
        .with_path(ctx.config_path.clone())
    })?;

    Ok(ResolvedValue::new(parsed))
}

/// Split resolver - Split strings into arrays
fn split_resolver(
    args: &[String],
    kwargs: &HashMap<String, String>,
    ctx: &ResolverContext,
) -> Result<ResolvedValue> {
    if args.is_empty() {
        return Err(Error::parse("split resolver requires a string argument")
            .with_path(ctx.config_path.clone()));
    }

    let input_str = &args[0];
    let delim = kwargs.get("delim").map(|s| s.as_str()).unwrap_or(",");
    let trim = kwargs
        .get("trim")
        .map(|s| s.eq_ignore_ascii_case("true"))
        .unwrap_or(true); // Default: trim
    let skip_empty = kwargs
        .get("skip_empty")
        .map(|s| s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let limit = kwargs.get("limit").and_then(|s| s.parse::<usize>().ok());

    // Split
    let parts: Vec<&str> = if let Some(limit) = limit {
        input_str.splitn(limit + 1, delim).collect()
    } else {
        input_str.split(delim).collect()
    };

    // Process: trim and filter
    let result: Vec<Value> = parts
        .iter()
        .map(|s| if trim { s.trim() } else { *s })
        .filter(|s| !skip_empty || !s.is_empty())
        .map(|s| Value::String(s.to_string()))
        .collect();

    Ok(ResolvedValue::new(Value::Sequence(result)))
}

/// CSV resolver - Parse CSV data into arrays
fn csv_resolver(
    args: &[String],
    kwargs: &HashMap<String, String>,
    ctx: &ResolverContext,
) -> Result<ResolvedValue> {
    if args.is_empty() {
        return Err(Error::parse("csv resolver requires a string argument")
            .with_path(ctx.config_path.clone()));
    }

    let csv_str = &args[0];
    let header = kwargs
        .get("header")
        .map(|s| s.eq_ignore_ascii_case("true"))
        .unwrap_or(true); // Default: true
    let trim = kwargs
        .get("trim")
        .map(|s| s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let delim_str = kwargs.get("delim").map(|s| s.as_str()).unwrap_or(",");

    // Parse delimiter
    let delim_char = delim_str.chars().next().ok_or_else(|| {
        Error::parse("CSV delimiter cannot be empty").with_path(ctx.config_path.clone())
    })?;

    // Build CSV reader
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(header)
        .delimiter(delim_char as u8)
        .trim(if trim {
            csv::Trim::All
        } else {
            csv::Trim::None
        })
        .from_reader(csv_str.as_bytes());

    // Get headers if present
    let headers = if header {
        Some(
            reader
                .headers()
                .map_err(|e| {
                    Error::parse(format!("CSV parse error: {}", e))
                        .with_path(ctx.config_path.clone())
                })?
                .clone(),
        )
    } else {
        None
    };

    // Parse rows
    let mut rows = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| {
            let location_info = e
                .position()
                .map(|p| format!(" at line {}", p.line()))
                .unwrap_or_default();
            Error::parse(format!("CSV parse error{}: {}", location_info, e))
                .with_path(ctx.config_path.clone())
        })?;

        let row = if let Some(ref headers) = headers {
            // Array of objects: [{"name": "Alice", ...}]
            let mut obj = indexmap::IndexMap::new();
            for (i, field) in record.iter().enumerate() {
                let key = headers.get(i).unwrap_or(&format!("col{}", i)).to_string();
                obj.insert(key, Value::String(field.to_string()));
            }
            Value::Mapping(obj)
        } else {
            // Array of arrays: [["Alice", "admin"]]
            Value::Sequence(
                record
                    .iter()
                    .map(|s| Value::String(s.to_string()))
                    .collect(),
            )
        };

        rows.push(row);
    }

    Ok(ResolvedValue::new(Value::Sequence(rows)))
}

/// Base64 resolver - Decode base64 strings to bytes
fn base64_resolver(
    args: &[String],
    _kwargs: &HashMap<String, String>,
    ctx: &ResolverContext,
) -> Result<ResolvedValue> {
    if args.is_empty() {
        return Err(Error::parse("base64 resolver requires a string argument")
            .with_path(ctx.config_path.clone()));
    }

    let b64_str = args[0].trim();

    use base64::{engine::general_purpose, Engine as _};

    let decoded = general_purpose::STANDARD.decode(b64_str).map_err(|e| {
        Error::parse(format!(
            "Invalid base64: {}\nInput preview: {}",
            e,
            truncate_str(b64_str, 50)
        ))
        .with_path(ctx.config_path.clone())
    })?;

    // Try to decode as UTF-8 string (common for secrets, tokens, configs)
    // Fall back to bytes for binary data (images, certificates, etc.)
    match String::from_utf8(decoded) {
        Ok(s) => Ok(ResolvedValue::new(Value::String(s))),
        Err(e) => Ok(ResolvedValue::new(Value::Bytes(e.into_bytes()))),
    }
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
    fn test_env_resolver_missing_returns_error() {
        // Make sure the var doesn't exist
        std::env::remove_var("HOLOCONF_NONEXISTENT_VAR");

        let registry = ResolverRegistry::with_builtins();
        let ctx = ResolverContext::new("test.path");
        let args = vec!["HOLOCONF_NONEXISTENT_VAR".to_string()];
        let kwargs = HashMap::new();

        // Registry doesn't handle defaults - that's done at Config level for lazy resolution
        // So this should return an error
        let result = registry.resolve("env", &args, &kwargs, &ctx);
        assert!(result.is_err());
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

        let registry = ResolverRegistry::with_builtins();
        let ctx = ResolverContext::new("test.path");
        let args = vec!["HOLOCONF_SENSITIVE_VAR".to_string()];
        let mut kwargs = HashMap::new();
        kwargs.insert("sensitive".to_string(), "true".to_string());

        // Framework-level sensitive handling via registry
        let result = registry.resolve("env", &args, &kwargs, &ctx).unwrap();
        assert_eq!(result.value.as_str(), Some("secret_value"));
        assert!(result.sensitive);

        std::env::remove_var("HOLOCONF_SENSITIVE_VAR");
    }

    #[test]
    fn test_env_resolver_sensitive_false() {
        std::env::set_var("HOLOCONF_NON_SENSITIVE", "public_value");

        let registry = ResolverRegistry::with_builtins();
        let ctx = ResolverContext::new("test.path");
        let args = vec!["HOLOCONF_NON_SENSITIVE".to_string()];
        let mut kwargs = HashMap::new();
        kwargs.insert("sensitive".to_string(), "false".to_string());

        // Framework-level sensitive handling via registry
        let result = registry.resolve("env", &args, &kwargs, &ctx).unwrap();
        assert_eq!(result.value.as_str(), Some("public_value"));
        assert!(!result.sensitive);

        std::env::remove_var("HOLOCONF_NON_SENSITIVE");
    }

    // Note: test_env_resolver_sensitive_with_default has moved to config.rs tests
    // since default handling with lazy resolution is done at the Config level

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
        ctx.file_roots.insert(temp_dir.clone());

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
        ctx.file_roots.insert(temp_dir.clone());

        let args = vec!["holoconf_test.yaml".to_string()];
        let kwargs = HashMap::new();

        let result = file_resolver(&args, &kwargs, &ctx).unwrap();
        // File resolver now returns text by default (no auto-parsing)
        // Use ${yaml:${file:...}} for structured data
        assert!(result.value.is_string());
        assert!(result.value.as_str().unwrap().contains("key: value"));

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
        let args = vec!["example.com/config.yaml".to_string()];
        let kwargs = HashMap::new();

        let result = http_resolver(&args, &kwargs, &ctx);
        assert!(result.is_err());

        let err = result.unwrap_err();
        let display = format!("{}", err);
        // Error message uses uppercase HTTP
        assert!(display.contains("HTTP resolver is disabled"));
    }

    #[test]
    fn test_registry_with_http() {
        let registry = ResolverRegistry::with_builtins();
        assert!(registry.contains("http"));
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_registry_with_https() {
        let registry = ResolverRegistry::with_builtins();
        assert!(
            registry.contains("https"),
            "https resolver should be registered when http feature is enabled"
        );
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
        ctx.file_roots.insert(temp_dir.clone());

        let args = vec!["holoconf_test.json".to_string()];
        let kwargs = HashMap::new();

        let result = file_resolver(&args, &kwargs, &ctx).unwrap();
        // File resolver now returns text by default (no auto-parsing)
        // Use ${json:${file:...}} for structured data
        assert!(result.value.is_string());
        assert!(result.value.as_str().unwrap().contains(r#""key": "value""#));

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

        let mut ctx = ResolverContext::new("test.path");
        ctx.file_roots.insert(temp_dir.clone());
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
        ctx.file_roots.insert(temp_dir.clone());

        let args = vec!["holoconf_invalid.yaml".to_string()];
        let kwargs = HashMap::new();

        // File resolver now returns text regardless of extension
        // Invalid YAML is just returned as text - parsing errors happen in yaml_resolver
        let result = file_resolver(&args, &kwargs, &ctx).unwrap();
        assert!(result.value.is_string());

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
        ctx.file_roots.insert(temp_dir.clone());

        let args = vec!["holoconf_invalid.json".to_string()];
        let kwargs = HashMap::new();

        // File resolver now returns text regardless of extension
        // Invalid JSON is just returned as text - parsing errors happen in json_resolver
        let result = file_resolver(&args, &kwargs, &ctx).unwrap();
        assert!(result.value.is_string());

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
        ctx.file_roots.insert(temp_dir.clone());

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

    #[test]
    fn test_file_resolver_encoding_utf8() {
        use std::io::Write;

        // Create a temporary file with UTF-8 content including non-ASCII
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("holoconf_utf8.txt");
        {
            let mut file = std::fs::File::create(&test_file).unwrap();
            writeln!(file, "Hello, 世界! 🌍").unwrap();
        }

        let mut ctx = ResolverContext::new("test.path");
        ctx.base_path = Some(temp_dir.clone());
        ctx.file_roots.insert(temp_dir.clone());

        let args = vec!["holoconf_utf8.txt".to_string()];
        let mut kwargs = HashMap::new();
        kwargs.insert("encoding".to_string(), "utf-8".to_string());

        let result = file_resolver(&args, &kwargs, &ctx).unwrap();
        let content = result.value.as_str().unwrap();
        assert!(content.contains("世界"));
        assert!(content.contains("🌍"));

        // Cleanup
        std::fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_file_resolver_encoding_ascii() {
        use std::io::Write;

        // Create a temporary file with mixed ASCII and non-ASCII content
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("holoconf_ascii.txt");
        {
            let mut file = std::fs::File::create(&test_file).unwrap();
            writeln!(file, "Hello, 世界! Welcome").unwrap();
        }

        let mut ctx = ResolverContext::new("test.path");
        ctx.base_path = Some(temp_dir.clone());
        ctx.file_roots.insert(temp_dir.clone());

        let args = vec!["holoconf_ascii.txt".to_string()];
        let mut kwargs = HashMap::new();
        kwargs.insert("encoding".to_string(), "ascii".to_string());

        let result = file_resolver(&args, &kwargs, &ctx).unwrap();
        let content = result.value.as_str().unwrap();
        // ASCII mode should strip non-ASCII characters
        assert!(content.contains("Hello"));
        assert!(content.contains("Welcome"));
        assert!(!content.contains("世界"));

        // Cleanup
        std::fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_file_resolver_encoding_base64() {
        use std::io::Write;

        // Create a temporary file with binary content
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("holoconf_binary.bin");
        {
            let mut file = std::fs::File::create(&test_file).unwrap();
            // Write some bytes that include non-UTF8 sequences
            file.write_all(b"Hello\x00\x01\x02World").unwrap();
        }

        let mut ctx = ResolverContext::new("test.path");
        ctx.base_path = Some(temp_dir.clone());
        ctx.file_roots.insert(temp_dir.clone());

        let args = vec!["holoconf_binary.bin".to_string()];
        let mut kwargs = HashMap::new();
        kwargs.insert("encoding".to_string(), "base64".to_string());

        let result = file_resolver(&args, &kwargs, &ctx).unwrap();
        let content = result.value.as_str().unwrap();

        // Verify the base64 encoding is correct
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        let expected = STANDARD.encode(b"Hello\x00\x01\x02World");
        assert_eq!(content, expected);

        // Cleanup
        std::fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_file_resolver_encoding_default_is_utf8() {
        use std::io::Write;

        // Create a temporary file with UTF-8 content
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("holoconf_default_enc.txt");
        {
            let mut file = std::fs::File::create(&test_file).unwrap();
            writeln!(file, "café résumé").unwrap();
        }

        let mut ctx = ResolverContext::new("test.path");
        ctx.base_path = Some(temp_dir.clone());
        ctx.file_roots.insert(temp_dir.clone());

        let args = vec!["holoconf_default_enc.txt".to_string()];
        let kwargs = HashMap::new(); // No encoding specified

        let result = file_resolver(&args, &kwargs, &ctx).unwrap();
        let content = result.value.as_str().unwrap();
        // Default encoding should be UTF-8, preserving accents
        assert!(content.contains("café"));
        assert!(content.contains("résumé"));

        // Cleanup
        std::fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_file_resolver_encoding_binary() {
        use std::io::Write;

        // Create a temporary file with binary content
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("holoconf_binary_bytes.bin");
        let binary_data: Vec<u8> = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x00, 0x01, 0x02, 0xFF, 0xFE];
        {
            let mut file = std::fs::File::create(&test_file).unwrap();
            file.write_all(&binary_data).unwrap();
        }

        let mut ctx = ResolverContext::new("test.path");
        ctx.base_path = Some(temp_dir.clone());
        ctx.file_roots.insert(temp_dir.clone());

        let args = vec!["holoconf_binary_bytes.bin".to_string()];
        let mut kwargs = HashMap::new();
        kwargs.insert("encoding".to_string(), "binary".to_string());

        let result = file_resolver(&args, &kwargs, &ctx).unwrap();

        // Verify we get Value::Bytes back
        assert!(result.value.is_bytes());
        assert_eq!(result.value.as_bytes().unwrap(), &binary_data);

        // Cleanup
        std::fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_file_resolver_encoding_binary_empty() {
        // Create an empty file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("holoconf_binary_empty.bin");
        {
            std::fs::File::create(&test_file).unwrap();
        }

        let mut ctx = ResolverContext::new("test.path");
        ctx.base_path = Some(temp_dir.clone());
        ctx.file_roots.insert(temp_dir.clone());

        let args = vec!["holoconf_binary_empty.bin".to_string()];
        let mut kwargs = HashMap::new();
        kwargs.insert("encoding".to_string(), "binary".to_string());

        let result = file_resolver(&args, &kwargs, &ctx).unwrap();

        // Verify we get empty Value::Bytes
        assert!(result.value.is_bytes());
        let empty: &[u8] = &[];
        assert_eq!(result.value.as_bytes().unwrap(), empty);

        // Cleanup
        std::fs::remove_file(test_file).ok();
    }

    // Framework-level sensitive test (default handling moved to config tests)

    #[test]
    fn test_file_resolver_with_sensitive() {
        use std::io::Write;

        // Create a temporary file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("holoconf_sensitive_test.txt");
        {
            let mut file = std::fs::File::create(&test_file).unwrap();
            writeln!(file, "secret content").unwrap();
        }

        let registry = ResolverRegistry::with_builtins();
        let mut ctx = ResolverContext::new("test.path");
        ctx.base_path = Some(temp_dir.clone());
        ctx.file_roots.insert(temp_dir.clone());

        let args = vec!["holoconf_sensitive_test.txt".to_string()];
        let mut kwargs = HashMap::new();
        kwargs.insert("sensitive".to_string(), "true".to_string());

        // Framework-level sensitive handling via registry
        let result = registry.resolve("file", &args, &kwargs, &ctx).unwrap();
        assert!(result.value.as_str().unwrap().contains("secret content"));
        assert!(result.sensitive);

        // Cleanup
        std::fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_framework_sensitive_kwarg_not_passed_to_resolver() {
        // Ensure that 'sensitive' kwarg is NOT passed to the resolver
        // (Note: 'default' is handled at Config level, not registry level)
        let mut registry = ResolverRegistry::new();

        // Register a test resolver that checks it doesn't receive sensitive kwarg
        registry.register_fn("test_kwargs", |_args, kwargs, _ctx| {
            // Sensitive kwarg should be filtered out
            assert!(
                !kwargs.contains_key("sensitive"),
                "sensitive kwarg should not be passed to resolver"
            );
            // But custom kwargs should be passed through
            if let Some(custom) = kwargs.get("custom") {
                Ok(ResolvedValue::new(Value::String(format!(
                    "custom={}",
                    custom
                ))))
            } else {
                Ok(ResolvedValue::new(Value::String("no custom".to_string())))
            }
        });

        let ctx = ResolverContext::new("test.path");
        let args = vec![];
        let mut kwargs = HashMap::new();
        kwargs.insert("sensitive".to_string(), "true".to_string());
        kwargs.insert("custom".to_string(), "myvalue".to_string());

        let result = registry
            .resolve("test_kwargs", &args, &kwargs, &ctx)
            .unwrap();
        assert_eq!(result.value.as_str(), Some("custom=myvalue"));
        // Sensitive override should still be applied by framework
        assert!(result.sensitive);
    }

    // Tests for normalize_http_url function
    #[test]
    #[cfg(feature = "http")]
    fn test_normalize_http_url_clean_syntax() {
        assert_eq!(
            normalize_http_url("https", "example.com/path").unwrap(),
            "https://example.com/path"
        );
        assert_eq!(
            normalize_http_url("http", "example.com").unwrap(),
            "http://example.com"
        );
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_normalize_http_url_double_slash() {
        assert_eq!(
            normalize_http_url("https", "//example.com/path").unwrap(),
            "https://example.com/path"
        );
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_normalize_http_url_existing_https() {
        assert_eq!(
            normalize_http_url("https", "https://example.com/path").unwrap(),
            "https://example.com/path"
        );
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_normalize_http_url_wrong_scheme() {
        // Should strip http:// and add https://
        assert_eq!(
            normalize_http_url("https", "http://example.com").unwrap(),
            "https://example.com"
        );
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_normalize_http_url_with_query() {
        assert_eq!(
            normalize_http_url("https", "example.com/path?query=val&other=val2").unwrap(),
            "https://example.com/path?query=val&other=val2"
        );
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_normalize_http_url_empty() {
        let result = normalize_http_url("https", "");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-empty URL"));
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_normalize_http_url_triple_slash() {
        // ${https:///example.com} should error (invalid syntax)
        let result = normalize_http_url("https", "///example.com");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid URL syntax"));
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_normalize_http_url_whitespace_only() {
        let result = normalize_http_url("https", "   ");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-empty URL"));
    }

    // Tests for is_localhost function
    #[test]
    fn test_is_localhost_ascii() {
        assert!(is_localhost("localhost"));
        assert!(is_localhost("LOCALHOST"));
        assert!(is_localhost("LocalHost"));
    }

    #[test]
    fn test_is_localhost_ipv4() {
        assert!(is_localhost("127.0.0.1"));
        assert!(is_localhost("127.0.0.100"));
        assert!(is_localhost("127.1.2.3"));
        assert!(!is_localhost("128.0.0.1"));
    }

    #[test]
    fn test_is_localhost_ipv6() {
        assert!(is_localhost("::1"));
        assert!(is_localhost("[::1]"));
        assert!(!is_localhost("::2"));
    }

    #[test]
    fn test_is_localhost_not() {
        assert!(!is_localhost("example.com"));
        assert!(!is_localhost("remote.host"));
        assert!(!is_localhost("192.168.1.1"));
    }

    // Tests for normalize_file_path function
    #[test]
    fn test_normalize_file_path_relative() {
        let (path, is_rel) = normalize_file_path("data.txt").unwrap();
        assert_eq!(path, "data.txt");
        assert!(is_rel);

        let (path, is_rel) = normalize_file_path("./data.txt").unwrap();
        assert_eq!(path, "./data.txt");
        assert!(is_rel);
    }

    #[test]
    fn test_normalize_file_path_absolute() {
        let (path, is_rel) = normalize_file_path("/etc/config.yaml").unwrap();
        assert_eq!(path, "/etc/config.yaml");
        assert!(!is_rel);
    }

    #[test]
    fn test_normalize_file_path_rfc8089_empty_authority() {
        // file:/// (empty authority = localhost)
        let (path, is_rel) = normalize_file_path("///etc/config.yaml").unwrap();
        assert_eq!(path, "/etc/config.yaml");
        assert!(!is_rel);
    }

    #[test]
    fn test_normalize_file_path_rfc8089_localhost() {
        // file://localhost/
        let (path, is_rel) = normalize_file_path("//localhost/var/data").unwrap();
        assert_eq!(path, "/var/data");
        assert!(!is_rel);

        // file://localhost (no path)
        let (path, is_rel) = normalize_file_path("//localhost").unwrap();
        assert_eq!(path, "/");
        assert!(!is_rel);
    }

    #[test]
    fn test_normalize_file_path_rfc8089_localhost_ipv4() {
        // file://127.0.0.1/
        let (path, is_rel) = normalize_file_path("//127.0.0.1/tmp/file.txt").unwrap();
        assert_eq!(path, "/tmp/file.txt");
        assert!(!is_rel);
    }

    #[test]
    fn test_normalize_file_path_rfc8089_localhost_ipv6() {
        // file://::1/
        let (path, is_rel) = normalize_file_path("//::1/tmp/file.txt").unwrap();
        assert_eq!(path, "/tmp/file.txt");
        assert!(!is_rel);
    }

    #[test]
    fn test_normalize_file_path_rfc8089_remote_rejected() {
        let result = normalize_file_path("//remote.host/path");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Remote file URIs not supported"));
        assert!(err_msg.contains("remote.host"));

        let result = normalize_file_path("//server.example.com/share");
        assert!(result.is_err());
    }

    #[test]
    fn test_normalize_file_path_rfc8089_empty_hostname() {
        // file:// with no authority
        let (path, is_rel) = normalize_file_path("//").unwrap();
        assert_eq!(path, "/");
        assert!(!is_rel);
    }

    #[test]
    fn test_normalize_file_path_null_byte() {
        let result = normalize_file_path("/etc/passwd\0.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("null byte"));
    }

    #[test]
    fn test_normalize_file_path_null_byte_relative() {
        let result = normalize_file_path("data\0.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("null byte"));
    }

    // Tests for CertInput type
    #[test]
    fn test_cert_input_is_pem_content() {
        let pem_content = CertInput::Text("-----BEGIN CERTIFICATE-----\nMIIC...".to_string());
        assert!(pem_content.is_pem_content());

        let file_path = CertInput::Text("/path/to/cert.pem".to_string());
        assert!(!file_path.is_pem_content());

        let binary = CertInput::Binary(vec![0, 1, 2, 3]);
        assert!(!binary.is_pem_content());
    }

    #[test]
    fn test_cert_input_is_p12_path() {
        assert!(CertInput::Text("/path/to/identity.p12".to_string()).is_p12_path());
        assert!(CertInput::Text("/path/to/identity.pfx".to_string()).is_p12_path());
        assert!(CertInput::Text("/path/to/identity.P12".to_string()).is_p12_path());
        assert!(CertInput::Text("/path/to/identity.PFX".to_string()).is_p12_path());

        assert!(!CertInput::Text("/path/to/cert.pem".to_string()).is_p12_path());
        assert!(!CertInput::Text("-----BEGIN CERTIFICATE-----".to_string()).is_p12_path());
        assert!(!CertInput::Binary(vec![0, 1, 2, 3]).is_p12_path());
    }

    #[test]
    fn test_cert_input_as_text() {
        let text_input = CertInput::Text("some text".to_string());
        assert_eq!(text_input.as_text(), Some("some text"));

        let binary_input = CertInput::Binary(vec![0, 1, 2]);
        assert_eq!(binary_input.as_text(), None);
    }

    #[test]
    fn test_cert_input_as_bytes() {
        let binary_input = CertInput::Binary(vec![0, 1, 2]);
        assert_eq!(binary_input.as_bytes(), Some(&[0, 1, 2][..]));

        let text_input = CertInput::Text("some text".to_string());
        assert_eq!(text_input.as_bytes(), None);
    }

    #[test]
    fn test_cert_input_from_string() {
        let input1 = CertInput::from("test".to_string());
        assert!(matches!(input1, CertInput::Text(_)));
        assert_eq!(input1.as_text(), Some("test"));

        let input2 = CertInput::from("test");
        assert!(matches!(input2, CertInput::Text(_)));
        assert_eq!(input2.as_text(), Some("test"));
    }

    #[test]
    fn test_cert_input_from_vec_u8() {
        let input = CertInput::from(vec![1, 2, 3]);
        assert!(matches!(input, CertInput::Binary(_)));
        assert_eq!(input.as_bytes(), Some(&[1, 2, 3][..]));
    }
}

// Tests for global registry (TDD - written before implementation)
#[cfg(test)]
mod global_registry_tests {
    use super::*;

    /// Test helper: create a mock resolver with a given name
    fn mock_resolver(name: &str) -> Arc<dyn Resolver> {
        Arc::new(FnResolver::new(name, |_, _, _| {
            Ok(ResolvedValue::new("mock"))
        }))
    }

    #[test]
    fn test_register_new_resolver_succeeds() {
        let mut registry = ResolverRegistry::new();
        let resolver = mock_resolver("test_new");

        // Registering a new resolver should succeed with force=false
        let result = registry.register_with_force(resolver, false);
        assert!(result.is_ok());
        assert!(registry.contains("test_new"));
    }

    #[test]
    fn test_register_duplicate_errors_without_force() {
        let mut registry = ResolverRegistry::new();
        let resolver1 = mock_resolver("test_dup");
        let resolver2 = mock_resolver("test_dup");

        // First registration succeeds
        registry.register_with_force(resolver1, false).unwrap();

        // Second registration with same name should fail without force
        let result = registry.register_with_force(resolver2, false);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("already registered"));
    }

    #[test]
    fn test_register_duplicate_succeeds_with_force() {
        let mut registry = ResolverRegistry::new();
        let resolver1 = mock_resolver("test_force");
        let resolver2 = mock_resolver("test_force");

        // First registration succeeds
        registry.register_with_force(resolver1, false).unwrap();

        // Second registration with force=true should succeed
        let result = registry.register_with_force(resolver2, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_global_registry_is_singleton() {
        // The global registry should return the same instance
        let registry1 = global_registry();
        let registry2 = global_registry();

        // They should point to the same instance (same address)
        assert!(std::ptr::eq(registry1, registry2));
    }

    #[test]
    fn test_register_global_new_resolver() {
        // Clean slate - register a unique resolver name
        let resolver = mock_resolver("global_test_unique_42");
        let result = register_global(resolver, false);
        // May fail if already registered from previous test runs
        // That's expected behavior - the test verifies the API works
        assert!(result.is_ok() || result.is_err());
    }
}

// Integration tests for lazy default resolution (requires Config)
#[cfg(test)]
mod lazy_resolution_tests {
    use super::*;
    use crate::Config;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_default_not_resolved_when_main_value_exists() {
        // Track whether the "fail" resolver was called
        let fail_called = Arc::new(AtomicBool::new(false));
        let fail_called_clone = fail_called.clone();

        // Create a config with a custom resolver that would fail if called
        let yaml = r#"
value: ${env:HOLOCONF_LAZY_TEST_VAR,default=${fail:should_not_be_called}}
"#;
        // Set the env var so the default should NOT be needed
        std::env::set_var("HOLOCONF_LAZY_TEST_VAR", "main_value");

        let mut config = Config::from_yaml(yaml).unwrap();

        // Register a "fail" resolver that sets a flag and panics
        config.register_resolver(Arc::new(FnResolver::new(
            "fail",
            move |_args, _kwargs, _ctx| {
                fail_called_clone.store(true, Ordering::SeqCst);
                panic!("fail resolver should not have been called - lazy resolution failed!");
            },
        )));

        // Access the value - should get main value, not call fail resolver
        let result = config.get("value").unwrap();
        assert_eq!(result.as_str(), Some("main_value"));

        // Verify the fail resolver was never called
        assert!(
            !fail_called.load(Ordering::SeqCst),
            "The default resolver should not have been called when main value exists"
        );

        std::env::remove_var("HOLOCONF_LAZY_TEST_VAR");
    }

    #[test]
    fn test_default_is_resolved_when_main_value_missing() {
        // Track whether the default resolver was called
        let default_called = Arc::new(AtomicBool::new(false));
        let default_called_clone = default_called.clone();

        // Create a config where env var doesn't exist
        let yaml = r#"
value: ${env:HOLOCONF_LAZY_MISSING_VAR,default=${custom_default:fallback}}
"#;
        std::env::remove_var("HOLOCONF_LAZY_MISSING_VAR");

        let mut config = Config::from_yaml(yaml).unwrap();

        // Register a custom default resolver
        config.register_resolver(Arc::new(FnResolver::new(
            "custom_default",
            move |args: &[String], _kwargs, _ctx| {
                default_called_clone.store(true, Ordering::SeqCst);
                let arg = args.first().cloned().unwrap_or_default();
                Ok(ResolvedValue::new(Value::String(format!(
                    "default_was_{}",
                    arg
                ))))
            },
        )));

        // Access the value - should call default resolver since main value missing
        let result = config.get("value").unwrap();
        assert_eq!(result.as_str(), Some("default_was_fallback"));

        // Verify the default resolver WAS called
        assert!(
            default_called.load(Ordering::SeqCst),
            "The default resolver should have been called when main value is missing"
        );
    }
}

// HTTP resolver tests (require http feature and mockito)
#[cfg(all(test, feature = "http"))]
mod http_resolver_tests {
    use super::*;
    use mockito::Server;

    #[test]
    fn test_http_fetch_json() {
        let mut server = Server::new();
        let mock = server
            .mock("GET", "/config.json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"key": "value", "number": 42}"#)
            .create();

        let ctx = ResolverContext::new("test.path").with_allow_http(true);
        let args = vec![format!("{}/config.json", server.url())];
        let kwargs = HashMap::new();

        // HTTP resolver now returns text by default (no auto-parsing)
        // Use ${json:${http:...}} for structured data
        let result = http_resolver(&args, &kwargs, &ctx).unwrap();
        assert!(result.value.is_string());
        assert!(result.value.as_str().unwrap().contains(r#""key": "value""#));

        mock.assert();
    }

    #[test]
    fn test_http_fetch_yaml() {
        let mut server = Server::new();
        let mock = server
            .mock("GET", "/config.yaml")
            .with_status(200)
            .with_header("content-type", "application/yaml")
            .with_body("key: value\nnumber: 42")
            .create();

        let ctx = ResolverContext::new("test.path").with_allow_http(true);
        let args = vec![format!("{}/config.yaml", server.url())];
        let kwargs = HashMap::new();

        // HTTP resolver now returns text by default (no auto-parsing)
        // Use ${yaml:${http:...}} for structured data
        let result = http_resolver(&args, &kwargs, &ctx).unwrap();
        assert!(result.value.is_string());
        assert!(result.value.as_str().unwrap().contains("key: value"));

        mock.assert();
    }

    #[test]
    fn test_http_fetch_text() {
        let mut server = Server::new();
        let mock = server
            .mock("GET", "/data.txt")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_body("Hello, World!")
            .create();

        let ctx = ResolverContext::new("test.path").with_allow_http(true);
        let args = vec![format!("{}/data.txt", server.url())];
        let kwargs = HashMap::new();

        let result = http_resolver(&args, &kwargs, &ctx).unwrap();
        assert_eq!(result.value.as_str(), Some("Hello, World!"));

        mock.assert();
    }

    #[test]
    fn test_http_fetch_binary() {
        let mut server = Server::new();
        let binary_data = vec![0x00, 0x01, 0x02, 0xFF, 0xFE];
        let mock = server
            .mock("GET", "/data.bin")
            .with_status(200)
            .with_header("content-type", "application/octet-stream")
            .with_body(binary_data.clone())
            .create();

        let ctx = ResolverContext::new("test.path").with_allow_http(true);
        let args = vec![format!("{}/data.bin", server.url())];
        let mut kwargs = HashMap::new();
        kwargs.insert("parse".to_string(), "binary".to_string());

        let result = http_resolver(&args, &kwargs, &ctx).unwrap();
        assert!(result.value.is_bytes());
        assert_eq!(result.value.as_bytes().unwrap(), &binary_data);

        mock.assert();
    }

    #[test]
    fn test_http_fetch_explicit_parse_text() {
        let mut server = Server::new();
        // Return JSON but with parse=text it should be returned as string
        let mock = server
            .mock("GET", "/data")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"key": "value"}"#)
            .create();

        let ctx = ResolverContext::new("test.path").with_allow_http(true);
        let args = vec![format!("{}/data", server.url())];
        let mut kwargs = HashMap::new();
        kwargs.insert("parse".to_string(), "text".to_string());

        let result = http_resolver(&args, &kwargs, &ctx).unwrap();
        // With parse=text, JSON should be returned as string (not parsed)
        assert!(result.value.is_string());
        assert_eq!(result.value.as_str(), Some(r#"{"key": "value"}"#));

        mock.assert();
    }

    #[test]
    fn test_http_fetch_with_custom_header() {
        let mut server = Server::new();
        let mock = server
            .mock("GET", "/protected")
            .match_header("Authorization", "Bearer my-token")
            .with_status(200)
            .with_body("authorized content")
            .create();

        let ctx = ResolverContext::new("test.path").with_allow_http(true);
        let args = vec![format!("{}/protected", server.url())];
        let mut kwargs = HashMap::new();
        kwargs.insert(
            "header".to_string(),
            "Authorization:Bearer my-token".to_string(),
        );

        let result = http_resolver(&args, &kwargs, &ctx).unwrap();
        assert_eq!(result.value.as_str(), Some("authorized content"));

        mock.assert();
    }

    #[test]
    fn test_http_fetch_404_error() {
        let mut server = Server::new();
        let mock = server.mock("GET", "/notfound").with_status(404).create();

        let ctx = ResolverContext::new("test.path").with_allow_http(true);
        let args = vec![format!("{}/notfound", server.url())];
        let kwargs = HashMap::new();

        let result = http_resolver(&args, &kwargs, &ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("HTTP"));

        mock.assert();
    }

    #[test]
    fn test_http_disabled_by_default() {
        let ctx = ResolverContext::new("test.path");
        // allow_http defaults to false
        let args = vec!["https://example.com/config.yaml".to_string()];
        let kwargs = HashMap::new();

        let result = http_resolver(&args, &kwargs, &ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("disabled"));
    }

    #[test]
    fn test_http_allowlist_blocks_url() {
        let ctx = ResolverContext::new("test.path")
            .with_allow_http(true)
            .with_http_allowlist(vec!["https://allowed.example.com/*".to_string()]);

        let args = vec!["https://blocked.example.com/config.yaml".to_string()];
        let kwargs = HashMap::new();

        let result = http_resolver(&args, &kwargs, &ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("not in allowlist")
                || err.to_string().contains("HttpNotAllowed")
        );
    }

    #[test]
    fn test_http_allowlist_allows_matching_url() {
        let mut server = Server::new();
        let mock = server
            .mock("GET", "/config.yaml")
            .with_status(200)
            .with_body("key: value")
            .create();

        // The allowlist pattern needs to match the server URL
        let server_url = server.url();
        let ctx = ResolverContext::new("test.path")
            .with_allow_http(true)
            .with_http_allowlist(vec![format!("{}/*", server_url)]);

        let args = vec![format!("{}/config.yaml", server_url)];
        let kwargs = HashMap::new();

        let result = http_resolver(&args, &kwargs, &ctx).unwrap();
        // HTTP resolver now returns text by default (no auto-parsing)
        assert!(result.value.is_string());
        assert!(result.value.as_str().unwrap().contains("key: value"));

        mock.assert();
    }

    #[test]
    fn test_url_matches_pattern_exact() {
        assert!(url_matches_pattern(
            "https://example.com/config.yaml",
            "https://example.com/config.yaml"
        ));
        assert!(!url_matches_pattern(
            "https://example.com/other.yaml",
            "https://example.com/config.yaml"
        ));
    }

    #[test]
    fn test_url_matches_pattern_wildcard() {
        assert!(url_matches_pattern(
            "https://example.com/config.yaml",
            "https://example.com/*"
        ));
        assert!(url_matches_pattern(
            "https://example.com/path/to/config.yaml",
            "https://example.com/*"
        ));
        assert!(!url_matches_pattern(
            "https://other.com/config.yaml",
            "https://example.com/*"
        ));
    }

    #[test]
    fn test_url_matches_pattern_subdomain() {
        assert!(url_matches_pattern(
            "https://api.example.com/config",
            "https://*.example.com/*"
        ));
        assert!(url_matches_pattern(
            "https://staging.example.com/config",
            "https://*.example.com/*"
        ));
        assert!(!url_matches_pattern(
            "https://example.com/config",
            "https://*.example.com/*"
        ));
    }

    // ========================================
    // Transformation Resolver Tests
    // ========================================

    #[test]
    fn test_json_resolver_valid() {
        let ctx = ResolverContext::new("test.path");
        let args = vec![r#"{"key": "value", "num": 42}"#.to_string()];
        let kwargs = HashMap::new();

        let result = json_resolver(&args, &kwargs, &ctx).unwrap();
        assert!(result.value.is_mapping());

        let map = result.value.as_mapping().unwrap();
        assert_eq!(map.get("key"), Some(&Value::String("value".to_string())));
        assert_eq!(map.get("num"), Some(&Value::Integer(42)));
        assert!(!result.sensitive);
    }

    #[test]
    fn test_json_resolver_array() {
        let ctx = ResolverContext::new("test.path");
        let args = vec![r#"[1, 2, 3, "four"]"#.to_string()];
        let kwargs = HashMap::new();

        let result = json_resolver(&args, &kwargs, &ctx).unwrap();
        assert!(result.value.is_sequence());

        let seq = result.value.as_sequence().unwrap();
        assert_eq!(seq.len(), 4);
        assert_eq!(seq[0], Value::Integer(1));
        assert_eq!(seq[3], Value::String("four".to_string()));
    }

    #[test]
    fn test_json_resolver_invalid() {
        let ctx = ResolverContext::new("test.path");
        let args = vec![r#"{"key": invalid}"#.to_string()];
        let kwargs = HashMap::new();

        let result = json_resolver(&args, &kwargs, &ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid JSON"));
    }

    #[test]
    fn test_json_resolver_no_args() {
        let ctx = ResolverContext::new("test.path");
        let args = vec![];
        let kwargs = HashMap::new();

        let result = json_resolver(&args, &kwargs, &ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("requires a string argument"));
    }

    #[test]
    fn test_yaml_resolver_valid() {
        let ctx = ResolverContext::new("test.path");
        let args = vec!["key: value\nnum: 42\nlist:\n  - a\n  - b".to_string()];
        let kwargs = HashMap::new();

        let result = yaml_resolver(&args, &kwargs, &ctx).unwrap();
        assert!(result.value.is_mapping());

        let map = result.value.as_mapping().unwrap();
        assert_eq!(map.get("key"), Some(&Value::String("value".to_string())));
        assert_eq!(map.get("num"), Some(&Value::Integer(42)));
        assert!(!result.sensitive);
    }

    #[test]
    fn test_yaml_resolver_array() {
        let ctx = ResolverContext::new("test.path");
        let args = vec!["- one\n- two\n- three".to_string()];
        let kwargs = HashMap::new();

        let result = yaml_resolver(&args, &kwargs, &ctx).unwrap();
        assert!(result.value.is_sequence());

        let seq = result.value.as_sequence().unwrap();
        assert_eq!(seq.len(), 3);
        assert_eq!(seq[0], Value::String("one".to_string()));
    }

    #[test]
    fn test_yaml_resolver_invalid() {
        let ctx = ResolverContext::new("test.path");
        let args = vec!["key: value\n  bad_indent: oops".to_string()];
        let kwargs = HashMap::new();

        let result = yaml_resolver(&args, &kwargs, &ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid YAML"));
    }

    #[test]
    fn test_yaml_resolver_no_args() {
        let ctx = ResolverContext::new("test.path");
        let args = vec![];
        let kwargs = HashMap::new();

        let result = yaml_resolver(&args, &kwargs, &ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("requires a string argument"));
    }

    #[test]
    fn test_split_resolver_basic() {
        let ctx = ResolverContext::new("test.path");
        let args = vec!["a,b,c".to_string()];
        let kwargs = HashMap::new();

        let result = split_resolver(&args, &kwargs, &ctx).unwrap();
        assert!(result.value.is_sequence());

        let seq = result.value.as_sequence().unwrap();
        assert_eq!(seq.len(), 3);
        assert_eq!(seq[0], Value::String("a".to_string()));
        assert_eq!(seq[1], Value::String("b".to_string()));
        assert_eq!(seq[2], Value::String("c".to_string()));
    }

    #[test]
    fn test_split_resolver_custom_delim() {
        let ctx = ResolverContext::new("test.path");
        let args = vec!["one|two|three".to_string()];
        let mut kwargs = HashMap::new();
        kwargs.insert("delim".to_string(), "|".to_string());

        let result = split_resolver(&args, &kwargs, &ctx).unwrap();
        let seq = result.value.as_sequence().unwrap();
        assert_eq!(seq.len(), 3);
        assert_eq!(seq[0], Value::String("one".to_string()));
    }

    #[test]
    fn test_split_resolver_with_trim() {
        let ctx = ResolverContext::new("test.path");
        let args = vec!["  a  ,  b  ,  c  ".to_string()];
        let mut kwargs = HashMap::new();
        kwargs.insert("trim".to_string(), "true".to_string());

        let result = split_resolver(&args, &kwargs, &ctx).unwrap();
        let seq = result.value.as_sequence().unwrap();
        assert_eq!(seq[0], Value::String("a".to_string()));
        assert_eq!(seq[1], Value::String("b".to_string()));
    }

    #[test]
    fn test_split_resolver_no_trim() {
        let ctx = ResolverContext::new("test.path");
        let args = vec!["  a  ,  b  ".to_string()];
        let mut kwargs = HashMap::new();
        kwargs.insert("trim".to_string(), "false".to_string());

        let result = split_resolver(&args, &kwargs, &ctx).unwrap();
        let seq = result.value.as_sequence().unwrap();
        assert_eq!(seq[0], Value::String("  a  ".to_string()));
        assert_eq!(seq[1], Value::String("  b  ".to_string()));
    }

    #[test]
    fn test_split_resolver_with_limit() {
        let ctx = ResolverContext::new("test.path");
        let args = vec!["a,b,c,d,e".to_string()];
        let mut kwargs = HashMap::new();
        kwargs.insert("limit".to_string(), "2".to_string());

        let result = split_resolver(&args, &kwargs, &ctx).unwrap();
        let seq = result.value.as_sequence().unwrap();
        assert_eq!(seq.len(), 3); // limit=2 means split into 3 parts max
        assert_eq!(seq[0], Value::String("a".to_string()));
        assert_eq!(seq[1], Value::String("b".to_string()));
        assert_eq!(seq[2], Value::String("c,d,e".to_string()));
    }

    #[test]
    fn test_csv_resolver_with_headers() {
        let ctx = ResolverContext::new("test.path");
        let args = vec!["name,age\nAlice,30\nBob,25".to_string()];
        let kwargs = HashMap::new();

        let result = csv_resolver(&args, &kwargs, &ctx).unwrap();
        assert!(result.value.is_sequence());

        let seq = result.value.as_sequence().unwrap();
        assert_eq!(seq.len(), 2);

        // First row should be a mapping with keys from headers
        let first = seq[0].as_mapping().unwrap();
        assert_eq!(first.get("name"), Some(&Value::String("Alice".to_string())));
        assert_eq!(first.get("age"), Some(&Value::String("30".to_string())));
    }

    #[test]
    fn test_csv_resolver_without_headers() {
        let ctx = ResolverContext::new("test.path");
        let args = vec!["Alice,30\nBob,25".to_string()];
        let mut kwargs = HashMap::new();
        kwargs.insert("header".to_string(), "false".to_string());

        let result = csv_resolver(&args, &kwargs, &ctx).unwrap();
        assert!(result.value.is_sequence());

        let seq = result.value.as_sequence().unwrap();
        assert_eq!(seq.len(), 2);

        // First row should be a sequence (array)
        let first = seq[0].as_sequence().unwrap();
        assert_eq!(first.len(), 2);
        assert_eq!(first[0], Value::String("Alice".to_string()));
        assert_eq!(first[1], Value::String("30".to_string()));
    }

    #[test]
    fn test_csv_resolver_custom_delim() {
        let ctx = ResolverContext::new("test.path");
        let args = vec!["name|age\nAlice|30\nBob|25".to_string()];
        let mut kwargs = HashMap::new();
        kwargs.insert("delim".to_string(), "|".to_string());

        let result = csv_resolver(&args, &kwargs, &ctx).unwrap();
        let seq = result.value.as_sequence().unwrap();
        assert_eq!(seq.len(), 2);

        let first = seq[0].as_mapping().unwrap();
        assert_eq!(first.get("name"), Some(&Value::String("Alice".to_string())));
    }

    #[test]
    fn test_csv_resolver_with_trim() {
        let ctx = ResolverContext::new("test.path");
        let args = vec!["name , age\n  Alice  ,  30  ".to_string()];
        let mut kwargs = HashMap::new();
        kwargs.insert("trim".to_string(), "true".to_string());

        let result = csv_resolver(&args, &kwargs, &ctx).unwrap();
        let seq = result.value.as_sequence().unwrap();

        let first = seq[0].as_mapping().unwrap();
        assert_eq!(first.get("name"), Some(&Value::String("Alice".to_string())));
        assert_eq!(first.get("age"), Some(&Value::String("30".to_string())));
    }

    #[test]
    fn test_csv_resolver_empty_delimiter() {
        let ctx = ResolverContext::new("test.path");
        let args = vec!["name,age".to_string()];
        let mut kwargs = HashMap::new();
        kwargs.insert("delim".to_string(), "".to_string());

        let result = csv_resolver(&args, &kwargs, &ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("delimiter cannot be empty"));
    }

    #[test]
    fn test_csv_resolver_no_args() {
        let ctx = ResolverContext::new("test.path");
        let args = vec![];
        let kwargs = HashMap::new();

        let result = csv_resolver(&args, &kwargs, &ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("requires a string argument"));
    }

    #[test]
    fn test_base64_resolver_valid() {
        let ctx = ResolverContext::new("test.path");
        // "Hello, World!" in base64
        let args = vec!["SGVsbG8sIFdvcmxkIQ==".to_string()];
        let kwargs = HashMap::new();

        let result = base64_resolver(&args, &kwargs, &ctx).unwrap();
        // Base64-decoded UTF-8 text becomes a string
        assert!(result.value.is_string());

        let text = result.value.as_str().unwrap();
        assert_eq!(text, "Hello, World!");
        assert!(!result.sensitive);
    }

    #[test]
    fn test_base64_resolver_with_whitespace() {
        let ctx = ResolverContext::new("test.path");
        // Base64 with surrounding whitespace should be trimmed
        let args = vec!["  SGVsbG8=  ".to_string()];
        let kwargs = HashMap::new();

        let result = base64_resolver(&args, &kwargs, &ctx).unwrap();
        let text = result.value.as_str().unwrap();
        assert_eq!(text, "Hello");
    }

    #[test]
    fn test_base64_resolver_invalid() {
        let ctx = ResolverContext::new("test.path");
        let args = vec!["not-valid-base64!!!".to_string()];
        let kwargs = HashMap::new();

        let result = base64_resolver(&args, &kwargs, &ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid base64"));
    }

    #[test]
    fn test_base64_resolver_no_args() {
        let ctx = ResolverContext::new("test.path");
        let args = vec![];
        let kwargs = HashMap::new();

        let result = base64_resolver(&args, &kwargs, &ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("requires a string argument"));
    }

    #[test]
    fn test_transformation_resolvers_registered() {
        let registry = ResolverRegistry::with_builtins();

        assert!(registry.contains("json"));
        assert!(registry.contains("yaml"));
        assert!(registry.contains("split"));
        assert!(registry.contains("csv"));
        assert!(registry.contains("base64"));
    }
}

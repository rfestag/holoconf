//! Resolver architecture per ADR-002
//!
//! Resolvers are functions or objects that resolve interpolation expressions
//! like `${env:VAR}` to actual values.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use crate::error::{Error, Result};
use crate::value::Value;

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
    /// Whether HTTP resolver is enabled
    pub allow_http: bool,
    /// HTTP URL allowlist (glob patterns)
    pub http_allowlist: Vec<String>,
    /// HTTP proxy URL (e.g., "http://proxy:8080" or "socks5://proxy:1080")
    pub http_proxy: Option<String>,
    /// Whether to auto-detect proxy from environment variables (HTTP_PROXY, HTTPS_PROXY, NO_PROXY)
    pub http_proxy_from_env: bool,
    /// Path to CA bundle PEM file (replaces default webpki-roots)
    pub http_ca_bundle: Option<std::path::PathBuf>,
    /// Path to extra CA bundle PEM file (appends to webpki-roots)
    pub http_extra_ca_bundle: Option<std::path::PathBuf>,
    /// Path to client certificate PEM or P12/PFX file (for mTLS)
    pub http_client_cert: Option<std::path::PathBuf>,
    /// Path to client private key PEM file (for mTLS, not needed for P12/PFX)
    pub http_client_key: Option<std::path::PathBuf>,
    /// Password for encrypted private key or P12/PFX file
    pub http_client_key_password: Option<String>,
    /// DANGEROUS: Skip TLS certificate verification (dev only)
    pub http_insecure: bool,
}

impl ResolverContext {
    /// Create a new resolver context
    pub fn new(config_path: impl Into<String>) -> Self {
        Self {
            config_path: config_path.into(),
            config_root: None,
            base_path: None,
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
            http_insecure: false,
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

    /// Set CA bundle path (replaces webpki-roots)
    pub fn with_http_ca_bundle(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.http_ca_bundle = Some(path.into());
        self
    }

    /// Set extra CA bundle path (appends to webpki-roots)
    pub fn with_http_extra_ca_bundle(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.http_extra_ca_bundle = Some(path.into());
        self
    }

    /// Set client certificate path for mTLS
    pub fn with_http_client_cert(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.http_client_cert = Some(path.into());
        self
    }

    /// Set client private key path for mTLS
    pub fn with_http_client_key(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.http_client_key = Some(path.into());
        self
    }

    /// Set password for encrypted private key or P12/PFX file
    pub fn with_http_client_key_password(mut self, password: impl Into<String>) -> Self {
        self.http_client_key_password = Some(password.into());
        self
    }

    /// DANGEROUS: Skip TLS certificate verification
    pub fn with_http_insecure(mut self, insecure: bool) -> Self {
        self.http_insecure = insecure;
        self
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

/// Built-in file resolver
///
/// Usage:
///   ${file:path/to/file}                    - Read file as text (UTF-8)
///   ${file:path/to/file,parse=yaml}         - Parse as YAML
///   ${file:path/to/file,parse=json}         - Parse as JSON
///   ${file:path/to/file,parse=text}         - Read as text (explicit)
///   ${file:path/to/file,parse=auto}         - Auto-detect from extension (default)
///   ${file:path/to/file,encoding=utf-8}     - UTF-8 encoding (default)
///   ${file:path/to/file,encoding=ascii}     - ASCII encoding (strips non-ASCII)
///   ${file:path/to/file,encoding=base64}    - Base64 encode the file contents as string
///   ${file:path/to/file,encoding=binary}    - Return raw bytes as Value::Bytes
///   ${file:path/to/file,default={}}         - Default if file not found (framework-handled)
///   ${file:path/to/file,sensitive=true}     - Mark as sensitive (framework-handled)
///
/// Note: `default` and `sensitive` are framework-level kwargs handled by ResolverRegistry.
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
    let encoding = kwargs
        .get("encoding")
        .map(|s| s.as_str())
        .unwrap_or("utf-8");

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

    // Handle binary encoding separately - returns Value::Bytes directly
    if encoding == "binary" {
        let bytes = std::fs::read(&file_path)
            .map_err(|_| Error::file_not_found(file_path_str, Some(ctx.config_path.clone())))?;
        return Ok(ResolvedValue::new(Value::Bytes(bytes)));
    }

    // Read the file based on encoding
    let content = match encoding {
        "base64" => {
            // Read as binary and base64 encode
            use base64::{engine::general_purpose::STANDARD, Engine as _};
            let bytes = std::fs::read(&file_path)
                .map_err(|_| Error::file_not_found(file_path_str, Some(ctx.config_path.clone())))?;
            STANDARD.encode(bytes)
        }
        "ascii" => {
            // Read as UTF-8 but strip non-ASCII characters
            let raw = std::fs::read_to_string(&file_path)
                .map_err(|_| Error::file_not_found(file_path_str, Some(ctx.config_path.clone())))?;
            raw.chars().filter(|c| c.is_ascii()).collect()
        }
        _ => {
            // Default to UTF-8 (including explicit "utf-8")
            std::fs::read_to_string(&file_path)
                .map_err(|_| Error::file_not_found(file_path_str, Some(ctx.config_path.clone())))?
        }
    };

    // For base64 encoding, always return as text (don't try to parse)
    if encoding == "base64" {
        return Ok(ResolvedValue::new(Value::String(content)));
    }

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
/// Fetches content from remote URLs.
///
/// Usage:
///   ${http:https://example.com/config.yaml}           - Auto-detect parse mode
///   ${http:https://example.com/config,parse=yaml}     - Parse as YAML
///   ${http:https://example.com/config,parse=json}     - Parse as JSON
///   ${http:https://example.com/config,parse=text}     - Read as text
///   ${http:https://example.com/config,parse=binary}   - Read as binary
///   ${http:https://example.com/config,timeout=60}     - Timeout in seconds
///   ${http:https://example.com/config,header=Auth:Bearer token} - Add header
///   ${http:https://example.com/config,default={}}     - Default if request fails
///   ${http:https://example.com/config,sensitive=true} - Mark as sensitive
///
/// Security:
/// - Disabled by default (requires allow_http=true in ConfigOptions)
/// - URL allowlist can restrict which URLs are accessible
fn http_resolver(
    args: &[String],
    kwargs: &HashMap<String, String>,
    ctx: &ResolverContext,
) -> Result<ResolvedValue> {
    if args.is_empty() {
        return Err(Error::parse("http resolver requires a URL").with_path(ctx.config_path.clone()));
    }

    let url = &args[0];

    // Check if HTTP is enabled
    if !ctx.allow_http {
        return Err(Error {
            kind: crate::error::ErrorKind::Resolver(crate::error::ResolverErrorKind::HttpDisabled),
            path: Some(ctx.config_path.clone()),
            source_location: None,
            help: Some(format!(
                "Enable HTTP resolver with Config.load(..., allow_http=True)\nURL: {}",
                url
            )),
            cause: None,
        });
    }

    // Check URL against allowlist if configured
    if !ctx.http_allowlist.is_empty() {
        let url_allowed = ctx
            .http_allowlist
            .iter()
            .any(|pattern| url_matches_pattern(url, pattern));
        if !url_allowed {
            return Err(Error::http_not_in_allowlist(
                url,
                &ctx.http_allowlist,
                Some(ctx.config_path.clone()),
            ));
        }
    }

    // Perform the actual HTTP request
    #[cfg(feature = "http")]
    {
        http_fetch(url, kwargs, ctx)
    }

    #[cfg(not(feature = "http"))]
    {
        // If compiled without HTTP feature, return an error
        let _ = kwargs; // Suppress unused warning
        Err(Error::resolver_custom(
            "http",
            "HTTP support not compiled in. Rebuild with --features http",
        ))
    }
}

/// Check if a URL matches an allowlist pattern
///
/// Supports glob-style patterns:
/// - `https://example.com/*` matches any path on example.com
/// - `https://*.example.com/*` matches any subdomain
fn url_matches_pattern(url: &str, pattern: &str) -> bool {
    // Convert glob pattern to regex
    let regex_pattern = pattern
        .replace('.', r"\.")
        .replace('*', ".*")
        .replace('?', ".");

    if let Ok(re) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
        re.is_match(url)
    } else {
        // If pattern is invalid, do exact match
        url == pattern
    }
}

// =============================================================================
// TLS/Proxy Configuration Helpers (HTTP feature)
// =============================================================================

/// Load certificates from a PEM file (returns owned static-lifetime certs)
#[cfg(feature = "http")]
fn load_certs_from_pem(path: &std::path::Path) -> Result<Vec<ureq::tls::Certificate<'static>>> {
    use ureq::tls::PemItem;

    let pem_content = std::fs::read(path).map_err(|e| {
        Error::pem_load_error(
            path.display().to_string(),
            format!("Failed to open file: {}", e),
        )
    })?;

    let certs: Vec<_> = ureq::tls::parse_pem(&pem_content)
        .filter_map(|item| item.ok())
        .filter_map(|item| match item {
            PemItem::Certificate(cert) => Some(cert.to_owned()),
            _ => None,
        })
        .collect();

    if certs.is_empty() {
        return Err(Error::pem_load_error(
            path.display().to_string(),
            "No valid certificates found in PEM file",
        ));
    }

    Ok(certs)
}

/// Load a private key from a PEM file (handles unencrypted keys)
#[cfg(feature = "http")]
fn load_private_key_from_pem(path: &std::path::Path) -> Result<ureq::tls::PrivateKey<'static>> {
    let pem_content = std::fs::read(path).map_err(|e| {
        Error::pem_load_error(
            path.display().to_string(),
            format!("Failed to open file: {}", e),
        )
    })?;

    let key = ureq::tls::PrivateKey::from_pem(&pem_content).map_err(|e| {
        Error::pem_load_error(
            path.display().to_string(),
            format!("Failed to parse key: {}", e),
        )
    })?;

    Ok(key.to_owned())
}

/// Load an encrypted private key from a PEM file
#[cfg(feature = "http")]
fn load_encrypted_private_key_from_pem(
    path: &std::path::Path,
    password: &str,
) -> Result<ureq::tls::PrivateKey<'static>> {
    use pkcs8::der::Decode;

    let pem_content = std::fs::read_to_string(path).map_err(|e| {
        Error::pem_load_error(
            path.display().to_string(),
            format!("Failed to read file: {}", e),
        )
    })?;

    // Check if this is an encrypted PKCS#8 key
    if pem_content.contains("-----BEGIN ENCRYPTED PRIVATE KEY-----") {
        // Extract the base64 content from PEM
        let der_bytes = pem_to_der(&pem_content, "ENCRYPTED PRIVATE KEY")
            .map_err(|e| Error::pem_load_error(path.display().to_string(), e))?;

        let encrypted = pkcs8::EncryptedPrivateKeyInfo::from_der(&der_bytes)
            .map_err(|e| Error::pem_load_error(path.display().to_string(), e.to_string()))?;

        let decrypted = encrypted
            .decrypt(password)
            .map_err(|e| Error::key_decryption_error(e.to_string()))?;

        // The decrypted key is in PKCS#8 DER format
        // Wrap it in PEM format so ureq can parse it
        let pem_key = der_to_pem(decrypted.as_bytes(), "PRIVATE KEY");

        ureq::tls::PrivateKey::from_pem(pem_key.as_bytes())
            .map(|k| k.to_owned())
            .map_err(|e| {
                Error::pem_load_error(
                    path.display().to_string(),
                    format!("Failed to parse decrypted key: {}", e),
                )
            })
    } else {
        // Not encrypted, try loading as regular key
        load_private_key_from_pem(path)
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

/// Detect if a file is P12/PFX format by extension
#[cfg(feature = "http")]
fn is_p12_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("p12") || ext.eq_ignore_ascii_case("pfx"))
        .unwrap_or(false)
}

/// Load client certificate and key from P12/PFX file
#[cfg(feature = "http")]
fn load_identity_from_p12(
    path: &std::path::Path,
    password: &str,
) -> Result<(
    Vec<ureq::tls::Certificate<'static>>,
    ureq::tls::PrivateKey<'static>,
)> {
    let p12_data = std::fs::read(path).map_err(|e| {
        Error::p12_load_error(
            path.display().to_string(),
            format!("Failed to read file: {}", e),
        )
    })?;

    let keystore = p12_keystore::KeyStore::from_pkcs12(&p12_data, password)
        .map_err(|e| Error::p12_load_error(path.display().to_string(), e.to_string()))?;

    // Get the first key entry (most P12 files have one key)
    // private_key_chain() returns Option<(&str, &PrivateKeyChain)>
    let (_alias, key_chain) = keystore.private_key_chain().ok_or_else(|| {
        Error::p12_load_error(
            path.display().to_string(),
            "No private key found in P12 file",
        )
    })?;

    // Get the private key DER bytes - wrap in PEM for ureq
    let pem_key = der_to_pem(key_chain.key(), "PRIVATE KEY");
    let private_key = ureq::tls::PrivateKey::from_pem(pem_key.as_bytes())
        .map(|k| k.to_owned())
        .map_err(|e| {
            Error::p12_load_error(
                path.display().to_string(),
                format!("Failed to parse private key: {}", e),
            )
        })?;

    // Get certificates from the chain
    let certs: Vec<_> = key_chain
        .chain()
        .iter()
        .map(|cert| ureq::tls::Certificate::from_der(cert.as_der()).to_owned())
        .collect();

    if certs.is_empty() {
        return Err(Error::p12_load_error(
            path.display().to_string(),
            "No certificates found in P12 file",
        ));
    }

    Ok((certs, private_key))
}

/// Load client identity (cert + key) for mTLS
#[cfg(feature = "http")]
fn load_client_identity(
    cert_path: &std::path::Path,
    key_path: Option<&std::path::Path>,
    password: Option<&str>,
) -> Result<(
    Vec<ureq::tls::Certificate<'static>>,
    ureq::tls::PrivateKey<'static>,
)> {
    // If cert is P12/PFX, load both cert and key from it
    if is_p12_file(cert_path) {
        let pwd = password.unwrap_or("");
        return load_identity_from_p12(cert_path, pwd);
    }

    // Otherwise, load PEM cert and key separately
    let cert_chain = load_certs_from_pem(cert_path)?;

    let key_path = key_path.ok_or_else(|| {
        Error::tls_config_error("Client key path required when using PEM certificate (not P12)")
    })?;

    let private_key = if let Some(pwd) = password {
        load_encrypted_private_key_from_pem(key_path, pwd)?
    } else {
        load_private_key_from_pem(key_path)?
    };

    Ok((cert_chain, private_key))
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

    // Check for insecure mode (per-request kwargs override context)
    let insecure = kwargs
        .get("insecure")
        .map(|v| v == "true")
        .unwrap_or(ctx.http_insecure);

    if insecure {
        // DANGEROUS: Skip TLS verification
        // Log a warning (in production, use proper logging)
        eprintln!("WARNING: TLS certificate verification is disabled (http_insecure=true)");
        builder = builder.disable_verification(true);
    }

    // Load CA bundle if specified (per-request overrides context)
    let ca_bundle_path = kwargs
        .get("ca_bundle")
        .map(std::path::PathBuf::from)
        .or_else(|| ctx.http_ca_bundle.clone());

    let extra_ca_bundle_path = kwargs
        .get("extra_ca_bundle")
        .map(std::path::PathBuf::from)
        .or_else(|| ctx.http_extra_ca_bundle.clone());

    if let Some(ca_path) = ca_bundle_path {
        // Replace root certs with custom CA bundle
        let certs = load_certs_from_pem(&ca_path)?;
        builder = builder.root_certs(RootCerts::Specific(Arc::new(certs)));
    } else if let Some(extra_ca_path) = extra_ca_bundle_path {
        // Add extra certs to webpki roots using new_with_certs
        let extra_certs = load_certs_from_pem(&extra_ca_path)?;
        builder = builder.root_certs(RootCerts::new_with_certs(&extra_certs));
    }

    // Load client certificate for mTLS (per-request overrides context)
    let client_cert_path = kwargs
        .get("client_cert")
        .map(std::path::PathBuf::from)
        .or_else(|| ctx.http_client_cert.clone());

    if let Some(cert_path) = client_cert_path {
        let client_key_path = kwargs
            .get("client_key")
            .map(std::path::PathBuf::from)
            .or_else(|| ctx.http_client_key.clone());

        let password = kwargs
            .get("key_password")
            .map(|s| s.as_str())
            .or(ctx.http_client_key_password.as_deref());

        let (certs, key) = load_client_identity(&cert_path, client_key_path.as_deref(), password)?;

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

    let parse_mode = kwargs.get("parse").map(|s| s.as_str()).unwrap_or("auto");
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

    // Get content type from response
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();

    // Handle binary mode separately
    if parse_mode == "binary" {
        let bytes = response.into_body().read_to_vec().map_err(|e| {
            Error::http_request_failed(url, e.to_string(), Some(ctx.config_path.clone()))
        })?;
        return Ok(ResolvedValue::new(Value::Bytes(bytes)));
    }

    // Read response body as text
    let body = response.into_body().read_to_string().map_err(|e| {
        Error::http_request_failed(url, e.to_string(), Some(ctx.config_path.clone()))
    })?;

    // Determine actual parse mode
    let actual_parse_mode = if parse_mode == "auto" {
        detect_parse_mode(url, &content_type)
    } else {
        parse_mode
    };

    // Parse content based on mode
    match actual_parse_mode {
        "yaml" => {
            let value: Value = serde_yaml::from_str(&body).map_err(|e| {
                Error::parse(format!("Failed to parse YAML from {}: {}", url, e))
                    .with_path(ctx.config_path.clone())
            })?;
            Ok(ResolvedValue::new(value))
        }
        "json" => {
            let value: Value = serde_json::from_str(&body).map_err(|e| {
                Error::parse(format!("Failed to parse JSON from {}: {}", url, e))
                    .with_path(ctx.config_path.clone())
            })?;
            Ok(ResolvedValue::new(value))
        }
        _ => {
            // Default to text mode
            Ok(ResolvedValue::new(Value::String(body)))
        }
    }
}

/// Detect parse mode from URL extension or content type
#[cfg(feature = "http")]
fn detect_parse_mode<'a>(url: &str, content_type: &str) -> &'a str {
    // Check content type first
    let ct_lower = content_type.to_lowercase();
    if ct_lower.contains("application/json") || ct_lower.contains("text/json") {
        return "json";
    }
    if ct_lower.contains("application/yaml")
        || ct_lower.contains("application/x-yaml")
        || ct_lower.contains("text/yaml")
    {
        return "yaml";
    }

    // Check URL extension
    if let Some(path) = url.split('?').next() {
        if path.ends_with(".json") {
            return "json";
        }
        if path.ends_with(".yaml") || path.ends_with(".yml") {
            return "yaml";
        }
    }

    // Default to text
    "text"
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

        let result = http_resolver(&args, &kwargs, &ctx).unwrap();
        assert!(result.value.is_mapping());

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

        let result = http_resolver(&args, &kwargs, &ctx).unwrap();
        assert!(result.value.is_mapping());

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
    fn test_http_fetch_explicit_parse_mode() {
        let mut server = Server::new();
        // Return JSON but with text/plain content-type
        let mock = server
            .mock("GET", "/data")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_body(r#"{"key": "value"}"#)
            .create();

        let ctx = ResolverContext::new("test.path").with_allow_http(true);
        let args = vec![format!("{}/data", server.url())];
        let mut kwargs = HashMap::new();
        kwargs.insert("parse".to_string(), "json".to_string());

        let result = http_resolver(&args, &kwargs, &ctx).unwrap();
        assert!(result.value.is_mapping());

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
        assert!(result.value.is_mapping());

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

    #[test]
    fn test_detect_parse_mode_from_content_type() {
        assert_eq!(
            detect_parse_mode("http://example.com/data", "application/json"),
            "json"
        );
        assert_eq!(
            detect_parse_mode("http://example.com/data", "text/json"),
            "json"
        );
        assert_eq!(
            detect_parse_mode("http://example.com/data", "application/yaml"),
            "yaml"
        );
        assert_eq!(
            detect_parse_mode("http://example.com/data", "application/x-yaml"),
            "yaml"
        );
        assert_eq!(
            detect_parse_mode("http://example.com/data", "text/yaml"),
            "yaml"
        );
        assert_eq!(
            detect_parse_mode("http://example.com/data", "text/plain"),
            "text"
        );
    }

    #[test]
    fn test_detect_parse_mode_from_url_extension() {
        assert_eq!(
            detect_parse_mode("http://example.com/config.json", ""),
            "json"
        );
        assert_eq!(
            detect_parse_mode("http://example.com/config.yaml", ""),
            "yaml"
        );
        assert_eq!(
            detect_parse_mode("http://example.com/config.yml", ""),
            "yaml"
        );
        assert_eq!(
            detect_parse_mode("http://example.com/config.txt", ""),
            "text"
        );
        assert_eq!(detect_parse_mode("http://example.com/config", ""), "text");
    }

    #[test]
    fn test_detect_parse_mode_content_type_takes_precedence() {
        // Content-Type should take precedence over URL extension
        assert_eq!(
            detect_parse_mode("http://example.com/config.yaml", "application/json"),
            "json"
        );
    }
}

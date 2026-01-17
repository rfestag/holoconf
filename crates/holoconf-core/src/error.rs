//! Error types for holoconf
//!
//! Error handling follows ADR-008: structured errors with context,
//! path information, and actionable help messages.

use std::fmt;

/// Result type alias for holoconf operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for holoconf operations
#[derive(Debug, Clone)]
pub struct Error {
    /// The kind of error that occurred
    pub kind: ErrorKind,
    /// Path in the config where the error occurred (e.g., "database.port")
    pub path: Option<String>,
    /// Source location (file, line) if available
    pub source_location: Option<SourceLocation>,
    /// Actionable help message
    pub help: Option<String>,
    /// Underlying cause (as string for Clone compatibility)
    pub cause: Option<String>,
}

/// Location in a source file
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub file: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

/// Categories of errors that can occur
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// Error parsing YAML/JSON
    Parse,
    /// Error during value resolution
    Resolver(ResolverErrorKind),
    /// Error during schema validation
    Validation,
    /// Error accessing a path that doesn't exist
    PathNotFound,
    /// Circular reference detected
    CircularReference,
    /// Type coercion failed
    TypeCoercion,
    /// I/O error (file not found, etc.)
    Io,
    /// Internal error (bug in holoconf)
    Internal,
}

/// Specific resolver error categories
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolverErrorKind {
    /// Resource not found (triggers default handling if default is provided)
    /// This is used when the resolver cannot find the requested resource
    /// (e.g., env var not set, file not found, SSM parameter missing)
    NotFound { resource: String },
    /// Environment variable not found
    EnvNotFound { var_name: String },
    /// File not found
    FileNotFound { path: String },
    /// HTTP request failed
    HttpError { url: String, status: Option<u16> },
    /// HTTP resolver is disabled
    HttpDisabled,
    /// URL not in allowlist
    HttpNotAllowed { url: String },
    /// TLS configuration error
    TlsConfigError { message: String },
    /// Proxy configuration error
    ProxyConfigError { message: String },
    /// PEM file loading error
    PemLoadError { path: String, message: String },
    /// P12/PFX file loading error
    P12LoadError { path: String, message: String },
    /// Key decryption error
    KeyDecryptionError { message: String },
    /// Referenced config path not found
    RefNotFound { ref_path: String },
    /// Unknown resolver
    UnknownResolver { name: String },
    /// Resolver returned an error
    Custom { resolver: String, message: String },
    /// Resolver already registered
    AlreadyRegistered { name: String },
}

impl Error {
    /// Create a new parse error
    pub fn parse(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Parse,
            path: None,
            source_location: None,
            help: None,
            cause: Some(message.into()),
        }
    }

    /// Create a path not found error
    pub fn path_not_found(path: impl Into<String>) -> Self {
        let path_str = path.into();
        Self {
            kind: ErrorKind::PathNotFound,
            path: Some(path_str.clone()),
            source_location: None,
            help: Some(format!(
                "Check that '{}' exists in the configuration",
                path_str
            )),
            cause: None,
        }
    }

    /// Create a circular reference error
    pub fn circular_reference(path: impl Into<String>, chain: Vec<String>) -> Self {
        let chain_str = chain.join(" → ");
        Self {
            kind: ErrorKind::CircularReference,
            path: Some(path.into()),
            source_location: None,
            help: Some("Break the circular dependency by removing one of the references".into()),
            cause: Some(format!("Chain: {}", chain_str)),
        }
    }

    /// Create a not found error (triggers default handling at framework level)
    pub fn not_found(resource: impl Into<String>, config_path: Option<String>) -> Self {
        let res = resource.into();
        Self {
            kind: ErrorKind::Resolver(ResolverErrorKind::NotFound { resource: res }),
            path: config_path,
            source_location: None,
            help: None,
            cause: None,
        }
    }

    /// Create an env var not found error
    pub fn env_not_found(var_name: impl Into<String>, config_path: Option<String>) -> Self {
        let var = var_name.into();
        Self {
            kind: ErrorKind::Resolver(ResolverErrorKind::EnvNotFound {
                var_name: var.clone(),
            }),
            path: config_path,
            source_location: None,
            help: Some(format!(
                "Set the {} environment variable or provide a default: ${{env:{},default=value}}",
                var, var
            )),
            cause: None,
        }
    }

    /// Create a reference not found error
    pub fn ref_not_found(ref_path: impl Into<String>, config_path: Option<String>) -> Self {
        let ref_p = ref_path.into();
        Self {
            kind: ErrorKind::Resolver(ResolverErrorKind::RefNotFound {
                ref_path: ref_p.clone(),
            }),
            path: config_path,
            source_location: None,
            help: Some(format!(
                "Check that '{}' exists in the configuration",
                ref_p
            )),
            cause: None,
        }
    }

    /// Create a file not found error
    pub fn file_not_found(file_path: impl Into<String>, config_path: Option<String>) -> Self {
        let fp = file_path.into();
        Self {
            kind: ErrorKind::Resolver(ResolverErrorKind::FileNotFound { path: fp.clone() }),
            path: config_path,
            source_location: None,
            help: Some("Check that the file exists relative to the config file".into()),
            cause: None,
        }
    }

    /// Create an unknown resolver error
    pub fn unknown_resolver(name: impl Into<String>, config_path: Option<String>) -> Self {
        let n = name.into();
        Self {
            kind: ErrorKind::Resolver(ResolverErrorKind::UnknownResolver { name: n.clone() }),
            path: config_path,
            source_location: None,
            help: Some(format!("Register the '{}' resolver or check for typos", n)),
            cause: None,
        }
    }

    /// Create a resolver already registered error
    pub fn resolver_already_registered(name: impl Into<String>) -> Self {
        let n = name.into();
        Self {
            kind: ErrorKind::Resolver(ResolverErrorKind::AlreadyRegistered { name: n.clone() }),
            path: None,
            source_location: None,
            help: Some(format!(
                "Use register_with_force(..., force=true) to override the '{}' resolver",
                n
            )),
            cause: None,
        }
    }

    /// Create a type coercion error
    pub fn type_coercion(
        path: impl Into<String>,
        expected: impl Into<String>,
        got: impl Into<String>,
    ) -> Self {
        Self {
            kind: ErrorKind::TypeCoercion,
            path: Some(path.into()),
            source_location: None,
            help: Some(format!(
                "Ensure the value can be converted to {}",
                expected.into()
            )),
            cause: Some(format!("Got: {}", got.into())),
        }
    }

    /// Create a validation error
    pub fn validation(path: impl Into<String>, message: impl Into<String>) -> Self {
        let p = path.into();
        Self {
            kind: ErrorKind::Validation,
            path: if p.is_empty() || p == "<root>" {
                None
            } else {
                Some(p)
            },
            source_location: None,
            help: Some("Fix the value to match the schema requirements".into()),
            cause: Some(message.into()),
        }
    }

    /// Add path context to the error
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Add source location to the error
    pub fn with_source_location(mut self, loc: SourceLocation) -> Self {
        self.source_location = Some(loc);
        self
    }

    /// Add help message to the error
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Create a custom resolver error
    pub fn resolver_custom(resolver: impl Into<String>, message: impl Into<String>) -> Self {
        let resolver_name = resolver.into();
        Self {
            kind: ErrorKind::Resolver(ResolverErrorKind::Custom {
                resolver: resolver_name.clone(),
                message: message.into(),
            }),
            path: None,
            source_location: None,
            help: Some(format!(
                "Check the '{}' resolver implementation",
                resolver_name
            )),
            cause: None,
        }
    }

    /// Create an HTTP request failed error
    pub fn http_request_failed(
        url: impl Into<String>,
        message: impl Into<String>,
        config_path: Option<String>,
    ) -> Self {
        let url_str = url.into();
        Self {
            kind: ErrorKind::Resolver(ResolverErrorKind::HttpError {
                url: url_str.clone(),
                status: None,
            }),
            path: config_path,
            source_location: None,
            help: Some(format!(
                "Check that the URL '{}' is accessible and returns valid content",
                url_str
            )),
            cause: Some(message.into()),
        }
    }

    /// Create an HTTP not in allowlist error
    pub fn http_not_in_allowlist(
        url: impl Into<String>,
        allowlist: &[String],
        config_path: Option<String>,
    ) -> Self {
        let url_str = url.into();
        let allowlist_str = if allowlist.is_empty() {
            "(empty)".to_string()
        } else {
            allowlist.join(", ")
        };
        Self {
            kind: ErrorKind::Resolver(ResolverErrorKind::HttpNotAllowed {
                url: url_str.clone(),
            }),
            path: config_path,
            source_location: None,
            help: Some(format!(
                "Add '{}' to the http_allowlist, or use a pattern that matches it.\nCurrent allowlist: {}",
                url_str, allowlist_str
            )),
            cause: None,
        }
    }

    /// Create a TLS configuration error
    pub fn tls_config_error(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Resolver(ResolverErrorKind::TlsConfigError {
                message: message.into(),
            }),
            path: None,
            source_location: None,
            help: Some("Check your TLS configuration (CA bundles, client certificates)".into()),
            cause: None,
        }
    }

    /// Create a proxy configuration error
    pub fn proxy_config_error(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Resolver(ResolverErrorKind::ProxyConfigError {
                message: message.into(),
            }),
            path: None,
            source_location: None,
            help: Some(
                "Check your proxy URL format (e.g., http://proxy:8080 or socks5://proxy:1080)"
                    .into(),
            ),
            cause: None,
        }
    }

    /// Create a PEM file loading error
    pub fn pem_load_error(path: impl Into<String>, message: impl Into<String>) -> Self {
        let path_str = path.into();
        Self {
            kind: ErrorKind::Resolver(ResolverErrorKind::PemLoadError {
                path: path_str.clone(),
                message: message.into(),
            }),
            path: None,
            source_location: None,
            help: Some(format!(
                "Ensure '{}' exists and contains valid PEM-encoded data",
                path_str
            )),
            cause: None,
        }
    }

    /// Create a P12/PFX file loading error
    pub fn p12_load_error(path: impl Into<String>, message: impl Into<String>) -> Self {
        let path_str = path.into();
        Self {
            kind: ErrorKind::Resolver(ResolverErrorKind::P12LoadError {
                path: path_str.clone(),
                message: message.into(),
            }),
            path: None,
            source_location: None,
            help: Some(format!(
                "Ensure '{}' exists and contains a valid PKCS#12/PFX bundle with the correct password",
                path_str
            )),
            cause: None,
        }
    }

    /// Create a key decryption error
    pub fn key_decryption_error(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Resolver(ResolverErrorKind::KeyDecryptionError {
                message: message.into(),
            }),
            path: None,
            source_location: None,
            help: Some("Check that the password is correct for the encrypted private key".into()),
            cause: None,
        }
    }

    /// Create an internal error (bug in holoconf)
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Internal,
            path: None,
            source_location: None,
            help: Some("This is likely a bug in holoconf. Please report it.".into()),
            cause: Some(message.into()),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Main error message
        match &self.kind {
            ErrorKind::Parse => write!(f, "Parse error")?,
            ErrorKind::Resolver(r) => match r {
                ResolverErrorKind::NotFound { resource } => {
                    write!(f, "Resource not found: {}", resource)?
                }
                ResolverErrorKind::EnvNotFound { var_name } => {
                    write!(f, "Environment variable not found: {}", var_name)?
                }
                ResolverErrorKind::FileNotFound { path } => write!(f, "File not found: {}", path)?,
                ResolverErrorKind::HttpError { url, status } => {
                    write!(f, "HTTP request failed: {}", url)?;
                    if let Some(s) = status {
                        write!(f, " (status {})", s)?;
                    }
                }
                ResolverErrorKind::HttpDisabled => write!(f, "HTTP resolver is disabled")?,
                ResolverErrorKind::HttpNotAllowed { url } => {
                    write!(f, "URL not in allowlist: {}", url)?
                }
                ResolverErrorKind::RefNotFound { ref_path } => {
                    write!(f, "Referenced path not found: {}", ref_path)?
                }
                ResolverErrorKind::UnknownResolver { name } => {
                    write!(f, "Unknown resolver: {}", name)?
                }
                ResolverErrorKind::Custom { resolver, message } => {
                    write!(f, "Resolver '{}' error: {}", resolver, message)?
                }
                ResolverErrorKind::AlreadyRegistered { name } => {
                    write!(f, "Resolver '{}' is already registered", name)?
                }
                ResolverErrorKind::TlsConfigError { message } => {
                    write!(f, "TLS configuration error: {}", message)?
                }
                ResolverErrorKind::ProxyConfigError { message } => {
                    write!(f, "Proxy configuration error: {}", message)?
                }
                ResolverErrorKind::PemLoadError { path, message } => {
                    write!(f, "Failed to load PEM file '{}': {}", path, message)?
                }
                ResolverErrorKind::P12LoadError { path, message } => {
                    write!(f, "Failed to load P12/PFX file '{}': {}", path, message)?
                }
                ResolverErrorKind::KeyDecryptionError { message } => {
                    write!(f, "Failed to decrypt private key: {}", message)?
                }
            },
            ErrorKind::Validation => write!(f, "Validation error")?,
            ErrorKind::PathNotFound => write!(f, "Path not found")?,
            ErrorKind::CircularReference => write!(f, "Circular reference detected")?,
            ErrorKind::TypeCoercion => write!(f, "Type coercion failed")?,
            ErrorKind::Io => write!(f, "I/O error")?,
            ErrorKind::Internal => write!(f, "Internal error")?,
        }

        // Path context
        if let Some(path) = &self.path {
            write!(f, "\n  Path: {}", path)?;
        }

        // Source location
        if let Some(loc) = &self.source_location {
            write!(f, "\n  File: {}", loc.file)?;
            if let Some(line) = loc.line {
                write!(f, ":{}", line)?;
            }
        }

        // Cause
        if let Some(cause) = &self.cause {
            write!(f, "\n  {}", cause)?;
        }

        // Help
        if let Some(help) = &self.help {
            write!(f, "\n  Help: {}", help)?;
        }

        Ok(())
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_not_found_error_display() {
        let err = Error::env_not_found("MY_VAR", Some("database.password".into()));
        let display = format!("{}", err);

        assert!(display.contains("Environment variable not found: MY_VAR"));
        assert!(display.contains("Path: database.password"));
        assert!(display.contains("Help:"));
        assert!(display.contains("${env:MY_VAR,default=value}"));
    }

    #[test]
    fn test_circular_reference_error_display() {
        let err = Error::circular_reference(
            "config.a",
            vec!["a".into(), "b".into(), "c".into(), "a".into()],
        );
        let display = format!("{}", err);

        assert!(display.contains("Circular reference detected"));
        assert!(display.contains("a → b → c → a"));
    }

    #[test]
    fn test_path_not_found_error() {
        let err = Error::path_not_found("database.host");

        assert_eq!(err.kind, ErrorKind::PathNotFound);
        assert_eq!(err.path, Some("database.host".into()));
    }

    #[test]
    fn test_not_found_error() {
        let err = Error::not_found("my-resource", Some("config.key".into()));
        let display = format!("{}", err);

        assert!(display.contains("Resource not found: my-resource"));
        assert!(display.contains("Path: config.key"));
        assert!(matches!(
            err.kind,
            ErrorKind::Resolver(ResolverErrorKind::NotFound { .. })
        ));
    }

    #[test]
    fn test_ref_not_found_error() {
        let err = Error::ref_not_found("database.missing", Some("app.db".into()));
        let display = format!("{}", err);

        assert!(display.contains("Referenced path not found: database.missing"));
        assert!(display.contains("Path: app.db"));
        assert!(display.contains("Help:"));
    }

    #[test]
    fn test_file_not_found_error() {
        let err = Error::file_not_found("/path/to/missing.yaml", Some("config.file".into()));
        let display = format!("{}", err);

        assert!(display.contains("File not found: /path/to/missing.yaml"));
        assert!(display.contains("Path: config.file"));
    }

    #[test]
    fn test_unknown_resolver_error() {
        let err = Error::unknown_resolver("unknown", Some("config.value".into()));
        let display = format!("{}", err);

        assert!(display.contains("Unknown resolver: unknown"));
        assert!(display.contains("Help:"));
        assert!(display.contains("Register the 'unknown' resolver"));
    }

    #[test]
    fn test_resolver_custom_error() {
        let err = Error::resolver_custom("myresolver", "Something went wrong");
        let display = format!("{}", err);

        assert!(display.contains("Resolver 'myresolver' error: Something went wrong"));
        assert!(display.contains("Help:"));
    }

    #[test]
    fn test_internal_error() {
        let err = Error::internal("Unexpected state");
        let display = format!("{}", err);

        assert!(display.contains("Internal error"));
        // The message goes into the cause field
        assert!(display.contains("Unexpected state"));
    }

    #[test]
    fn test_with_source_location() {
        let err = Error::parse("syntax error").with_source_location(SourceLocation {
            file: "config.yaml".into(),
            line: Some(42),
            column: None,
        });
        let display = format!("{}", err);

        assert!(display.contains("config.yaml:42"));
    }

    #[test]
    fn test_with_help() {
        let err = Error::parse("bad input").with_help("Try fixing the syntax");
        let display = format!("{}", err);

        assert!(display.contains("Help: Try fixing the syntax"));
    }

    #[test]
    fn test_type_coercion_error() {
        let err = Error::type_coercion("server.port", "integer", "string");
        let display = format!("{}", err);

        assert!(display.contains("Type coercion failed"));
        assert!(display.contains("Path: server.port"));
        assert!(display.contains("Got: string"));
    }

    #[test]
    fn test_validation_error() {
        let err = Error::validation("users[0].name", "must be at least 3 characters");
        let display = format!("{}", err);

        assert!(display.contains("Validation error"));
        assert!(display.contains("Path: users[0].name"));
        assert!(display.contains("must be at least 3 characters"));
    }

    #[test]
    fn test_validation_error_root_path() {
        // Root path should not show path field
        let err = Error::validation("<root>", "missing required field");
        assert!(err.path.is_none());

        let err2 = Error::validation("", "missing required field");
        assert!(err2.path.is_none());
    }

    #[test]
    fn test_http_error_display() {
        let err = Error {
            kind: ErrorKind::Resolver(ResolverErrorKind::HttpError {
                url: "https://example.com/config".into(),
                status: Some(404),
            }),
            path: Some("remote.config".into()),
            source_location: None,
            help: None,
            cause: None,
        };
        let display = format!("{}", err);

        assert!(display.contains("HTTP request failed: https://example.com/config"));
        assert!(display.contains("status 404"));
    }

    #[test]
    fn test_http_disabled_error() {
        let err = Error {
            kind: ErrorKind::Resolver(ResolverErrorKind::HttpDisabled),
            path: Some("remote.config".into()),
            source_location: None,
            help: None,
            cause: None,
        };
        let display = format!("{}", err);

        assert!(display.contains("HTTP resolver is disabled"));
    }
}

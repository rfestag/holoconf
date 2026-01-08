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
    /// Referenced config path not found
    RefNotFound { ref_path: String },
    /// Unknown resolver
    UnknownResolver { name: String },
    /// Resolver returned an error
    Custom { resolver: String, message: String },
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
                "Set the {} environment variable or provide a default: ${{env:{},default}}",
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
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Main error message
        match &self.kind {
            ErrorKind::Parse => write!(f, "Parse error")?,
            ErrorKind::Resolver(r) => match r {
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
        assert!(display.contains("${env:MY_VAR,default}"));
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
}

//! holoconf-core: Configuration library with resolver support
//!
//! This crate provides the core functionality for loading, parsing, and resolving
//! configuration files with interpolation support.
//!
//! # Example
//!
//! ```rust
//! use holoconf_core::Config;
//!
//! let yaml = r#"
//! database:
//!   host: localhost
//!   port: 5432
//! "#;
//!
//! let config = Config::from_yaml(yaml).unwrap();
//! assert_eq!(config.get("database.host").unwrap().as_str(), Some("localhost"));
//! ```

pub mod error;
pub mod interpolation;
pub mod resolver;
pub mod schema;
pub mod value;

mod config;

pub use config::{Config, ConfigOptions};
pub use error::{Error, Result};
pub use resolver::{ResolvedValue, Resolver, ResolverRegistry};
pub use schema::Schema;
pub use value::Value;

# Error

Structured error type with variants for each error kind.

## Overview

`Error` provides detailed error information including:

- The kind of error that occurred
- The configuration path where it happened (when applicable)
- A human-readable message with context
- Source error chain for debugging

## Error Kinds

```rust
pub enum ErrorKind {
    /// YAML or JSON syntax error
    Parse,

    /// Requested path doesn't exist
    PathNotFound,

    /// Resolver failed (env var not found, HTTP error, etc.)
    Resolver(ResolverKind),

    /// Schema validation failed
    Validation,

    /// Circular reference detected (a -> b -> a)
    CircularReference,

    /// Cannot convert value to requested type
    TypeCoercion,

    /// File I/O error
    Io,
}
```

## Handling Errors

### Pattern Matching

```rust
use holoconf_core::{Config, Error, error::ErrorKind};

let result = Config::from_yaml_file("config.yaml");

match result {
    Ok(config) => { /* use config */ }
    Err(e) => match e.kind() {
        ErrorKind::Parse => {
            eprintln!("Invalid YAML/JSON: {}", e);
        }
        ErrorKind::Io => {
            eprintln!("File error: {}", e);
        }
        _ => {
            eprintln!("Error: {}", e);
        }
    }
}
```

### Resolver Errors

```rust
use holoconf_core::error::{ErrorKind, ResolverKind};

match e.kind() {
    ErrorKind::Resolver(ResolverKind::Env) => {
        eprintln!("Environment variable not found: {}", e);
    }
    ErrorKind::Resolver(ResolverKind::File) => {
        eprintln!("Include file not found: {}", e);
    }
    ErrorKind::Resolver(ResolverKind::Http) => {
        eprintln!("HTTP request failed: {}", e);
    }
    _ => {}
}
```

## Error Context

### Path Information

```rust
if let Some(path) = e.path() {
    eprintln!("Error at path '{}': {}", path, e.message());
}
```

### Source Chain

```rust
// Print full error chain
let mut current: Option<&dyn std::error::Error> = Some(&e);
while let Some(err) = current {
    eprintln!("  Caused by: {}", err);
    current = err.source();
}
```

## Converting to Result

```rust
use holoconf_core::{Config, Error};

fn load_config() -> Result<Config, Error> {
    let config = Config::from_yaml_file("config.yaml")?;
    config.validate(&schema)?;
    Ok(config)
}
```

## Display and Debug

```rust
let e: Error = /* ... */;

// User-friendly message
println!("{}", e);
// e.g., "Path 'database.host' not found"

// Debug representation with full context
println!("{:?}", e);
// e.g., Error { kind: PathNotFound, path: Some("database.host"), ... }
```

## Common Patterns

### Graceful Fallback

```rust
use holoconf_core::error::ErrorKind;

let port = match config.get_i64("database.port") {
    Ok(p) => p,
    Err(e) if matches!(e.kind(), ErrorKind::PathNotFound) => 5432, // default
    Err(e) => return Err(e),
};
```

### Collecting All Errors

```rust
let paths = ["database.host", "database.port", "app.name"];
let mut errors = Vec::new();

for path in &paths {
    if let Err(e) = config.get_string(path) {
        errors.push(format!("{}: {}", path, e));
    }
}

if !errors.is_empty() {
    eprintln!("Configuration errors:\n{}", errors.join("\n"));
}
```

## API Reference

📚 **[Full rustdoc on docs.rs](https://docs.rs/holoconf-core/latest/holoconf_core/struct.Error.html)**

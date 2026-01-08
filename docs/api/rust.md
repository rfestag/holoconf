# Rust API Reference

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
holoconf = "0.1"
```

## Config Struct

The main entry point for loading and accessing configuration.

### Loading Configuration

```rust
use holoconf::Config;

// From a file
let config = Config::from_file("config.yaml")?;

// From a string
let config = Config::from_str(r#"
database:
  host: localhost
  port: 5432
"#)?;

// From multiple files (merged in order)
let config = Config::from_files(&["base.yaml", "override.yaml"])?;
```

### Accessing Values

```rust
use holoconf::Config;

let config = Config::from_file("config.yaml")?;

// Get a value with type inference
let host: String = config.get("database.host")?;
let port: i64 = config.get("database.port")?;
let debug: bool = config.get("app.debug")?;

// Get a subsection
let db_section = config.get_section("database")?;

// Check if path exists
if config.has("database.host") {
    // ...
}
```

### Validation

```rust
use holoconf::{Config, Error};

let config = Config::from_file("config.yaml")?;

match config.validate("schema.json") {
    Ok(()) => println!("Configuration is valid"),
    Err(Error::ValidationError { path, message, .. }) => {
        eprintln!("Invalid at {}: {}", path, message);
    }
    Err(e) => return Err(e),
}
```

### Serialization

```rust
use holoconf::{Config, Format};

let config = Config::from_file("config.yaml")?;

// Export to YAML
let yaml = config.dump(Format::Yaml)?;

// Export to JSON
let json = config.dump(Format::Json)?;

// Export with resolution
let resolved = config.dump_resolved(Format::Yaml)?;

// Export with redaction
let safe = config.dump_redacted(Format::Yaml, &["password", "secret"])?;
```

## Error Enum

All errors are represented by the `Error` enum:

```rust
use holoconf::Error;

match result {
    Err(Error::ParseError { message, line, column, .. }) => {
        eprintln!("Parse error at {}:{}: {}", line, column, message);
    }
    Err(Error::PathNotFound { path, .. }) => {
        eprintln!("Path not found: {}", path);
    }
    Err(Error::ResolverError { resolver, message, .. }) => {
        eprintln!("Resolver '{}' failed: {}", resolver, message);
    }
    Err(Error::ValidationError { path, message, .. }) => {
        eprintln!("Validation failed at {}: {}", path, message);
    }
    Err(Error::CircularReference { path, .. }) => {
        eprintln!("Circular reference at: {}", path);
    }
    Err(Error::TypeCoercionError { path, expected, actual, .. }) => {
        eprintln!("Type error at {}: expected {}, got {}", path, expected, actual);
    }
    Err(e) => eprintln!("Other error: {}", e),
    Ok(value) => { /* use value */ }
}
```

## Value Enum

Raw configuration values are represented by the `Value` enum:

```rust
use holoconf::Value;

let value = config.get_raw("some.path")?;

match value {
    Value::Null => println!("null"),
    Value::Bool(b) => println!("bool: {}", b),
    Value::Integer(i) => println!("int: {}", i),
    Value::Float(f) => println!("float: {}", f),
    Value::String(s) => println!("string: {}", s),
    Value::Sequence(arr) => println!("array of {} items", arr.len()),
    Value::Mapping(map) => println!("map with {} keys", map.len()),
}
```

## Type Conversions

When using the generic `get<T>()` method, holoconf converts values:

| YAML Type | Rust Types |
|-----------|------------|
| string | `String`, `&str` |
| integer | `i64`, `i32`, `u64`, `u32`, etc. |
| float | `f64`, `f32` |
| boolean | `bool` |
| null | `Option<T>` |
| sequence | `Vec<T>` |
| mapping | `HashMap<String, T>` |

## Thread Safety

`Config` is `Send + Sync` and can be safely shared across threads:

```rust
use holoconf::Config;
use std::sync::Arc;
use std::thread;

let config = Arc::new(Config::from_file("config.yaml")?);

let handles: Vec<_> = (0..4).map(|_| {
    let config = Arc::clone(&config);
    thread::spawn(move || {
        let host: String = config.get("database.host").unwrap();
        println!("Host: {}", host);
    })
}).collect();

for handle in handles {
    handle.join().unwrap();
}
```

See [ADR-010 Thread Safety](../adr/ADR-010-thread-safety.md) for implementation details.

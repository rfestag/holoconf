# Config

The main configuration container with lazy resolution.

## Overview

`Config` is the primary type for loading and accessing configuration values. It supports:

- Loading from YAML and JSON files or strings
- Merging multiple configuration sources
- Lazy resolution of interpolations (cached for performance)
- Thread-safe access (`Send + Sync`)

## Creating a Config

### From Files

```rust
use holoconf_core::Config;

// From a single file
let config = Config::from_yaml_file("config.yaml")?;
let config = Config::from_json_file("config.json")?;

// Merge multiple files (later files override earlier)
let config = Config::load_merged(&["base.yaml", "production.yaml"])?;
```

### From Strings

```rust
use holoconf_core::Config;

let yaml = r#"
database:
  host: localhost
  port: 5432
"#;

let config = Config::from_yaml(yaml)?;

let json = r#"{"database": {"host": "localhost"}}"#;
let config = Config::from_json(json)?;
```

### With Options

```rust
use holoconf_core::{Config, ConfigOptions};

let mut options = ConfigOptions::default();
options.allow_http = true;  // Enable HTTP resolver

let config = Config::from_yaml_with_options(yaml_str, options)?;
```

## Accessing Values

### Typed Getters

```rust
// String values
let host: String = config.get_string("database.host")?;

// Numeric values
let port: i64 = config.get_i64("database.port")?;
let timeout: f64 = config.get_f64("server.timeout")?;

// Boolean values
let enabled: bool = config.get_bool("feature.enabled")?;
```

### Raw Values

Access values before type conversion:

```rust
use holoconf_core::Value;

let value = config.get_raw("some.path")?;

match value {
    Value::String(s) => println!("string: {}", s),
    Value::Integer(i) => println!("int: {}", i),
    Value::Sequence(arr) => println!("array: {} items", arr.len()),
    Value::Mapping(map) => println!("map: {} keys", map.len()),
    _ => {}
}
```

### Nested Paths

Use dot notation to access nested values:

```rust
// config.yaml:
// database:
//   connection:
//     host: localhost

let host = config.get_string("database.connection.host")?;
```

## Merging Configurations

```rust
use holoconf_core::Config;

// Merge at load time
let config = Config::load_merged(&["base.yaml", "env.yaml", "local.yaml"])?;

// Or merge programmatically
let base = Config::from_yaml_file("base.yaml")?;
let overlay = Config::from_yaml_file("production.yaml")?;
let merged = base.merge(&overlay)?;
```

## Validation

```rust
use holoconf_core::{Config, Schema};

let config = Config::from_yaml_file("config.yaml")?;
let schema = Schema::from_file("schema.json")?;

// Validate resolved values
config.validate(&schema)?;

// Validate raw structure (before resolution)
config.validate_raw(&schema)?;
```

## Thread Safety

`Config` is `Send + Sync` with interior mutability for the resolution cache:

```rust
use holoconf_core::Config;
use std::sync::Arc;
use std::thread;

let config = Arc::new(Config::from_yaml_file("config.yaml")?);

let handles: Vec<_> = (0..4).map(|_| {
    let config = Arc::clone(&config);
    thread::spawn(move || {
        let host = config.get_string("database.host").unwrap();
        println!("Host: {}", host);
    })
}).collect();

for handle in handles {
    handle.join().unwrap();
}
```

## API Reference

📚 **[Full rustdoc on docs.rs](https://docs.rs/holoconf-core/latest/holoconf_core/struct.Config.html)**

# Value

Raw configuration values before type conversion.

## Overview

`Value` represents a configuration value in its raw form, before being converted to a specific Rust type. Use it when you need to:

- Inspect the type of a value before conversion
- Handle dynamic configuration structures
- Work with values that could be multiple types

## Variants

```rust
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Sequence(Vec<Value>),
    Mapping(IndexMap<String, Value>),
}
```

## Accessing Raw Values

```rust
use holoconf_core::{Config, Value};

let config = Config::from_yaml(r#"
database:
  host: localhost
  port: 5432
  replicas:
    - host: replica1
    - host: replica2
  settings:
    timeout: 30
    ssl: true
"#)?;

let value = config.get_raw("database")?;
```

## Pattern Matching

```rust
use holoconf_core::Value;

match config.get_raw("some.path")? {
    Value::Null => println!("null value"),
    Value::Bool(b) => println!("boolean: {}", b),
    Value::Integer(i) => println!("integer: {}", i),
    Value::Float(f) => println!("float: {}", f),
    Value::String(s) => println!("string: {}", s),
    Value::Sequence(arr) => {
        println!("array with {} items", arr.len());
        for item in arr {
            println!("  - {:?}", item);
        }
    }
    Value::Mapping(map) => {
        println!("map with {} keys", map.len());
        for (key, val) in map {
            println!("  {}: {:?}", key, val);
        }
    }
}
```

## Type Checking Methods

```rust
let value = config.get_raw("database.port")?;

if value.is_integer() {
    let port = value.as_i64().unwrap();
    println!("Port: {}", port);
}

// Available methods:
// - is_null(), as_null()
// - is_bool(), as_bool()
// - is_integer(), as_i64()
// - is_float(), as_f64()
// - is_string(), as_str()
// - is_sequence(), as_sequence()
// - is_mapping(), as_mapping()
```

## Converting to Typed Values

```rust
use holoconf_core::Value;

let value = config.get_raw("database.port")?;

// Try to convert to a specific type
match value {
    Value::Integer(i) => {
        let port: u16 = i.try_into()?;
        println!("Port: {}", port);
    }
    Value::String(s) => {
        // Handle string that might be a number
        let port: u16 = s.parse()?;
        println!("Port: {}", port);
    }
    _ => return Err("Expected integer or string".into()),
}
```

## Working with Sequences

```rust
let replicas = config.get_raw("database.replicas")?;

if let Value::Sequence(items) = replicas {
    for item in items {
        if let Value::Mapping(map) = item {
            if let Some(Value::String(host)) = map.get("host") {
                println!("Replica: {}", host);
            }
        }
    }
}
```

## Working with Mappings

```rust
use indexmap::IndexMap;

let settings = config.get_raw("database.settings")?;

if let Value::Mapping(map) = settings {
    for (key, value) in &map {
        println!("{}: {:?}", key, value);
    }
}
```

## API Reference

📚 **[Full rustdoc on docs.rs](https://docs.rs/holoconf-core/latest/holoconf_core/enum.Value.html)**

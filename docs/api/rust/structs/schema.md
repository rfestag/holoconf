# Schema

JSON Schema validation for configuration files.

## Overview

`Schema` provides validation against JSON Schema specifications. Use it to:

- Ensure configuration structure matches expectations
- Validate data types and constraints
- Provide helpful error messages for invalid configurations

## Creating a Schema

### From Files

```rust
use holoconf_core::Schema;

// From JSON Schema file
let schema = Schema::from_file("schema.json")?;
```

### From Strings

```rust
use holoconf_core::Schema;

let schema_json = r#"{
    "type": "object",
    "properties": {
        "database": {
            "type": "object",
            "properties": {
                "host": { "type": "string" },
                "port": { "type": "integer", "minimum": 1, "maximum": 65535 }
            },
            "required": ["host"]
        }
    }
}"#;

let schema = Schema::from_json(schema_json)?;

// Also supports YAML schemas
let schema = Schema::from_yaml(yaml_str)?;
```

## Validating Configuration

### Validate Resolved Values

```rust
use holoconf_core::{Config, Schema};

let config = Config::from_yaml_file("config.yaml")?;
let schema = Schema::from_file("schema.json")?;

// Validates after resolving all interpolations
match config.validate(&schema) {
    Ok(()) => println!("Configuration is valid"),
    Err(e) => eprintln!("Validation failed: {}", e),
}
```

### Validate Raw Structure

```rust
// Validates structure without resolving interpolations
// Useful for checking config shape before deployment
config.validate_raw(&schema)?;
```

## Example Schema

```json
{
    "$schema": "http://json-schema.org/draft-07/schema#",
    "type": "object",
    "required": ["app", "database"],
    "properties": {
        "app": {
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": {
                    "type": "string",
                    "minLength": 1
                },
                "debug": {
                    "type": "boolean",
                    "default": false
                }
            }
        },
        "database": {
            "type": "object",
            "required": ["host", "port"],
            "properties": {
                "host": { "type": "string" },
                "port": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 65535
                },
                "ssl": {
                    "type": "boolean",
                    "default": true
                }
            }
        }
    },
    "additionalProperties": false
}
```

## Error Messages

Validation errors include the path to the invalid value:

```rust
match config.validate(&schema) {
    Err(e) => {
        // e.g., "database.port: -1 is less than the minimum of 1"
        eprintln!("{}", e);
    }
    Ok(()) => {}
}
```

## API Reference

📚 **[Full rustdoc on docs.rs](https://docs.rs/holoconf-core/latest/holoconf_core/struct.Schema.html)**

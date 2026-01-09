# Schema Validation

!!! note "Coming Soon"
    This page is under construction. See [FEAT-004 Schema Validation](../specs/features/FEAT-004-schema-validation.md) for the full specification.

## Overview

HoloConf supports validating configuration against JSON Schema, helping you catch configuration errors early and enforce structure requirements.

## Basic Validation

=== "Python"

    ```python
    from holoconf import Config, ValidationError

    config = Config.from_file("config.yaml")

    try:
        config.validate("schema.json")
        print("Configuration is valid")
    except ValidationError as e:
        print(f"Validation failed: {e}")
    ```

=== "Rust"

    ```rust
    use holoconf::{Config, Error};

    fn main() -> Result<(), Error> {
        let config = Config::from_file("config.yaml")?;

        config.validate("schema.json")?;
        println!("Configuration is valid");

        Ok(())
    }
    ```

=== "CLI"

    ```bash
    holoconf validate config.yaml --schema schema.json
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
        "name": { "type": "string" },
        "debug": { "type": "boolean", "default": false }
      }
    },
    "database": {
      "type": "object",
      "required": ["host"],
      "properties": {
        "host": { "type": "string" },
        "port": { "type": "integer", "minimum": 1, "maximum": 65535 },
        "pool_size": { "type": "integer", "minimum": 1, "default": 10 }
      }
    }
  }
}
```

## Type Coercion

When validation is enabled, HoloConf can automatically coerce values to match the schema:

```yaml
# config.yaml
database:
  port: "5432"  # String in YAML
```

With schema validation, `database.port` will be coerced to integer `5432` based on the schema definition.

See [ADR-012 Type Coercion](../adr/ADR-012-type-coercion.md) for details on type coercion behavior.

## Validation Errors

Validation errors include detailed information about what failed:

=== "Python"

    ```python
    from holoconf import Config, ValidationError

    try:
        config = Config.from_file("config.yaml")
        config.validate("schema.json")
    except ValidationError as e:
        print(f"Path: {e.path}")
        print(f"Message: {e.message}")
        # Path: database.port
        # Message: -1 is less than the minimum of 1
    ```

=== "CLI"

    ```bash
    $ holoconf validate config.yaml --schema schema.json
    Validation error at 'database.port': -1 is less than the minimum of 1
    ```

See [ADR-007 Schema Validation](../adr/ADR-007-schema-validation.md) for the design rationale.

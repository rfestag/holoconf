# Schema Validation

HoloConf supports validating configuration against JSON Schema, helping you catch configuration errors early, enforce structure requirements, and apply default values for missing fields.

## Overview

Schemas serve two purposes in HoloConf:

1. **Validation**: Verify your configuration matches the expected structure and types
2. **Default Values**: Automatically provide values for missing configuration paths

## Loading with Schema

=== "Python"

    ```python
    from holoconf import Config

    # Load config with schema attached for default values
    config = Config.load("config.yaml", schema="schema.yaml")

    # Access a value that might be in config or schema default
    print(config.database.pool_size)  # Returns schema default if not in config
    ```

=== "CLI"

    ```bash
    # Get a value with schema defaults
    holoconf get config.yaml database.pool_size --schema schema.yaml

    # Dump config with schema defaults applied
    holoconf dump config.yaml --schema schema.yaml --resolve
    ```

## Explicit Validation

Attaching a schema for defaults does **not** automatically validate. Use `validate()` to check:

=== "Python"

    ```python
    from holoconf import Config, Schema, ValidationError

    config = Config.load("config.yaml", schema="schema.yaml")

    try:
        config.validate()  # Uses attached schema
        print("Configuration is valid")
    except ValidationError as e:
        print(f"Validation failed: {e}")

    # Or validate with a different schema
    other_schema = Schema.load("other.yaml")
    config.validate(other_schema)
    ```

=== "CLI"

    ```bash
    holoconf validate config.yaml --schema schema.yaml
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

## Schema Default Values

When a schema is attached to a config, accessing a missing path returns the schema default instead of raising `PathNotFoundError`:

=== "Python"

    ```python
    from holoconf import Config, Schema

    config = Config.load("config.yaml", schema="schema.yaml")

    # If pool_size is not in config.yaml but schema has default: 10
    print(config.database.pool_size)  # Returns 10

    # You can also attach a schema after loading
    config = Config.load("config.yaml")
    schema = Schema.load("schema.yaml")
    config.set_schema(schema)
    ```

### Value Precedence

When accessing a value, the precedence is:

1. **Config value**: If the path exists in the config, that value is used
2. **Resolver default**: If using `${env:VAR,default=value}`, the resolver default
3. **Schema default**: If the path is missing and schema has a default

```yaml
# schema.yaml
type: object
properties:
  database:
    type: object
    properties:
      pool_size:
        type: integer
        default: 10
```

```yaml
# config.yaml
database:
  pool_size: 20  # Config wins - returns 20
```

### Null Handling

If a config value is explicitly `null` and the schema doesn't allow null for that field, the schema default is used:

```yaml
# schema.yaml
properties:
  timeout:
    type: integer  # null not allowed
    default: 30
```

```yaml
# config.yaml
timeout: null  # null not allowed by schema
```

```python
config.timeout  # Returns 30 (schema default)
```

If the schema allows null (using `type: ["integer", "null"]`), the null value is preserved.

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

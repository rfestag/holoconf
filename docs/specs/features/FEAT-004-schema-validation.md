# FEAT-004: Schema Validation

## Overview

Validate configuration against a JSON Schema to catch errors early, document the expected structure, and enable type coercion for resolved values.

## User Stories

- As a developer, I want to define what my config should look like so mistakes are caught early
- As a developer, I want helpful error messages when config doesn't match the schema
- As a developer, I want environment variables (strings) automatically converted to the right types
- As a developer, I want to document my config structure in a standard format

## Dependencies

- [ADR-007: Schema and Validation](../../adr/ADR-007-schema-validation.md)
- [ADR-012: Type Coercion](../../adr/ADR-012-type-coercion.md)
- [FEAT-001: Configuration File Loading](FEAT-001-config-loading.md)

## API Surface

### Loading with Schema

```python
from holoconf import Config

# Load with schema validation
config = Config.load(
    "config.yaml",
    schema="schema.yaml"
)

# Schema can be JSON or YAML
config = Config.load("config.yaml", schema="schema.json")

# Validate after loading
config = Config.load("config.yaml")
config.validate(schema="schema.yaml")

# Validate resolved values explicitly
await config.resolve_all()
config.validate()  # Re-validates with resolved values
```

### JavaScript

```javascript
const config = await Config.load("config.yaml", {
    schema: "schema.yaml"
});

// Or validate separately
const config = await Config.load("config.yaml");
config.validate("schema.yaml");
```

### Schema Format

Schemas use JSON Schema (Draft 2020-12), written in YAML or JSON:

```yaml
# schema.yaml
type: object
required:
  - database
  - api

properties:
  database:
    type: object
    required: [host, port]
    properties:
      host:
        type: string
        description: Database hostname
      port:
        type: integer
        minimum: 1
        maximum: 65535
      pool_size:
        type: integer
        minimum: 1
        default: 10

  api:
    type: object
    properties:
      timeout:
        type: number
        minimum: 0
        description: Request timeout in seconds
      retries:
        type: integer
        minimum: 0
        maximum: 10
        default: 3
```

## Behavior

### Two-Phase Validation (from [ADR-007](../../adr/ADR-007-schema-validation.md))

**Phase 1: Structural Validation** (at load time)
- Required keys present
- Object/array structure matches
- Additional properties allowed/denied
- Interpolations (`${...}`) pass as valid placeholders

**Phase 2: Type/Value Validation** (on access or `validate()`)
- Resolved values match expected types
- Constraints (min, max, pattern, enum) are checked
- Type coercion applied if schema expects different type

### Type Coercion (from [ADR-012](../../adr/ADR-012-type-coercion.md))

When a schema specifies a type, string values are automatically coerced:

```yaml
# schema.yaml
properties:
  port:
    type: integer
```

```yaml
# config.yaml
port: ${env:PORT}  # Returns "8080" (string)
```

```python
config = Config.load("config.yaml", schema="schema.yaml")
port = config.port  # Returns 8080 (integer) - coerced
```

**Coercion Rules:**

| From | To | Rule |
|------|-----|------|
| string | integer | Parse as integer |
| string | number | Parse as float |
| string | boolean | `"true"/"false"/"1"/"0"` |

### Schema Composition with $ref

Split schemas across files:

```yaml
# schema.yaml
type: object
properties:
  database:
    $ref: "./schemas/database.yaml"
  api:
    $ref: "./schemas/api.yaml"
```

```yaml
# schemas/database.yaml
type: object
required: [host]
properties:
  host:
    type: string
  port:
    type: integer
    default: 5432
```

Refs are resolved relative to the schema file.

### Default Values

Schema defaults are applied during validation:

```yaml
# schema.yaml
properties:
  pool_size:
    type: integer
    default: 10
```

```yaml
# config.yaml
# pool_size not specified
```

```python
config = Config.load("config.yaml", schema="schema.yaml")
print(config.pool_size)  # 10 (from schema default)
```

### Additional Properties

By default, extra keys are allowed. Use `additionalProperties: false` to reject:

```yaml
# schema.yaml
type: object
properties:
  name:
    type: string
additionalProperties: false  # Only 'name' allowed
```

```yaml
# config.yaml
name: myapp
extra_key: value  # This will cause an error
```

## Error Cases

### Missing Required Key

```
StructuralValidationError: Missing required key
  Path: database.host
  Schema: schema.yaml#/properties/database/required
  Help: Add 'host' key to the database section
```

### Type Mismatch

```
TypeValidationError: Invalid type
  Path: database.port
  Expected: integer
  Got: string ("not-a-number")
  Resolved from: ${env:DB_PORT}
  Help: Ensure DB_PORT contains a valid integer
```

### Constraint Violation

```
TypeValidationError: Value out of range
  Path: database.port
  Constraint: minimum: 1, maximum: 65535
  Got: 70000
  Help: Port must be between 1 and 65535
```

### Additional Property Not Allowed

```
StructuralValidationError: Additional property not allowed
  Path: extra_key
  Schema: schema.yaml#/additionalProperties
  Help: Remove 'extra_key' or update schema to allow it
```

### Enum Violation

```yaml
# schema.yaml
properties:
  log_level:
    type: string
    enum: [debug, info, warn, error]
```

```
TypeValidationError: Value not in allowed set
  Path: log_level
  Allowed: debug, info, warn, error
  Got: "verbose"
  Help: Use one of the allowed values
```

## Examples

### Complete Schema Example

```yaml
# schema.yaml
$schema: "https://json-schema.org/draft/2020-12/schema"
title: Application Configuration
description: Configuration schema for MyApp

type: object
required:
  - app
  - database

properties:
  app:
    type: object
    required: [name]
    properties:
      name:
        type: string
        minLength: 1
        description: Application name
      version:
        type: string
        pattern: "^\\d+\\.\\d+\\.\\d+$"
        description: Semantic version
      debug:
        type: boolean
        default: false

  database:
    type: object
    required: [host]
    properties:
      host:
        type: string
        description: Database hostname
      port:
        type: integer
        minimum: 1
        maximum: 65535
        default: 5432
      ssl:
        type: boolean
        default: true
      pool:
        type: object
        properties:
          min:
            type: integer
            minimum: 1
            default: 5
          max:
            type: integer
            minimum: 1
            default: 20

  logging:
    type: object
    properties:
      level:
        type: string
        enum: [debug, info, warn, error]
        default: info
      format:
        type: string
        enum: [json, text]
        default: json
```

### Using the Schema

```yaml
# config.yaml
app:
  name: myapp
  version: 1.0.0

database:
  host: ${env:DB_HOST}
  port: ${env:DB_PORT,default=5432}

logging:
  level: ${env:LOG_LEVEL,default=info}
```

```python
config = Config.load("config.yaml", schema="schema.yaml")

# Access triggers resolution + type validation
print(config.app.name)       # "myapp"
print(config.database.port)  # 5432 (integer, coerced from string)
print(config.app.debug)      # False (from default)
print(config.database.ssl)   # True (from default)
```

### Validation Errors

```python
from holoconf import Config
from holoconf.errors import ValidationError

try:
    config = Config.load("config.yaml", schema="schema.yaml")
    await config.resolve_all()
    config.validate()
except ValidationError as e:
    print(f"Validation failed at {e.path}: {e.message}")
    print(f"Help: {e.help}")
```

### Custom Error Handling

```python
# Collect all validation errors instead of failing on first
errors = config.validate(collect_errors=True)

for error in errors:
    print(f"{error.path}: {error.message}")
```

## Implementation Notes

### JSON Schema Library

Use a Rust JSON Schema library (e.g., `jsonschema-rs`) that supports Draft 2020-12.

### Two-Phase Implementation

1. **Phase 1 (structural)**: Run JSON Schema validation, but treat `${...}` strings as wildcards that match any type
2. **Phase 2 (type/value)**: After resolution, validate individual values against their schema constraints

### Default Application

- Track which values have schema defaults
- Apply defaults lazily (on access) or eagerly (on `resolve_all()`)
- Defaults don't override explicit values

### $ref Resolution

- Parse schema files with a custom loader that handles `$ref`
- Resolve relative paths from schema file location
- Cache resolved schemas

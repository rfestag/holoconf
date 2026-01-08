# Python Package

The holoconf Python package provides native bindings to the Rust core library via PyO3, giving you high-performance configuration management with Pythonic ergonomics.

## Installation

```bash
pip install holoconf
```

Requires Python 3.9 or later. The package includes pre-built wheels for common platforms.

## Quick Start

```python
from holoconf import Config, Schema

# Load configuration
config = Config.load("config.yaml")

# Access values (resolves interpolations automatically)
host = config.get("database.host")
port = config.get("database.port")

# Or use dict-like access
host = config["database.host"]

# Export resolved configuration
print(config.to_yaml(resolve=True))

# Validate against a schema
schema = Schema.load("schema.json")
config.validate(schema)
```

## Features

- **Native performance** - Rust core compiled to native extension
- **Lazy resolution** - Values resolved on access, not at parse time
- **Type coercion** - `get_string()`, `get_int()`, `get_float()`, `get_bool()`
- **Dict-like access** - `config["key"]` and `config.key` syntax
- **Schema validation** - JSON Schema support with detailed errors
- **Serialization** - Export to YAML/JSON with optional redaction

## Package Contents

### Classes

| Class | Description |
|-------|-------------|
| [Config](classes/config.md) | Main configuration object for loading and accessing values |
| [Schema](classes/schema.md) | JSON Schema validator for configuration |

### Exceptions

| Exception | Description |
|-----------|-------------|
| [HoloconfError](exceptions/holoconf-error.md) | Base exception for all holoconf errors |
| [ParseError](exceptions/parse-error.md) | YAML/JSON syntax errors |
| [ValidationError](exceptions/validation-error.md) | Schema validation failures |
| [ResolverError](exceptions/resolver-error.md) | Resolution failures (missing env vars, etc.) |
| [PathNotFoundError](exceptions/path-not-found-error.md) | Config path doesn't exist |
| [CircularReferenceError](exceptions/circular-reference-error.md) | Circular reference detected |
| [TypeCoercionError](exceptions/type-coercion-error.md) | Type conversion failures |

## Examples

### Environment Variables with Defaults

```python
# config.yaml:
# database:
#   host: ${env:DB_HOST,localhost}
#   port: ${env:DB_PORT,5432}

config = Config.load("config.yaml")

# Uses environment variable if set, otherwise default
host = config.get("database.host")  # "localhost" or $DB_HOST
```

### Merging Configurations

```python
# Load base config, then override with environment-specific
config = Config.load_merged([
    "config/base.yaml",
    "config/production.yaml"
])
```

### Validation with Error Collection

```python
schema = Schema.load("schema.json")
errors = config.validate_collect(schema)

if errors:
    print("Validation errors:")
    for error in errors:
        print(f"  - {error}")
else:
    print("Configuration is valid")
```

### Safe Export (Redacted)

```python
# Redact sensitive values for logging
safe_yaml = config.to_yaml(resolve=True, redact=True)
print(safe_yaml)
# database:
#   host: prod-db.example.com
#   password: "[REDACTED]"
```

## See Also

- [Getting Started](../../guide/getting-started.md) - Installation and first steps
- [Interpolation](../../guide/interpolation.md) - Variable substitution syntax
- [Validation](../../guide/validation.md) - Schema validation details

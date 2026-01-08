# Python API Reference

## Installation

```bash
pip install holoconf
```

## Config Class

The main entry point for loading and accessing configuration.

### Loading Configuration

```python
from holoconf import Config

# From a file
config = Config.from_file("config.yaml")

# From a string
config = Config.from_string("""
database:
  host: localhost
  port: 5432
""")

# From multiple files (merged in order)
config = Config.from_files(["base.yaml", "override.yaml"])
```

### Accessing Values

```python
# Get a value by path (dot notation)
value = config.get("database.host")

# Get with type hint
port: int = config.get("database.port")

# Get a subsection
db_config = config.get("database")  # Returns dict

# Check if path exists
exists = config.has("database.host")
```

### Validation

```python
from holoconf import Config, ValidationError

config = Config.from_file("config.yaml")

try:
    config.validate("schema.json")
except ValidationError as e:
    print(f"Invalid: {e}")
```

### Serialization

```python
# Export to YAML
yaml_str = config.dump(format="yaml")

# Export to JSON
json_str = config.dump(format="json")

# Export with resolution (interpolations resolved)
resolved = config.dump(format="yaml", resolve=True)

# Export with sensitive values redacted
safe = config.dump(format="yaml", redact=["password", "secret"])
```

## Exception Classes

### HoloconfError

Base exception for all holoconf errors.

```python
from holoconf import HoloconfError

try:
    config = Config.from_file("config.yaml")
except HoloconfError as e:
    print(f"Configuration error: {e}")
```

### ParseError

Raised when configuration cannot be parsed.

```python
from holoconf import ParseError

try:
    config = Config.from_file("invalid.yaml")
except ParseError as e:
    print(f"Parse error at line {e.line}: {e.message}")
```

### PathNotFoundError

Raised when accessing a non-existent path.

```python
from holoconf import PathNotFoundError

try:
    value = config.get("nonexistent.path")
except PathNotFoundError as e:
    print(f"Path not found: {e.path}")
```

### ResolverError

Raised when a resolver fails (e.g., missing environment variable).

```python
from holoconf import ResolverError

try:
    value = config.get("database.password")
except ResolverError as e:
    print(f"Failed to resolve: {e}")
```

### ValidationError

Raised when configuration fails schema validation.

```python
from holoconf import ValidationError

try:
    config.validate("schema.json")
except ValidationError as e:
    print(f"Validation failed at {e.path}: {e.message}")
```

### CircularReferenceError

Raised when self-references create a cycle.

```python
from holoconf import CircularReferenceError

# If config has: a: ${b}, b: ${a}
try:
    value = config.get("a")
except CircularReferenceError as e:
    print(f"Circular reference detected: {e.path}")
```

## Type Conversions

When accessing values, holoconf returns Python-native types:

| YAML Type | Python Type |
|-----------|-------------|
| string | `str` |
| integer | `int` |
| float | `float` |
| boolean | `bool` |
| null | `None` |
| sequence | `list` |
| mapping | `dict` |

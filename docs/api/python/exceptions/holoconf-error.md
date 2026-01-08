# HoloconfError

Base exception for all holoconf errors.

## Exception Hierarchy

All holoconf exceptions inherit from `HoloconfError`:

```
HoloconfError (base)
├── ParseError          - YAML/JSON syntax errors
├── ValidationError     - Schema validation failures
├── ResolverError       - Resolution failures
├── PathNotFoundError   - Config path doesn't exist
├── CircularReferenceError - Circular reference detected
└── TypeCoercionError   - Type conversion failures
```

## Example

Catch all holoconf errors:

```python
from holoconf import Config, HoloconfError

try:
    config = Config.load("config.yaml")
    value = config.get("database.host")
except HoloconfError as e:
    print(f"Configuration error: {e}")
```

## Handling Specific Errors

For more granular error handling, catch specific exception types:

```python
from holoconf import (
    Config,
    ParseError,
    PathNotFoundError,
    ResolverError,
    HoloconfError,
)

try:
    config = Config.load("config.yaml")
    value = config.get("database.host")
except ParseError as e:
    print(f"Invalid config file: {e}")
except PathNotFoundError as e:
    print(f"Missing config key: {e}")
except ResolverError as e:
    print(f"Failed to resolve: {e}")
except HoloconfError as e:
    print(f"Other config error: {e}")
```

## Class Reference

::: holoconf.HoloconfError
    options:
      show_root_heading: false

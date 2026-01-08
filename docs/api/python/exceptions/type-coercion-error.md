# TypeCoercionError

Raised when a value cannot be converted to the requested type.

## When It's Raised

- Calling `get_int()` on a non-numeric string
- Calling `get_bool()` on a value that isn't boolean-like
- Calling `get_float()` on text that can't be parsed as a number
- Schema validation with type coercion enabled

## Example

```python
from holoconf import Config, TypeCoercionError

config = Config.loads("""
database:
  port: not-a-number
""")

try:
    port = config.get_int("database.port")
except TypeCoercionError as e:
    print(f"Type error: {e}")
    # Type error: Cannot convert 'not-a-number' to integer
```

## Type Coercion Rules

| Method | Accepts |
|--------|---------|
| `get_string()` | Any value (converts to string) |
| `get_int()` | Integers, numeric strings like `"42"` |
| `get_float()` | Floats, integers, numeric strings |
| `get_bool()` | Booleans, `"true"`/`"false"` (case-insensitive) |

## Handling

```python
from holoconf import Config, TypeCoercionError

config = Config.load("config.yaml")

try:
    port = config.get_int("database.port")
except TypeCoercionError:
    # Fall back to string and parse manually, or use default
    port_str = config.get_string("database.port")
    port = int(port_str) if port_str.isdigit() else 5432
```

## Class Reference

::: holoconf.TypeCoercionError
    options:
      show_root_heading: false

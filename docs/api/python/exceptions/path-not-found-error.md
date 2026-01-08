# PathNotFoundError

Raised when a requested configuration path doesn't exist.

## When It's Raised

- Accessing a key that doesn't exist in the configuration
- Typo in the path
- Path exists in a different config file that wasn't loaded

## Example

```python
from holoconf import Config, PathNotFoundError

config = Config.loads("""
database:
  host: localhost
""")

try:
    # 'port' doesn't exist
    port = config.get("database.port")
except PathNotFoundError as e:
    print(f"Path not found: {e}")
    # Path not found: 'database.port' does not exist
```

## Handling with Defaults

Use `get_raw()` with a try/except or check existence first:

```python
from holoconf import Config, PathNotFoundError

config = Config.load("config.yaml")

# Option 1: Try/except with default
try:
    port = config.get("database.port")
except PathNotFoundError:
    port = 5432  # default

# Option 2: Use get_raw and check
raw = config.to_dict(resolve=False)
port = raw.get("database", {}).get("port", 5432)
```

## Class Reference

::: holoconf.PathNotFoundError
    options:
      show_root_heading: false

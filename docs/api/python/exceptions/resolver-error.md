# ResolverError

Raised when a resolver fails to resolve a value.

## When It's Raised

- Environment variable not found (and no default provided)
- File include fails (file not found, permission denied)
- HTTP request fails (network error, non-200 response)
- Invalid resolver syntax

## Example

```python
from holoconf import Config, ResolverError

config = Config.loads("""
database:
  password: ${env:DB_PASSWORD}
""")

try:
    # Fails if DB_PASSWORD environment variable is not set
    password = config.get("database.password")
except ResolverError as e:
    print(f"Resolution failed: {e}")
    # Resolution failed: Environment variable 'DB_PASSWORD' not found
```

## Providing Defaults

Avoid `ResolverError` by providing default values:

```yaml
database:
  # With default - won't raise if DB_PASSWORD is not set
  password: ${env:DB_PASSWORD,default_password}
```

## Handling

```python
from holoconf import Config, ResolverError
import os

config = Config.load("config.yaml")

try:
    password = config.get("database.password")
except ResolverError:
    # Fall back to interactive input or other source
    password = os.environ.get("DB_PASSWORD") or input("Enter DB password: ")
```

## Class Reference

::: holoconf.ResolverError
    options:
      show_root_heading: false

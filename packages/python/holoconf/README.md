# holoconf

Cross-language configuration library with resolver support.

## Installation

```bash
pip install holoconf
```

## Quick Start

```python
from holoconf import Config

# Load from YAML string
config = Config.loads("""
database:
  host: ${env:DB_HOST,localhost}
  port: 5432
""")

# Access values
print(config.get("database.host"))  # Uses DB_HOST env var or "localhost"
print(config.get_int("database.port"))  # 5432

# Load from file
config = Config.load("config.yaml")

# Export resolved config
print(config.to_yaml())
```

## Features

- **Environment variables**: `${env:VAR}` or `${env:VAR,default}`
- **Self-references**: `${path.to.value}` or `${.sibling}`
- **File includes**: `${file:./other.yaml}`
- **Type coercion**: `get_int()`, `get_bool()`, `get_float()`
- **Lazy resolution**: Values resolved on access, cached for efficiency
- **Escape sequences**: `\${literal}` for literal `${`

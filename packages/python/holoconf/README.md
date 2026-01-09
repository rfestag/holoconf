# holoconf

[![PyPI](https://img.shields.io/pypi/v/holoconf)](https://pypi.org/project/holoconf/)
[![Python](https://img.shields.io/pypi/pyversions/holoconf)](https://pypi.org/project/holoconf/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A Python configuration library with hierarchical merging, interpolation, and schema validation. Built on a high-performance Rust core.

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
  url: postgresql://${.host}:${.port}/mydb
""")

# Access values with type coercion
print(config.get("database.host"))      # Uses DB_HOST env var or "localhost"
print(config.get_int("database.port"))  # 5432
print(config.get("database.url"))       # Resolves self-references

# Load from file
config = Config.load("config.yaml")

# Merge multiple configs (later files override earlier)
config = Config.load("base.yaml", "override.yaml")

# Export resolved configuration
print(config.to_yaml())
print(config.to_json())
```

## Features

- **Environment variables**: `${env:VAR}` or `${env:VAR,default}`
- **Self-references**: `${path.to.value}` or `${.sibling}` for relative paths
- **File includes**: `${file:./other.yaml}`
- **Type coercion**: `get_int()`, `get_bool()`, `get_float()`, `get_list()`, `get_dict()`
- **Lazy resolution**: Values resolved on access, cached for efficiency
- **Schema validation**: Validate against JSON Schema
- **Escape sequences**: `\${literal}` for literal `${`

## Interpolation Syntax

| Syntax | Description | Example |
|--------|-------------|---------|
| `${env:VAR}` | Environment variable | `${env:HOME}` |
| `${env:VAR,default}` | Env var with default | `${env:PORT,8080}` |
| `${path.to.value}` | Self-reference | `${database.host}` |
| `${.sibling}` | Relative reference | `${.port}` |
| `${file:path}` | Include file | `${file:./secrets.yaml}` |
| `\${literal}` | Escape (literal `${`) | `\${not_interpolated}` |

## CLI

holoconf includes a command-line interface:

```bash
# Get a configuration value
holoconf get database.host --config config.yaml

# Dump resolved configuration
holoconf dump --config config.yaml --format json
```

## Documentation

- **[User Guide](https://rfestag.github.io/holoconf/)** - Full documentation
- **[API Reference](https://rfestag.github.io/holoconf/api/python/)** - Python API docs
- **[GitHub](https://github.com/rfestag/holoconf)** - Source code and issues

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

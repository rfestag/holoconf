# HoloConf

[![CI](https://github.com/rfestag/holoconf/actions/workflows/rust.yml/badge.svg)](https://github.com/rfestag/holoconf/actions/workflows/rust.yml)
[![PyPI](https://img.shields.io/pypi/v/holoconf)](https://pypi.org/project/holoconf/)
[![crates.io](https://img.shields.io/crates/v/holoconf-core)](https://crates.io/crates/holoconf-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A multi-language hierarchical configuration library with a Rust core and bindings for Python.

## Overview

HoloConf provides a powerful and flexible configuration management system that works across multiple programming languages. Built with a high-performance Rust core, it offers language-specific bindings that feel native to each ecosystem while maintaining consistent behavior.

**[Read the Documentation](https://rfestag.github.io/holoconf/)**

## Key Features

- **Multi-language support** - Rust core with native bindings for Python (JavaScript and Go coming soon)
- **Interpolation** - Reference environment variables (`${env:VAR}`), other config values (`${path.to.value}`), and files (`${file:config.yaml}`)
- **Hierarchical merging** - Combine multiple config files with predictable override behavior
- **Schema validation** - Validate configuration against JSON Schema
- **Type coercion** - Automatic conversion between compatible types based on schema definitions
- **Lazy resolution** - Values are resolved on access, not at parse time

## Installation

### Python

```bash
pip install holoconf
```

### Rust

```toml
[dependencies]
holoconf-core = "0.1"
```

### CLI

```bash
cargo install holoconf-cli
```

## Quick Start

### Python

```python
from holoconf import Config

# Load from file
config = Config.from_file("config.yaml")

# Access values
db_host = config.get("database.host")

# With environment variable interpolation
# config.yaml: database.url: "${env:DATABASE_URL}"
db_url = config.get("database.url")
```

### Rust

```rust
use holoconf_core::Config;

let config = Config::from_file("config.yaml")?;
let db_host: String = config.get("database.host")?;
```

### CLI

```bash
# Get a configuration value
holoconf get database.host --config config.yaml

# Dump resolved configuration
holoconf dump --config config.yaml --format json
```

## Documentation

- **[Getting Started](https://rfestag.github.io/holoconf/guide/getting-started/)** - Installation and first configuration
- **[Interpolation](https://rfestag.github.io/holoconf/guide/interpolation/)** - Variable substitution syntax
- **[Resolvers](https://rfestag.github.io/holoconf/guide/resolvers/)** - Environment, file, and self-reference resolvers
- **[API Reference](https://rfestag.github.io/holoconf/api/python/)** - Detailed API documentation

## Testing

See the [Testing Guide](https://rfestag.github.io/holoconf/contributing/testing/) for information on running tests and the acceptance test framework.

## Contributing

We welcome contributions! See the [Contributing Guide](https://rfestag.github.io/holoconf/contributing/) for development setup and guidelines.

### Architecture & Design

- **[Architecture Decision Records](https://rfestag.github.io/holoconf/adr/README/)** - Design decisions and rationale
- **[Feature Specifications](https://rfestag.github.io/holoconf/specs/features/README/)** - Detailed feature specifications

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

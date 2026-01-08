# holoconf

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A multi-language configuration library with a Rust core and bindings for Python, JavaScript, and Go.

## Overview

holoconf provides a powerful and flexible configuration management system that works across multiple programming languages. Built with a high-performance Rust core, it offers language-specific bindings that feel native to each ecosystem while maintaining consistent behavior.

## Key Features

- **Interpolation Syntax**: Reference environment variables with `${env:VAR}` and configuration values with `${path.to.value}`
- **Lazy Resolution**: Configuration values are resolved on-demand, improving performance and enabling circular reference detection
- **Type Coercion**: Automatic conversion between compatible types (strings to numbers, booleans, etc.)
- **File Resolver**: Load and merge configuration from multiple file formats (YAML, JSON, etc.)
- **Multi-Language Support**: Consistent API across Python, JavaScript, and Go with language-specific idioms

## Installation

### Python

Install from PyPI:

```bash
pip install holoconf
```

Or build from source using maturin:

```bash
# Install maturin
pip install maturin

# Build and install in development mode
cd crates/holoconf-python
maturin develop

# Or build a wheel
maturin build --release
```

### JavaScript

Coming soon.

### Go

Coming soon.

## Documentation

- **[Architecture Decision Records](docs/adr/)** - Design decisions and rationale for key architectural choices
- **[Feature Specifications](docs/specs/features/)** - Detailed specifications for holoconf features

## Running Tests

### Rust Unit Tests

Run the Rust core library tests:

```bash
cargo test
```

### Python Acceptance Tests

Run the universal acceptance test suite against Python bindings:

```bash
python tools/test_runner.py --binding python
```

Run all acceptance tests across all available bindings:

```bash
python tools/test_runner.py --binding all
```

## Project Structure

```
holoconf/
├── crates/
│   ├── holoconf-core/       # Rust core library
│   └── holoconf-python/     # PyO3 Python bindings
├── packages/
│   └── python/holoconf/     # Python package
├── docs/
│   ├── adr/                 # Architecture Decision Records
│   └── specs/features/      # Feature specifications
├── tests/
│   └── acceptance/          # YAML acceptance test definitions
└── tools/
    └── test_runner.py       # Universal acceptance test runner
```

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to contribute to holoconf.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

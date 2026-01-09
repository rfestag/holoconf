<div style="display: flex; align-items: center; gap: 2rem; padding: 1.5rem 0;">
  <img src="images/logo.svg" alt="HoloConf Logo" style="width: 120px; height: auto; flex-shrink: 0;">
  <div>
    <h1 style="margin: 0; font-size: 2.5rem;">HoloConf</h1>
    <p style="font-size: 1.1rem; color: #666; margin: 0.5rem 0 0 0;">
      Multi-language hierarchical configuration library
    </p>
  </div>
</div>

<p style="text-align: center; margin-bottom: 2rem;">
<a href="https://github.com/rfestag/holoconf/actions/workflows/rust.yml"><img src="https://github.com/rfestag/holoconf/actions/workflows/rust.yml/badge.svg" alt="CI"></a>
<a href="https://pypi.org/project/holoconf/"><img src="https://img.shields.io/pypi/v/holoconf" alt="PyPI"></a>
<a href="https://crates.io/crates/holoconf-core"><img src="https://img.shields.io/crates/v/holoconf-core" alt="crates.io"></a>
</p>

## Overview

HoloConf provides a consistent, powerful configuration management experience across multiple programming languages. Write your configuration once in YAML or JSON, and access it seamlessly from Python, Rust, JavaScript, Go, and more.

<div style="text-align: center; margin: 2rem 0;">
  <a href="guide/getting-started/" style="display: inline-block; padding: 0.75rem 2rem; background: #354f7a; color: white; text-decoration: none; border-radius: 0.25rem; font-weight: bold; margin: 0.5rem;">Get Started</a>
  <a href="api/python/" style="display: inline-block; padding: 0.75rem 2rem; border: 2px solid #354f7a; color: #354f7a; text-decoration: none; border-radius: 0.25rem; font-weight: bold; margin: 0.5rem;">API Reference</a>
</div>

## Features

- **Multi-language support** - Rust core with native bindings for Python, JavaScript, Go, and more
- **Interpolation** - Reference environment variables, other config values, files, and HTTP endpoints
- **Hierarchical merging** - Combine multiple config files with predictable override behavior
- **Schema validation** - Validate configuration against JSON Schema
- **Type coercion** - Automatic type conversion based on schema definitions
- **Lazy resolution** - Values are resolved on access, not at parse time

## Quick Example

=== "Python"

    ```python
    from holoconf import Config

    # Load configuration
    config = Config.from_file("config.yaml")

    # Access values with dot notation
    host = config.get("database.host")
    port = config.get("database.port")

    # Environment variables are resolved automatically
    # database:
    #   host: ${env:DB_HOST,localhost}
    #   port: ${env:DB_PORT,5432}
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    fn main() -> Result<(), holoconf::Error> {
        // Load configuration
        let config = Config::from_file("config.yaml")?;

        // Access values with dot notation
        let host: String = config.get("database.host")?;
        let port: i64 = config.get("database.port")?;

        Ok(())
    }
    ```

=== "CLI"

    ```bash
    # Get a specific value
    holoconf get config.yaml database.host

    # Dump resolved configuration
    holoconf dump config.yaml --resolve

    # Validate against a schema
    holoconf validate config.yaml --schema schema.json
    ```

## Installation

=== "Python"

    ```bash
    pip install holoconf
    ```

=== "Rust"

    ```toml
    # Cargo.toml
    [dependencies]
    holoconf = "0.1"
    ```

=== "CLI"

    ```bash
    # Install via pip (includes CLI)
    pip install holoconf

    # Or install the Rust binary
    cargo install holoconf-cli
    ```

## Next Steps

- [Getting Started](guide/getting-started.md) - Installation and first configuration
- [Interpolation](guide/interpolation.md) - Learn about variable substitution
- [Resolvers](guide/resolvers.md) - Environment, file, HTTP, and self-reference resolvers
- [API Reference](api/python/index.md) - Detailed API documentation

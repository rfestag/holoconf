# holoconf

**Multi-language hierarchical configuration library**

holoconf provides a consistent, powerful configuration management experience across multiple programming languages. Write your configuration once in YAML or JSON, and access it seamlessly from Python, Rust, JavaScript, Go, and more.

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

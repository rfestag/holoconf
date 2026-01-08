# holoconf-core

The `holoconf-core` crate is the foundation of holoconf, providing high-performance configuration management with resolver support.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
holoconf-core = "0.1"
```

## Quick Start

```rust
use holoconf_core::{Config, ConfigOptions, Schema};

fn main() -> Result<(), holoconf_core::Error> {
    // Load configuration
    let config = Config::from_yaml_file("config.yaml")?;

    // Access values (resolves interpolations automatically)
    let host = config.get_string("database.host")?;
    let port = config.get_i64("database.port")?;

    // Validate against a schema
    let schema = Schema::from_file("schema.json")?;
    config.validate(&schema)?;

    Ok(())
}
```

## Features

- **Zero-copy parsing** - Efficient YAML/JSON parsing with serde
- **Lazy resolution** - Values resolved on access, cached for performance
- **Thread-safe** - `Config` is `Send + Sync` for concurrent access
- **Type coercion** - Automatic conversion with schema support
- **Structured errors** - Rich error context with paths and suggestions

## API Documentation

📚 **[Full API documentation on docs.rs](https://docs.rs/holoconf-core)**

The rustdoc documentation includes all public types, methods, and usage examples.

## Feature Flags

```toml
[dependencies]
holoconf-core = { version = "0.1", features = ["http"] }
```

| Feature | Description | Default |
|---------|-------------|---------|
| `http` | Enable HTTP resolver | No |

## See Also

- [Getting Started](../../guide/getting-started.md) - Installation and first steps
- [ADR-001 Multi-Language Architecture](../../adr/ADR-001-multi-language-architecture.md) - Why Rust core
- [ADR-010 Thread Safety](../../adr/ADR-010-thread-safety.md) - Concurrency design

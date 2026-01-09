# holoconf-core

[![crates.io](https://img.shields.io/crates/v/holoconf-core)](https://crates.io/crates/holoconf-core)
[![docs.rs](https://docs.rs/holoconf-core/badge.svg)](https://docs.rs/holoconf-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Core configuration library with hierarchical merging, interpolation, and schema validation.

## Features

- **Interpolation** - Reference environment variables (`${env:VAR}`), other config values (`${path.to.value}`), and files (`${file:config.yaml}`)
- **Hierarchical merging** - Combine multiple config files with predictable override behavior
- **Schema validation** - Validate configuration against JSON Schema
- **Type coercion** - Automatic conversion between compatible types based on schema definitions
- **Lazy resolution** - Values are resolved on access, not at parse time

## Installation

```toml
[dependencies]
holoconf-core = "0.1"
```

## Quick Start

```rust
use holoconf_core::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load from YAML string
    let config = Config::from_str(r#"
        database:
          host: ${env:DB_HOST,localhost}
          port: 5432
    "#)?;

    // Access values
    let host: String = config.get("database.host")?;
    let port: i64 = config.get("database.port")?;

    println!("Connecting to {}:{}", host, port);
    Ok(())
}
```

## Interpolation Syntax

| Syntax | Description | Example |
|--------|-------------|---------|
| `${env:VAR}` | Environment variable | `${env:HOME}` |
| `${env:VAR,default}` | Env var with default | `${env:PORT,8080}` |
| `${path.to.value}` | Self-reference | `${database.host}` |
| `${.sibling}` | Relative reference | `${.port}` |
| `${file:path}` | Include file | `${file:./secrets.yaml}` |
| `\${literal}` | Escape (literal `${`) | `\${not_interpolated}` |

## Documentation

- **[User Guide](https://rfestag.github.io/holoconf/)** - Full documentation
- **[API Reference](https://docs.rs/holoconf-core)** - Rust API docs
- **[GitHub](https://github.com/rfestag/holoconf)** - Source code and issues

## Related Crates

- [`holoconf-cli`](https://crates.io/crates/holoconf-cli) - Command-line interface

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

# holoconf-cli

[![crates.io](https://img.shields.io/crates/v/holoconf-cli)](https://crates.io/crates/holoconf-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Command-line interface for holoconf configuration management.

## Installation

```bash
cargo install holoconf-cli
```

Or download pre-built binaries from the [GitHub Releases](https://github.com/rfestag/holoconf/releases).

## Usage

### Get a configuration value

```bash
holoconf get database.host --config config.yaml
```

### Dump resolved configuration

```bash
# Output as YAML (default)
holoconf dump --config config.yaml

# Output as JSON
holoconf dump --config config.yaml --format json
```

### Merge multiple config files

```bash
holoconf dump --config base.yaml --config override.yaml
```

### Validate against a schema

```bash
holoconf validate --config config.yaml --schema schema.json
```

## Example

Given a `config.yaml`:

```yaml
database:
  host: ${env:DB_HOST,localhost}
  port: 5432
  url: postgresql://${.host}:${.port}/mydb
```

```bash
$ export DB_HOST=prod-db.example.com
$ holoconf get database.url --config config.yaml
postgresql://prod-db.example.com:5432/mydb
```

## Documentation

- **[User Guide](https://rfestag.github.io/holoconf/)** - Full documentation
- **[GitHub](https://github.com/rfestag/holoconf)** - Source code and issues

## Related Crates

- [`holoconf-core`](https://crates.io/crates/holoconf-core) - Core library

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

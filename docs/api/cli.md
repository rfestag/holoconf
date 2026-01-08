# CLI Reference

The `holoconf` command-line interface provides tools for inspecting, validating, and exporting configuration.

## Installation

=== "Via pip"

    ```bash
    pip install holoconf
    ```

=== "Via cargo"

    ```bash
    cargo install holoconf-cli
    ```

## Commands

### `holoconf get`

Get a specific value from a configuration file.

```bash
holoconf get <CONFIG_FILE> <PATH>
```

**Arguments:**

- `CONFIG_FILE` - Path to the configuration file
- `PATH` - Dot-notation path to the value (e.g., `database.host`)

**Options:**

- `--resolve` / `-r` - Resolve interpolations before returning

**Examples:**

```bash
# Get a simple value
$ holoconf get config.yaml app.name
my-application

# Get a nested value
$ holoconf get config.yaml database.connection.host
localhost

# With environment variable resolution
$ DB_HOST=prod.example.com holoconf get config.yaml database.host --resolve
prod.example.com
```

### `holoconf dump`

Dump the entire configuration to stdout.

```bash
holoconf dump <CONFIG_FILE> [OPTIONS]
```

**Arguments:**

- `CONFIG_FILE` - Path to the configuration file

**Options:**

- `--format` / `-f` - Output format: `yaml` (default) or `json`
- `--resolve` / `-r` - Resolve all interpolations
- `--redact` - Redact sensitive keys (comma-separated list)

**Examples:**

```bash
# Dump as YAML
$ holoconf dump config.yaml
app:
  name: my-application
database:
  host: ${env:DB_HOST,localhost}

# Dump as JSON with resolution
$ holoconf dump config.yaml --format json --resolve
{
  "app": {
    "name": "my-application"
  },
  "database": {
    "host": "localhost"
  }
}

# Dump with sensitive values redacted
$ holoconf dump config.yaml --redact password,secret,api_key
app:
  name: my-application
database:
  password: "[REDACTED]"
```

### `holoconf validate`

Validate a configuration file against a JSON Schema.

```bash
holoconf validate <CONFIG_FILE> --schema <SCHEMA_FILE>
```

**Arguments:**

- `CONFIG_FILE` - Path to the configuration file

**Options:**

- `--schema` / `-s` - Path to the JSON Schema file (required)

**Examples:**

```bash
# Validate configuration
$ holoconf validate config.yaml --schema schema.json
✓ Configuration is valid

# Validation failure
$ holoconf validate config.yaml --schema schema.json
✗ Validation failed:
  - database.port: -1 is less than the minimum of 1
  - app.name: missing required property
```

**Exit codes:**

- `0` - Configuration is valid
- `1` - Validation failed
- `2` - Error reading files

### `holoconf merge`

Merge multiple configuration files and output the result.

```bash
holoconf merge <CONFIG_FILES>... [OPTIONS]
```

**Arguments:**

- `CONFIG_FILES` - Two or more configuration files to merge

**Options:**

- `--format` / `-f` - Output format: `yaml` (default) or `json`
- `--resolve` / `-r` - Resolve all interpolations in output

**Examples:**

```bash
# Merge base with overrides
$ holoconf merge base.yaml production.yaml
app:
  name: my-application
  debug: false
database:
  host: prod-db.example.com
  port: 5432

# Merge and output as JSON
$ holoconf merge base.yaml production.yaml --format json
```

## Environment Variables

The CLI respects these environment variables:

| Variable | Description |
|----------|-------------|
| `HOLOCONF_LOG` | Log level: `error`, `warn`, `info`, `debug`, `trace` |
| `NO_COLOR` | Disable colored output when set |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Validation or resolution error |
| 2 | File not found or I/O error |
| 3 | Parse error (invalid YAML/JSON) |

## See Also

- [FEAT-006 CLI](../specs/features/FEAT-006-cli.md) - Full CLI specification

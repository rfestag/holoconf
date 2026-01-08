# FEAT-006: Command Line Interface

## Overview

Provide a `holoconf` CLI for validating configs, resolving values, debugging, and generating documentation outside of application code.

## User Stories

- As a developer, I want to validate my config files in CI before deployment
- As an operator, I want to see the resolved config for debugging
- As a developer, I want to check my schema is valid
- As a developer, I want to see where config values come from when debugging merge issues

## Dependencies

- [FEAT-001: Configuration File Loading](FEAT-001-config-loading.md)
- [FEAT-002: Core Resolvers](FEAT-002-core-resolvers.md)
- [FEAT-003: Configuration Merging](FEAT-003-config-merging.md)
- [FEAT-004: Schema Validation](FEAT-004-schema-validation.md)
- [FEAT-005: Serialization and Export](FEAT-005-serialization.md)

## Installation

```bash
# Install via pip (Python)
pip install holoconf

# Install via npm (Node.js)
npm install -g holoconf

# Install via cargo (Rust)
cargo install holoconf-cli
```

## Commands

### `holoconf validate`

Validate configuration files against a schema.

```bash
# Validate a single file
holoconf validate config.yaml --schema schema.yaml

# Validate merged configs
holoconf validate base.yaml production.yaml --schema schema.yaml

# Validate with resolution (checks resolved values against schema)
holoconf validate config.yaml --schema schema.yaml --resolve

# Output format
holoconf validate config.yaml --schema schema.yaml --format json
```

**Options:**

| Option | Description |
|--------|-------------|
| `--schema, -s` | Path to schema file |
| `--resolve, -r` | Resolve interpolations before validating |
| `--format, -f` | Output format: `text` (default), `json` |
| `--quiet, -q` | Only output errors |

**Exit Codes:**

| Code | Meaning |
|------|---------|
| 0 | Valid |
| 1 | Validation errors |
| 2 | File not found or parse error |

**Output:**
```
$ holoconf validate config.yaml --schema schema.yaml
✓ config.yaml is valid

$ holoconf validate config.yaml --schema schema.yaml
✗ Validation failed

  database.port: Value out of range
    Expected: integer between 1 and 65535
    Got: 70000
    Location: config.yaml:5

  api.timeout: Missing required key
    Location: config.yaml
    Help: Add 'timeout' to the api section
```

### `holoconf dump`

Export configuration in various formats.

```bash
# Dump raw config (with placeholders)
holoconf dump config.yaml

# Dump resolved config (with redaction)
holoconf dump config.yaml --resolve

# Dump resolved without redaction (careful!)
holoconf dump config.yaml --resolve --no-redact

# Output as JSON
holoconf dump config.yaml --format json

# Dump merged configs
holoconf dump base.yaml production.yaml --resolve
```

**Options:**

| Option | Description |
|--------|-------------|
| `--resolve, -r` | Resolve interpolations |
| `--no-redact` | Don't redact sensitive values (requires `--resolve`) |
| `--format, -f` | Output format: `yaml` (default), `json` |
| `--output, -o` | Write to file instead of stdout |

**Output:**
```yaml
$ holoconf dump config.yaml --resolve
database:
  host: db.prod.example.com
  password: "[REDACTED]"
  port: 5432

api:
  endpoint: https://api.example.com
  timeout: 30
```

### `holoconf get`

Get a specific value from the configuration.

```bash
# Get a single value
holoconf get config.yaml database.host
# Output: localhost

# Get with resolution
holoconf get config.yaml database.host --resolve
# Output: db.prod.example.com

# Get nested value
holoconf get config.yaml database.pool.max
# Output: 20

# Get as JSON (for complex values)
holoconf get config.yaml database --format json
# Output: {"host": "localhost", "port": 5432, "pool": {"min": 5, "max": 20}}
```

**Options:**

| Option | Description |
|--------|-------------|
| `--resolve, -r` | Resolve interpolations |
| `--format, -f` | Output format: `text` (default), `json`, `yaml` |
| `--default, -d` | Default value if key not found |

### `holoconf sources`

Show where each config value comes from (for debugging merges).

```bash
holoconf sources base.yaml production.yaml local.yaml
```

**Output:**
```
$ holoconf sources base.yaml production.yaml local.yaml
database.host         base.yaml:3
database.port         base.yaml:4
database.pool.min     base.yaml:6
database.pool.max     production.yaml:4    (overrides base.yaml:7)
api.endpoint          production.yaml:8
api.timeout           local.yaml:2         (overrides production.yaml:9)
logging.level         local.yaml:5         (overrides base.yaml:12)
```

**Options:**

| Option | Description |
|--------|-------------|
| `--path, -p` | Filter to specific path prefix |
| `--format, -f` | Output format: `text` (default), `json` |

### `holoconf schema`

Schema-related utilities.

```bash
# Validate a schema file
holoconf schema validate schema.yaml

# Generate a template config from schema
holoconf schema template schema.yaml > config.template.yaml

# Show schema documentation
holoconf schema docs schema.yaml
```

**Subcommands:**

#### `holoconf schema validate`

Check that a schema is valid JSON Schema.

```bash
holoconf schema validate schema.yaml
```

#### `holoconf schema template`

Generate a config template with placeholders from schema:

```bash
holoconf schema template schema.yaml
```

**Output:**
```yaml
# Generated from schema.yaml
# Required fields marked with # REQUIRED

database:  # REQUIRED
  host: ""  # REQUIRED - Database hostname
  port: 5432  # Default: 5432
  pool:
    min: 5  # Default: 5
    max: 20  # Default: 20

api:
  timeout: 30  # Default: 30
  retries: 3  # Default: 3
```

#### `holoconf schema docs`

Generate human-readable documentation from schema:

```bash
holoconf schema docs schema.yaml --format markdown
```

**Output:**
```markdown
# Configuration Reference

## database (required)

Database connection settings.

| Key | Type | Required | Default | Description |
|-----|------|----------|---------|-------------|
| host | string | Yes | - | Database hostname |
| port | integer | No | 5432 | Port number (1-65535) |
| pool.min | integer | No | 5 | Minimum pool size |
| pool.max | integer | No | 20 | Maximum pool size |

## api

API settings.

| Key | Type | Required | Default | Description |
|-----|------|----------|---------|-------------|
| timeout | number | No | 30 | Request timeout in seconds |
| retries | integer | No | 3 | Number of retries (0-10) |
```

### `holoconf check`

Quick syntax check without full validation.

```bash
# Check YAML/JSON syntax
holoconf check config.yaml

# Check multiple files
holoconf check config/*.yaml
```

**Output:**
```
$ holoconf check config.yaml
✓ config.yaml: valid YAML

$ holoconf check broken.yaml
✗ broken.yaml: invalid YAML
  Line 15: Unexpected indentation
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `HOLOCONF_ALLOW_REMOTE` | Enable remote URL resolution (`true`/`false`) |
| `HOLOCONF_FILE_ROOTS` | Colon-separated list of allowed file paths |
| `HOLOCONF_NO_COLOR` | Disable colored output |
| `HOLOCONF_DEBUG` | Enable debug logging |

## Examples

### CI/CD Validation

```yaml
# .github/workflows/validate.yml
jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: pip install holoconf
      - run: holoconf validate config/*.yaml --schema schema.yaml
```

### Pre-commit Hook

```yaml
# .pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: holoconf-validate
        name: Validate holoconf configs
        entry: holoconf validate --schema schema.yaml
        language: system
        files: ^config/.*\.yaml$
```

### Debugging Merge Issues

```bash
# See which file each value comes from
holoconf sources base.yaml env/production.yaml local.yaml

# See the final merged result
holoconf dump base.yaml env/production.yaml local.yaml

# Get a specific value to check
holoconf get base.yaml env/production.yaml database.pool.max
```

### Generating Documentation

```bash
# Generate markdown docs from schema
holoconf schema docs schema.yaml --format markdown > docs/CONFIG.md

# Generate a template for new configs
holoconf schema template schema.yaml > config.template.yaml
```

## Implementation Notes

### CLI Framework

- Rust: Use `clap` for argument parsing
- Provide as standalone binary (`holoconf`) and language-specific wrappers

### Output Formatting

- Support `--format` for machine-readable output (JSON)
- Use colors for human-readable output (disable with `--no-color` or `NO_COLOR` env)
- Structured exit codes for scripting

### Error Output

- Errors go to stderr
- Include file paths and line numbers where possible
- Provide actionable help text

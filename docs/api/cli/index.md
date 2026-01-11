# HoloConf CLI

The `holoconf` CLI provides tools for inspecting, validating, and exporting configuration files from the command line.

## Installation

Choose your preferred installation method:

=== "pip (Python)"

    Install from PyPI with the Python package:

    ```bash
    pip install holoconf
    ```

    The CLI is included with the Python package. Best for users who also use the Python library.

=== "cargo (Rust)"

    Install from crates.io:

    ```bash
    cargo install holoconf-cli
    ```

    Standalone binary without Python dependency. Best for system-wide installation or CI/CD pipelines.

=== "pipx (Isolated)"

    Install in an isolated environment:

    ```bash
    pipx install holoconf
    ```

    Keeps HoloConf isolated from your project's dependencies. Best for global CLI tools.

=== "Binary (Direct)"

    Download pre-built binaries from [GitHub Releases](https://github.com/rfestag/holoconf/releases):

    ```bash
    # Linux (x86_64)
    curl -L https://github.com/rfestag/holoconf/releases/latest/download/holoconf-linux-x86_64.tar.gz | tar xz
    sudo mv holoconf /usr/local/bin/

    # macOS (Apple Silicon)
    curl -L https://github.com/rfestag/holoconf/releases/latest/download/holoconf-darwin-arm64.tar.gz | tar xz
    sudo mv holoconf /usr/local/bin/
    ```

    Pre-built binaries for fast installation without compilation.

Verify installation:

```bash
holoconf --version
```

## Quick Start

```bash
# Check syntax
holoconf check config.yaml

# Get a specific value
holoconf get config.yaml database.host

# Dump resolved configuration
holoconf dump config.yaml --resolve

# Validate against a schema
holoconf validate config.yaml --schema schema.json
```

## Commands

### holoconf check

Quick syntax check for configuration files.

```
holoconf check <FILES>...
```

**Examples:**

```bash
$ holoconf check config.yaml
✓ config.yaml: valid YAML

$ holoconf check config.yaml secrets.yaml
✓ config.yaml: valid YAML
✓ secrets.yaml: valid YAML

$ holoconf check broken.yaml
✗ broken.yaml: expected ':', but found '-' at line 3 column 1
```

---

### holoconf get

Get a specific value from configuration.

```
holoconf get <FILES>... <PATH> [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-r, --resolve` | Resolve interpolations (default: raw value) |
| `-f, --format` | Output format: `text`, `json`, `yaml` (default: text) |
| `-d, --default` | Default value if path not found |

**Examples:**

```bash
# Get a value
$ holoconf get config.yaml app.name
my-application

# Get with resolution
$ DB_HOST=prod.example.com holoconf get config.yaml database.host -r
prod.example.com

# Get nested object as JSON
$ holoconf get config.yaml database -f json
{"host": "localhost", "port": 5432}

# Use default if not found
$ holoconf get config.yaml optional.key -d "fallback"
fallback

# Merge multiple files, then get
$ holoconf get base.yaml production.yaml database.host
prod-db.example.com
```

---

### holoconf dump

Export entire configuration.

```
holoconf dump <FILES>... [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-r, --resolve` | Resolve all interpolations |
| `-f, --format` | Output format: `yaml`, `json` (default: yaml) |
| `-o, --output` | Write to file instead of stdout |
| `--no-redact` | Don't redact sensitive values |

**Examples:**

```bash
# Dump as YAML (default)
$ holoconf dump config.yaml
app:
  name: my-application
database:
  host: ${env:DB_HOST,default=localhost}

# Dump resolved as JSON
$ holoconf dump config.yaml --resolve --format json
{
  "app": {"name": "my-application"},
  "database": {"host": "localhost"}
}

# Dump merged configs
$ holoconf dump base.yaml production.yaml --resolve

# Write to file
$ holoconf dump config.yaml --resolve -o resolved.yaml
✓ Wrote to resolved.yaml
```

---

### holoconf validate

Validate configuration against JSON Schema.

```
holoconf validate <FILES>... --schema <SCHEMA> [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-s, --schema` | Path to schema file (required) |
| `-r, --resolve` | Resolve before validating |
| `-f, --format` | Output format: `text`, `json` (default: text) |
| `-q, --quiet` | Only output errors |

**Examples:**

```bash
# Validate configuration
$ holoconf validate config.yaml --schema schema.json
✓ config.yaml is valid

# Validate resolved values
$ holoconf validate config.yaml --schema schema.json --resolve
✓ config.yaml is valid

# Validation failure
$ holoconf validate bad-config.yaml --schema schema.json
✗ Validation failed
database.port: -1 is less than the minimum of 1

# JSON output for CI
$ holoconf validate config.yaml --schema schema.json --format json
{"valid": true}

# Quiet mode (exit code only)
$ holoconf validate config.yaml --schema schema.json --quiet
$ echo $?
0
```

**Exit Codes:**

| Code | Meaning |
|------|---------|
| 0 | Configuration is valid |
| 1 | Validation failed |
| 2 | Error reading files |

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Validation or resolution error |
| 2 | File not found or I/O error |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `NO_COLOR` | Disable colored output when set |

## Shell Completion

Generate shell completion scripts:

=== "Bash"

    ```bash
    # Add to ~/.bashrc
    eval "$(holoconf --completion bash)"
    ```

=== "Zsh"

    ```bash
    # Add to ~/.zshrc
    eval "$(holoconf --completion zsh)"
    ```

=== "Fish"

    ```bash
    holoconf --completion fish | source
    ```

!!! note
    Shell completion is planned but not yet implemented.

## See Also

- [Getting Started](../../guide/getting-started.md) - Installation and first steps
- [FEAT-006 CLI](../../specs/features/FEAT-006-cli.md) - Full CLI specification

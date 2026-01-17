# FEAT-003: Configuration Merging

## Status

Implemented

## Changelog

- 2026-01-17: Marked as Implemented (v0.2.0)

## Overview

Load and merge multiple configuration files in a specified order, with later files overriding earlier ones. This enables layered configuration (base → environment → local overrides).

## User Stories

- As a developer, I want to have a base config with environment-specific overrides
- As a developer, I want local development settings without modifying shared configs
- As a developer, I want to understand which file a value came from when debugging

## Dependencies

- [ADR-004: Config Merging Semantics](../../adr/ADR-004-config-merging.md)
- [FEAT-001: Configuration File Loading](FEAT-001-config-loading.md)

## API Surface

### Loading Multiple Files

```python
from holoconf import Config

# Load multiple files - later files override earlier
config = Config.load(
    "base.yaml",
    "environment.yaml",
    "local.yaml"  # Highest priority
)

# Load with glob patterns
config = Config.load("config/*.yaml")  # Sorted alphabetically

# Load with explicit order
config = Config.load(
    "config/00-base.yaml",
    "config/10-environment.yaml",
    "config/99-local.yaml"
)
```

### JavaScript

```javascript
const config = await Config.load(
    "base.yaml",
    "environment.yaml",
    "local.yaml"
);
```

### Rust

```rust
let config = Config::load(&[
    "base.yaml",
    "environment.yaml",
    "local.yaml",
])?;
```

## Behavior

### Merge Semantics (from [ADR-004](../../adr/ADR-004-config-merging.md))

**Deep merge with last-writer-wins:**

| Scenario | Behavior |
|----------|----------|
| Key exists in both | Later value wins |
| Key only in base | Preserved |
| Key only in overlay | Added |
| Both are objects | Deep merge recursively |
| Type mismatch | Later value replaces entirely |
| Value is `null` | Removes key from result |

### Deep Merge Example

```yaml
# base.yaml
database:
  host: localhost
  port: 5432
  pool:
    min: 5
    max: 20

logging:
  level: debug
```

```yaml
# production.yaml
database:
  host: prod-db.example.com
  pool:
    max: 100  # Only override max, keep min

logging:
  level: info
```

```yaml
# Result after merge
database:
  host: prod-db.example.com  # From production.yaml
  port: 5432                  # From base.yaml
  pool:
    min: 5                    # From base.yaml
    max: 100                  # From production.yaml

logging:
  level: info                 # From production.yaml
```

### Removing Keys with Null

```yaml
# base.yaml
database:
  host: localhost
  port: 5432
  debug_logging: true

# production.yaml
database:
  debug_logging: null  # Remove this key
```

```yaml
# Result
database:
  host: localhost
  port: 5432
  # debug_logging is removed
```

### Array Handling

Arrays are **replaced entirely**, not merged:

```yaml
# base.yaml
servers:
  - host: server1.example.com
  - host: server2.example.com

# override.yaml
servers:
  - host: prod1.example.com
```

```yaml
# Result - array is replaced, not merged
servers:
  - host: prod1.example.com
```

### Glob Pattern Loading

When using glob patterns, files are sorted alphabetically:

```python
config = Config.load("config/*.yaml")
# Loads: 00-base.yaml, 10-database.yaml, 20-api.yaml, 99-local.yaml
# in that order
```

### Optional Files

Files can be marked as optional (no error if missing):

```python
config = Config.load(
    "base.yaml",                    # Required
    "environment.yaml",             # Required
    Config.optional("local.yaml")   # Optional
)
```

```javascript
const config = await Config.load(
    "base.yaml",
    "environment.yaml",
    { path: "local.yaml", optional: true }
);
```

### Source Tracking (Debug)

Source tracking is available when loading a single file with `Config.load()`:

```python
config = Config.load("config.yaml")

# Get source file for a specific path
source = config.get_source("database.host")
print(source)  # "config.yaml"

# Get all sources as a dict
sources = config.dump_sources()
# {"database.host": "config.yaml", "database.port": "config.yaml", ...}
```

Note: Source tracking does not persist through `merge()` operations. For merged configs,
individual file loading provides source info, but merged results track only values.

#### CLI Usage

```bash
# Show source files instead of values
holoconf dump --sources base.yaml override.yaml
# Output:
# database.host: override.yaml
# database.port: base.yaml

# JSON format
holoconf dump --sources --format json base.yaml override.yaml
```

## Error Cases

### File Not Found (Required)

```python
config = Config.load("base.yaml", "missing.yaml")
```

```
FileNotFoundError: Configuration file not found
  Path: missing.yaml
  Help: Check that the file exists or mark it as optional
```

### Glob No Matches

```python
config = Config.load("config/*.yaml")  # No matching files
```

```
FileNotFoundError: No configuration files matched pattern
  Pattern: config/*.yaml
  Help: Check the pattern and directory exist
```

### Parse Error in Any File

```python
config = Config.load("base.yaml", "broken.yaml")
```

```
ParseError: Invalid YAML syntax
  Path: broken.yaml
  Line: 10
  Help: Fix syntax error before merging
```

## Examples

### Environment-Based Configuration

```
config/
├── base.yaml           # Shared defaults
├── development.yaml    # Dev settings
├── staging.yaml        # Staging settings
└── production.yaml     # Production settings
```

```python
import os

env = os.environ.get("APP_ENV", "development")

config = Config.load(
    "config/base.yaml",
    f"config/{env}.yaml",
    Config.optional("config/local.yaml")  # Developer overrides
)
```

### Feature Flags Overlay

```yaml
# base.yaml
features:
  new_checkout: false
  dark_mode: false
  beta_api: false
```

```yaml
# features-enabled.yaml
features:
  new_checkout: true
  beta_api: true
```

```python
config = Config.load("base.yaml")

if enable_features:
    config = Config.load("base.yaml", "features-enabled.yaml")
```

### Multi-Region Configuration

```yaml
# base.yaml
aws:
  region: us-east-1

database:
  host: ${env:DB_HOST}
```

```yaml
# regions/eu-west-1.yaml
aws:
  region: eu-west-1

endpoints:
  api: https://api.eu.example.com
```

```python
region = os.environ.get("AWS_REGION", "us-east-1")
config = Config.load(
    "base.yaml",
    f"regions/{region}.yaml"
)
```

### Debugging Merge Issues

```python
# Load with source tracking
config = Config.load(
    "base.yaml",
    "override.yaml",
    track_sources=True
)

# Find where a value came from
source = config.get_source("database.pool.max")
print(f"Value came from {source.file}:{source.line}")

# Dump full merge trace
for path, source in config.sources():
    print(f"{path}: {source.file}:{source.line}")
```

## Implementation Notes

### Merge Algorithm

```
function merge(base, overlay):
    result = copy(base)

    for key, value in overlay:
        if value is null:
            delete result[key]
        else if key not in result:
            result[key] = value
        else if both are objects:
            result[key] = merge(result[key], value)
        else:
            result[key] = value  # Replace

    return result
```

### Source Tracking

Source tracking is always enabled (low overhead):
- Store filename for each leaf value path
- Update on merge (overlay source replaces base source)
- File-level only (no line numbers) - sufficient for debugging merged configs
- Accessed via `get_source(path)` and `dump_sources()` methods

### Glob Handling

- Use `glob` crate in Rust
- Sort results alphabetically for deterministic order
- Expand before loading

# FEAT-001: Configuration File Loading

## Overview

Load configuration data from YAML or JSON files into a `Config` object that provides dot-notation access to values.

## User Stories

- As a developer, I want to load a config file so I can access configuration values in my application
- As a developer, I want to load configs from different formats (YAML, JSON) without changing my code
- As a developer, I want clear errors when config files are missing or malformed

## Dependencies

- [ADR-001: Multi-Language Architecture](../../adr/ADR-001-multi-language-architecture.md) (Rust core)
- [ADR-008: Error Handling Strategy](../../adr/ADR-008-error-handling.md)

## API Surface

### Python

```python
from holoconf import Config

# Load a single file
config = Config.load("config.yaml")

# Load from JSON
config = Config.load("config.json")

# Access values with dot notation
host = config.database.host
port = config.database.port

# Access with bracket notation (for dynamic keys)
key = "database"
db_config = config[key]

# Check if key exists
if "database" in config:
    ...

# Get with default
timeout = config.get("api.timeout", 30)
```

> **Note:** `load_async()` was considered but not implemented. Async file I/O provides
> minimal benefit for small local config files. The real async value is in *resolver*
> execution (SSM, HTTP, etc.), which is tracked separately.

### JavaScript/TypeScript

```javascript
import { Config } from 'holoconf';

// Load a single file
const config = await Config.load("config.yaml");

// Access values
const host = config.database.host;
const port = config.database.port;

// Bracket notation
const dbConfig = config["database"];

// Check existence
if ("database" in config) { ... }

// Get with default
const timeout = config.get("api.timeout", 30);
```

### Rust

```rust
use holoconf::Config;

// Load a single file
let config = Config::load("config.yaml")?;

// Access values
let host: &str = config.get("database.host")?;
let port: i32 = config.get("database.port")?;
```

## Behavior

### File Format Detection

Format is detected by file extension:
- `.yaml`, `.yml` → YAML parser
- `.json` → JSON parser

If extension is ambiguous or missing, attempt YAML first (YAML is a superset of JSON).

### Config Object Structure

The `Config` object wraps the parsed data and provides:

1. **Dot-notation access**: `config.database.host`
2. **Bracket access**: `config["database"]["host"]`
3. **Path access**: `config.get("database.host")`
4. **Iteration**: Iterate over keys at any level
5. **Length**: Number of keys at current level

### Value Types

| YAML/JSON Type | Python | JavaScript | Rust |
|----------------|--------|------------|------|
| string | `str` | `string` | `String` |
| integer | `int` | `number` | `i64` |
| float | `float` | `number` | `f64` |
| boolean | `bool` | `boolean` | `bool` |
| null | `None` | `null` | `Option::None` |
| array | `list` | `Array` | `Vec` |
| object | `Config` (nested) | `Config` (nested) | `Config` (nested) |

### Nested Objects

Nested objects return `Config` wrappers, enabling chained dot-notation:

```python
config = Config.load("config.yaml")
# config.yaml:
#   database:
#     host: localhost
#     port: 5432

db = config.database      # Returns Config wrapping {host: localhost, port: 5432}
host = config.database.host  # Returns "localhost"
```

### Interpolation Placeholders

At load time, interpolation placeholders (`${...}`) are preserved as-is. Resolution happens on access (see [FEAT-002](FEAT-002-core-resolvers.md)).

```python
config = Config.load("config.yaml")
# config.yaml:
#   port: ${env:PORT}

# Before access, value is the placeholder string
raw = config._raw("port")  # "${env:PORT}"

# On access, resolution is triggered
port = config.port  # Resolves to actual value
```

## Error Cases

### FileNotFoundError

Raised when the config file doesn't exist.

```
FileNotFoundError: Configuration file not found
  Path: /path/to/missing.yaml
  Help: Check that the file exists and the path is correct
```

### ParseError

Raised when the file contains invalid YAML/JSON.

```
ParseError: Invalid YAML syntax
  Path: config.yaml
  Line: 15
  Column: 3
  Details: Unexpected indentation
  Help: Check YAML indentation at line 15
```

### TypeError (Access)

Raised when accessing a value with incorrect type expectations.

```python
config.database.host.foo  # database.host is a string, not an object
```

```
TypeError: Cannot access property on non-object value
  Path: database.host.foo
  Type: string
  Help: 'database.host' is a string, not an object
```

### KeyError (Access)

Raised when accessing a non-existent key.

```python
config.nonexistent_key
```

```
KeyError: Key not found in configuration
  Path: nonexistent_key
  Available keys: database, api, logging
  Help: Check spelling or use config.get() with a default
```

## Examples

### Basic Usage

```yaml
# config.yaml
app:
  name: myapp
  version: 1.0.0

database:
  host: localhost
  port: 5432

logging:
  level: info
  format: json
```

```python
from holoconf import Config

config = Config.load("config.yaml")

print(config.app.name)        # "myapp"
print(config.database.port)   # 5432
print(config.logging.level)   # "info"

# Iterate over keys
for key in config:
    print(key)  # "app", "database", "logging"

# Check structure
print(len(config))  # 3
print("database" in config)  # True
```

### JSON Config

```json
{
  "api": {
    "endpoint": "https://api.example.com",
    "timeout": 30
  }
}
```

```python
config = Config.load("config.json")
print(config.api.endpoint)  # "https://api.example.com"
```

### Error Handling

```python
from holoconf import Config
from holoconf.errors import FileNotFoundError, ParseError

try:
    config = Config.load("config.yaml")
except FileNotFoundError as e:
    print(f"Config not found: {e.path}")
except ParseError as e:
    print(f"Invalid config at line {e.line}: {e.message}")
```

## Implementation Notes

### Rust Core

- Use `serde_yaml` for YAML parsing
- Use `serde_json` for JSON parsing
- Store parsed data as internal tree structure
- Implement `Index` trait for bracket access
- FFI exposes opaque `Config` handle to language bindings

### Language Bindings

- Python: PyO3 with `__getattr__` for dot notation
- JavaScript: NAPI-RS with Proxy for dot notation
- Go: Struct with method chaining

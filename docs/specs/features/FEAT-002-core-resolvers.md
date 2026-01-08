# FEAT-002: Core Resolvers

## Overview

Provide built-in resolvers for common value sources: environment variables, self-references within the config, local files, and remote URLs.

## User Stories

- As a developer, I want to read values from environment variables so I can configure my app per environment
- As a developer, I want to reference other config values so I can avoid duplication
- As a developer, I want to include external config files so I can organize large configurations
- As a developer, I want to fetch config from URLs so I can use centralized configuration

## Dependencies

- [ADR-002: Resolver Architecture](../../adr/ADR-002-resolver-architecture.md)
- [ADR-005: Resolver Timing (Lazy Resolution)](../../adr/ADR-005-resolver-timing.md)
- [ADR-011: Interpolation Syntax](../../adr/ADR-011-interpolation-syntax.md)
- [ADR-012: Type Coercion](../../adr/ADR-012-type-coercion.md)
- [FEAT-001: Configuration File Loading](FEAT-001-config-loading.md)

## Core Resolvers

### 1. Environment Resolver (`env`)

Reads values from environment variables.

**Syntax:**
```yaml
# Basic usage
port: ${env:PORT}

# With default value
port: ${env:PORT,8080}

# With nested default
port: ${env:PORT,${env:DEFAULT_PORT,8080}}
```

**Behavior:**
- Returns the environment variable value as a string
- If variable is not set and no default provided, raises `ResolverError`
- If variable is not set and default is provided, returns default
- Sensitivity: Not sensitive by default (env vars are often non-secret)

**Arguments:**

| Position | Name | Required | Description |
|----------|------|----------|-------------|
| 1 | name | Yes | Environment variable name |
| 2 | default | No | Default value if not set |

### 2. Self-Reference Resolver (implicit)

References other values within the same configuration.

**Syntax:**
```yaml
defaults:
  timeout: 30
  host: localhost

database:
  host: ${defaults.host}
  timeout: ${defaults.timeout}

  # Relative reference (sibling)
  connection_string: "postgres://${.host}:5432/db"

  # Relative reference (parent's sibling)
  api_timeout: ${..api.timeout}

api:
  timeout: 60
```

**Path Syntax:**
- `${path.to.value}` - Absolute path from config root
- `${.sibling}` - Relative path to sibling key
- `${..parent.key}` - Relative path going up one level
- `${...grandparent.key}` - Relative path going up two levels

**Behavior:**
- Resolves to the value at the specified path
- If path doesn't exist, raises `ResolverError`
- Circular references are detected and raise `CircularReferenceError`
- Sensitivity: Inherits sensitivity from the referenced value

**Array Access:**
```yaml
servers:
  - host: server1.example.com
  - host: server2.example.com

primary_host: ${servers[0].host}
```

### 3. File Resolver (`file`)

Reads content from local files.

**Syntax:**
```yaml
# Local file (text content)
readme: ${file:./README.md}

# Local file (parsed as YAML/JSON and merged)
extra_config: ${file:./extra.yaml}

# With encoding
binary_key: ${file:./key.pem,encoding=utf-8}
```

**Arguments:**

| Position | Name | Required | Description |
|----------|------|----------|-------------|
| 1 | path | Yes | Local file path |
| 2+ | options | No | Key=value options |

**Options:**

| Option | Values | Default | Description |
|--------|--------|---------|-------------|
| `encoding` | `utf-8`, `ascii`, `base64` | `utf-8` | Text encoding |
| `parse` | `yaml`, `json`, `text`, `auto` | `auto` | How to parse content |

**Behavior:**
- Paths are relative to the config file's directory
- When `parse=auto`, format is detected by file extension
- Parsed content (YAML/JSON) returns a Config object for nested access
- Text content returns a string
- Sensitivity: Not sensitive by default

**Security:**
- Local file access is sandboxed to config directory by default

```python
# Expand sandbox to include other directories
config = Config.load("config.yaml", file_roots=["/etc/myapp", "./config"])
```

### 4. HTTP Resolver (`http`)

Fetches content from remote URLs.

**Syntax:**
```yaml
# Fetch remote config
remote_config: ${http:https://config.example.com/shared.yaml}

# With options
remote_config: ${http:https://config.example.com/shared.yaml,timeout=60}

# With authentication header
remote_config: ${http:https://config.example.com/shared.yaml,header=Authorization:Bearer ${env:API_TOKEN}}
```

**Arguments:**

| Position | Name | Required | Description |
|----------|------|----------|-------------|
| 1 | url | Yes | HTTP or HTTPS URL |
| 2+ | options | No | Key=value options |

**Options:**

| Option | Values | Default | Description |
|--------|--------|---------|-------------|
| `timeout` | integer (seconds) | 30 | Request timeout |
| `parse` | `yaml`, `json`, `text`, `auto` | `auto` | How to parse content |
| `header` | `Name:Value` | - | HTTP header to include (repeatable) |

**Behavior:**
- Supports `http://` and `https://` URLs
- When `parse=auto`, format is detected by Content-Type header or URL extension
- Parsed content (YAML/JSON) returns a Config object for nested access
- Text content returns a string
- Sensitivity: Not sensitive by default

**Security:**
- **Disabled by default** to prevent SSRF attacks
- Must be explicitly enabled at load time
- Consider using URL allowlists in production

```python
# Enable HTTP resolver
config = Config.load("config.yaml", allow_http=True)

# With URL allowlist (recommended for production)
config = Config.load(
    "config.yaml",
    allow_http=True,
    http_allowlist=["https://config.internal/*", "https://api.example.com/config/*"]
)
```

## API Surface

### Registering Resolvers

Core resolvers are registered automatically. Custom resolvers can be added:

```python
from holoconf import Config, Resolver, ResolvedValue

# Simple function resolver
def my_resolver(key: str, default: str = None) -> str:
    value = lookup(key)
    return value if value else default

Config.register_resolver("myresolver", my_resolver)

# Class resolver with sensitivity
class VaultResolver(Resolver):
    def resolve(self, path: str) -> ResolvedValue:
        secret = self.client.read(path)
        return ResolvedValue(value=secret, sensitive=True)

Config.register_resolver("vault", VaultResolver())
```

### Resolver Arguments

Resolvers receive arguments as parsed from the interpolation:

```yaml
value: ${resolver:arg1,arg2,key=value}
```

```python
def my_resolver(arg1: str, arg2: str, key: str = None) -> str:
    # arg1 = "arg1", arg2 = "arg2", key = "value"
    ...
```

## Behavior

### Resolution Timing

Per [ADR-005](../../adr/ADR-005-resolver-timing.md), resolution is lazy:

```python
config = Config.load("config.yaml")  # No resolution yet

# Resolution happens on access
port = config.port  # ${env:PORT} resolved here
```

### Caching

Resolved values are memoized per config instance:

```python
config.port  # Resolves ${env:PORT}
config.port  # Returns cached value, no re-resolution

# Different config instance = independent cache
config2 = Config.load("config.yaml")
config2.port  # Resolves again
```

### Parallel Resolution

When using `resolve_all()`, independent resolutions happen in parallel:

```python
await config.resolve_all()  # Resolves all values in parallel where possible
```

### Circular Reference Detection

```yaml
a: ${b}
b: ${c}
c: ${a}  # Circular!
```

```
CircularReferenceError: Circular reference detected
  Path: c
  Chain: a → b → c → a
  Help: Break the circular dependency
```

## Error Cases

### Missing Environment Variable

```yaml
port: ${env:UNDEFINED_VAR}
```

```
ResolverError: Environment variable not found
  Resolver: env
  Key: UNDEFINED_VAR
  Path: port
  Help: Set the UNDEFINED_VAR environment variable or provide a default: ${env:UNDEFINED_VAR,default}
```

### Invalid Self-Reference Path

```yaml
value: ${nonexistent.path}
```

```
ResolverError: Referenced path not found
  Resolver: self
  Key: nonexistent.path
  Path: value
  Help: Check that 'nonexistent.path' exists in the configuration
```

### File Not Found

```yaml
content: ${file:./missing.txt}
```

```
ResolverError: File not found
  Resolver: file
  Key: ./missing.txt
  Path: content
  Help: Check that the file exists relative to the config file
```

### HTTP Request Error

```yaml
config: ${http:https://example.com/config.yaml}
```

```
ResolverError: Failed to fetch remote configuration
  Resolver: http
  Key: https://example.com/config.yaml
  Path: config
  Cause: HTTP 404 Not Found
  Help: Check the URL is correct and accessible
```

### HTTP Resolver Disabled

```yaml
config: ${http:https://example.com/config.yaml}
```

```
ResolverError: HTTP resolver is disabled
  Resolver: http
  Key: https://example.com/config.yaml
  Path: config
  Help: Enable HTTP resolver with Config.load(..., allow_http=True)
```

### HTTP URL Not in Allowlist

```yaml
config: ${http:https://untrusted.com/config.yaml}
```

```
ResolverError: URL not in allowlist
  Resolver: http
  Key: https://untrusted.com/config.yaml
  Path: config
  Allowlist: https://config.internal/*, https://api.example.com/config/*
  Help: Add the URL to http_allowlist or check for typos
```

## Examples

### Environment-Based Configuration

```yaml
# config.yaml
database:
  host: ${env:DB_HOST,localhost}
  port: ${env:DB_PORT,5432}
  username: ${env:DB_USER}
  password: ${env:DB_PASSWORD}

logging:
  level: ${env:LOG_LEVEL,info}
```

```python
import os
os.environ["DB_HOST"] = "prod-db.example.com"
os.environ["DB_USER"] = "admin"
os.environ["DB_PASSWORD"] = "secret"

config = Config.load("config.yaml")
print(config.database.host)  # "prod-db.example.com"
print(config.database.port)  # "5432" (default)
```

### DRY Configuration with Self-References

```yaml
# config.yaml
defaults:
  region: us-east-1
  environment: production

aws:
  region: ${defaults.region}

  s3:
    bucket: myapp-${defaults.environment}-${defaults.region}

  dynamodb:
    table: myapp-${defaults.environment}-data
```

### Including External Files

```yaml
# config.yaml
app:
  name: myapp

# Include shared database config
database: ${file:./database.yaml}

# Include environment-specific overrides
overrides: ${file:./envs/${env:ENVIRONMENT,development}.yaml}
```

```yaml
# database.yaml
host: localhost
port: 5432
pool_size: 10
```

### Remote Configuration

```yaml
# config.yaml
# Fetch shared config from config server
shared: ${http:https://config.internal/shared/v1.yaml}

# Override with local values
local:
  feature_flags: ${http:https://config.internal/flags/${env:APP_ENV}.json}
```

```python
config = Config.load("config.yaml", allow_http=True)
```

## Implementation Notes

### Rust Core

- `env` resolver: Use `std::env::var`
- `self` resolver: Tree traversal with path parsing
- `file` resolver: Use `std::fs::read_to_string`
- `http` resolver: Use `reqwest` with async support
- Circular detection: Track resolution stack, error if path revisited

### Security Considerations

- HTTP resolver disabled by default to prevent SSRF
- Local file access sandboxed by default
- URL allowlists for HTTP resolver in production
- Log resolver calls for audit trail (opt-in)

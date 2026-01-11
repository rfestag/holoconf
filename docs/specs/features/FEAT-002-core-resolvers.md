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

**Arguments:**

| Position | Name | Required | Description |
|----------|------|----------|-------------|
| 1 | name | Yes | Environment variable name |

**Keyword Arguments:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `default` | any | none | Default value if variable is not set |
| `sensitive` | bool | `false` | Mark the resolved value as sensitive |

**Behavior:**
- Returns the environment variable value as a string
- If variable is not set and no default provided, raises `ResolverError`
- If variable is not set and default is provided, returns default
- Not sensitive by default; use `sensitive=true` for secrets

**Examples:**
```yaml
# Basic usage
port: ${env:PORT}

# With default value
port: ${env:PORT,default=8080}

# Mark as sensitive (for secrets)
db_password: ${env:DB_PASSWORD,sensitive=true}

# Combined default and sensitive
api_key: ${env:API_KEY,default=dev-key,sensitive=true}

# With nested default
port: ${env:PORT,default=${env:DEFAULT_PORT,default=8080}}
```

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

**Keyword Arguments:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `default` | any | none | Default value if path doesn't exist |
| `sensitive` | bool | inherited | Override sensitivity (inherits from referenced value by default) |

**Behavior:**
- Resolves to the value at the specified path
- If path doesn't exist and no default provided, raises `ResolverError`
- If path doesn't exist and default is provided, returns default
- Circular references are detected and raise `CircularReferenceError`
- Sensitivity is inherited from the referenced value by default; can be overridden

**Array Access:**
```yaml
servers:
  - host: server1.example.com
  - host: server2.example.com

primary_host: ${servers[0].host}
```

**Examples:**
```yaml
# Basic reference
timeout: ${defaults.timeout}

# With default for optional config
feature_timeout: ${features.timeout,default=30}

# Sensitivity inherited from referenced value
secrets:
  api_key: ${env:API_KEY,sensitive=true}

derived_key: ${secrets.api_key}  # Inherits sensitive=true

# Override sensitivity (rare, use with caution)
public_ref: ${secrets.api_key,sensitive=false}
```

### 3. File Resolver (`file`)

Reads content from local files.

**Arguments:**

| Position | Name | Required | Description |
|----------|------|----------|-------------|
| 1 | path | Yes | Local file path (relative to config file directory) |

**Keyword Arguments:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `default` | any | none | Default value if file doesn't exist |
| `parse` | string | `auto` | How to interpret content: `auto`, `yaml`, `json`, `text`, `binary` |
| `encoding` | string | `utf-8` | Text encoding: `utf-8`, `ascii`, `latin-1` (ignored for `binary`) |
| `sensitive` | bool | `false` | Mark the resolved value as sensitive |

**Parse Modes:**

| Mode | Return Type | Description |
|------|-------------|-------------|
| `auto` | varies | Detect by file extension (`.yaml`, `.yml`, `.json` → parsed; else → text) |
| `yaml` | structured data | Parse as YAML, accessible via dot notation |
| `json` | structured data | Parse as JSON, accessible via dot notation |
| `text` | string | Return raw text content |
| `binary` | bytes | Return raw bytes (`bytes` in Python, `Vec<u8>` in Rust) |

**Behavior:**
- Paths are relative to the config file's directory
- If file doesn't exist and no default provided, raises `ResolverError`
- If file doesn't exist and default is provided, returns default
- When `parse=auto`, format is detected by file extension
- Parsed content (YAML/JSON) returns a Config object for nested access
- Text content returns a string
- Binary content returns raw bytes (useful for certificates, keys, images)
- Not sensitive by default; use `sensitive=true` for secrets

**Examples:**
```yaml
# Auto-detect format by extension
config: ${file:./extra.yaml}

# With default if file doesn't exist
config: ${file:./optional.yaml,default={}}

# Explicit text mode
readme: ${file:./README.md,parse=text}

# Binary file (certificates, keys, etc.)
certificate: ${file:./ca.pem,parse=binary}
p12_cert: ${file:./client.p12,parse=binary,sensitive=true}

# Different encoding for legacy files
legacy_config: ${file:./old.txt,encoding=latin-1}

# Mark as sensitive
secret_key: ${file:./secret.key,sensitive=true}
```

**Security:**
- Local file access is sandboxed to config directory by default

```python
# Expand sandbox to include other directories
config = Config.load("config.yaml", file_roots=["/etc/myapp", "./config"])
```

### 4. HTTP Resolver (`http`)

Fetches content from remote URLs.

**Arguments:**

| Position | Name | Required | Description |
|----------|------|----------|-------------|
| 1 | url | Yes | HTTP or HTTPS URL |

**Keyword Arguments:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `default` | any | none | Default value if request fails |
| `parse` | string | `auto` | How to interpret content: `auto`, `yaml`, `json`, `text`, `binary` |
| `encoding` | string | `utf-8` | Text encoding: `utf-8`, `ascii`, `latin-1` (ignored for `binary`) |
| `timeout` | int | 30 | Request timeout in seconds |
| `header` | string | - | HTTP header to include as `Name:Value` (repeatable) |
| `sensitive` | bool | `false` | Mark the resolved value as sensitive |

**Parse Modes:**

| Mode | Return Type | Description |
|------|-------------|-------------|
| `auto` | varies | Detect by Content-Type header or URL extension |
| `yaml` | structured data | Parse as YAML, accessible via dot notation |
| `json` | structured data | Parse as JSON, accessible via dot notation |
| `text` | string | Return raw text content |
| `binary` | bytes | Return raw bytes (`bytes` in Python, `Vec<u8>` in Rust) |

**Behavior:**
- Supports `http://` and `https://` URLs
- If request fails and no default provided, raises `ResolverError`
- If request fails and default is provided, returns default
- When `parse=auto`, format is detected by Content-Type header or URL extension
- Parsed content (YAML/JSON) returns a Config object for nested access
- Text content returns a string
- Binary content returns raw bytes (useful for certificates, images)
- Not sensitive by default; use `sensitive=true` for secrets

**Examples:**
```yaml
# Fetch remote config (auto-detect format)
remote_config: ${http:https://config.example.com/shared.yaml}

# With default if request fails
remote_config: ${http:https://config.example.com/shared.yaml,default={}}

# With timeout
remote_config: ${http:https://config.example.com/shared.yaml,timeout=60}

# With authentication header
remote_config: ${http:https://config.example.com/shared.yaml,header=Authorization:Bearer ${env:API_TOKEN}}

# Explicit JSON parsing
api_config: ${http:https://api.example.com/config,parse=json}

# Binary content (certificate from URL)
ca_cert: ${http:https://pki.internal/ca.pem,parse=binary}

# Mark as sensitive
secret_config: ${http:https://vault.internal/config,sensitive=true}
```

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
  Help: Set the UNDEFINED_VAR environment variable or provide a default: ${env:UNDEFINED_VAR,default=value}
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
  host: ${env:DB_HOST,default=localhost}
  port: ${env:DB_PORT,default=5432}
  username: ${env:DB_USER}
  password: ${env:DB_PASSWORD,sensitive=true}

logging:
  level: ${env:LOG_LEVEL,default=info}
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
overrides: ${file:./envs/${env:ENVIRONMENT,default=development}.yaml}
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

## Transformation Resolvers

These resolvers transform string values into structured data. They are useful for parsing JSON or YAML stored in environment variables, SSM parameters, or other string sources.

### 5. JSON Resolver (`json`)

Parses a JSON string into a structured value.

**Syntax:**
```yaml
# Parse JSON from environment variable
settings: ${json:${env:SETTINGS_JSON}}

# Access nested values after parsing
db_host: ${json:${env:DB_CONFIG}}.host

# With sensitivity override
secrets: ${json:${ssm:/app/secrets},sensitive=false}
```

**Arguments:**

| Position | Name | Required | Description |
|----------|------|----------|-------------|
| 1 | value | Yes | JSON string to parse |

**Keyword Arguments:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `sensitive` | bool | inherited | Override sensitivity (inherits from input by default) |

**Behavior:**
- Parses strict JSON (no trailing commas, no comments)
- Supports all JSON root types: object, array, string, number, boolean, null
- Returns structured data accessible via dot notation
- Parse errors raise `ResolverError` with line/column information and truncated input preview (first 50 chars)
- Sensitivity is inherited from the input value by default; can be overridden

**Error Example:**
```
ResolverError: Invalid JSON at line 1, column 15: expected ':' but found '}'
  Resolver: json
  Input preview: {"invalid json}
  Path: settings
  Help: Check that the input is valid JSON
```

### 6. YAML Resolver (`yaml`)

Parses a YAML string into a structured value.

**Syntax:**
```yaml
# Parse YAML from environment variable
config: ${yaml:${env:CONFIG_YAML}}

# Parse YAML from file content
settings: ${yaml:${file:./settings.txt}}

# With sensitivity override
secrets: ${yaml:${ssm:/app/secrets},sensitive=false}
```

**Arguments:**

| Position | Name | Required | Description |
|----------|------|----------|-------------|
| 1 | value | Yes | YAML string to parse |

**Keyword Arguments:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `sensitive` | bool | inherited | Override sensitivity (inherits from input by default) |

**Behavior:**
- Parses the first YAML document only (ignores `---` separated documents)
- Preserves YAML's native type coercion (`yes` → boolean, `1.0` → float, etc.)
- Returns structured data accessible via dot notation
- Parse errors raise `ResolverError` with position information and truncated input preview
- Sensitivity is inherited from the input value by default; can be overridden

**Error Example:**
```
ResolverError: Invalid YAML at line 3, column 5: mapping values are not allowed here
  Resolver: yaml
  Input preview: "key: value\n  invalid: - item"
  Path: config
  Help: Check that the input is valid YAML
```

### 7. Split Resolver (`split`)

Splits a string into an array of strings using a delimiter.

**Syntax:**
```yaml
# Default delimiter (comma), with whitespace trimming
hosts: ${split:${env:DB_HOSTS}}
# Input: "host1, host2, host3" → ["host1", "host2", "host3"]

# Custom delimiter
path_parts: ${split:${env:PATH},delim=:}
# Input: "/usr/bin:/usr/local/bin" → ["/usr/bin", "/usr/local/bin"]

# Disable trimming
raw_values: ${split:${env:VALUES},trim=false}
# Input: "a, b, c" → ["a", " b", " c"]

# Skip empty elements
non_empty: ${split:${env:LIST},skip_empty=true}
# Input: "a,,b" → ["a", "b"]

# Limit number of splits
key_value: ${split:${env:PAIR},delim==,limit=2}
# Input: "key=value=with=equals" → ["key", "value=with=equals"]
```

**Arguments:**

| Position | Name | Required | Description |
|----------|------|----------|-------------|
| 1 | value | Yes | String to split |

**Keyword Arguments:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `delim` | string | `,` | Delimiter to split on |
| `trim` | bool | `true` | Trim whitespace from each element |
| `skip_empty` | bool | `false` | Remove empty strings from result |
| `limit` | int | none | Maximum number of splits (results in at most `limit + 1` elements) |
| `sensitive` | bool | inherited | Override sensitivity (inherits from input by default) |

**Behavior:**
- Splits string by delimiter (no escape sequence support)
- Trims whitespace from each element by default
- Empty input string returns empty array `[]`
- Empty elements are preserved by default (use `skip_empty=true` to filter)
- Always returns an array of strings (no type coercion)
- Sensitivity is inherited from the input value by default; can be overridden

**Examples:**

```yaml
# Environment: DB_HOSTS="primary.db.local, replica1.db.local, replica2.db.local"
database:
  hosts: ${split:${env:DB_HOSTS}}
  # Result: ["primary.db.local", "replica1.db.local", "replica2.db.local"]

  primary: ${split:${env:DB_HOSTS}}[0]
  # Result: "primary.db.local"

# Environment: FEATURES="dark_mode,,beta_ui,new_checkout"
features:
  all: ${split:${env:FEATURES}}
  # Result: ["dark_mode", "", "beta_ui", "new_checkout"]

  enabled: ${split:${env:FEATURES},skip_empty=true}
  # Result: ["dark_mode", "beta_ui", "new_checkout"]

# Environment: CONNECTION="user:password:host:5432:database"
connection:
  parts: ${split:${env:CONNECTION},delim=:,limit=4}
  # Result: ["user", "password", "host", "5432", "database"]
  # Note: limit=4 means 4 splits, resulting in 5 elements max
```

## Implementation Notes

### Rust Core

- `env` resolver: Use `std::env::var`
- `self` resolver: Tree traversal with path parsing
- `file` resolver: Use `std::fs::read` for binary, `std::fs::read_to_string` for text
- `http` resolver: Use `reqwest` with async support, `bytes()` for binary mode
- `json` resolver: Use `serde_json::from_str`
- `yaml` resolver: Use `serde_yaml::from_str` (first document only)
- `split` resolver: Use `str::split` with trim/filter options
- Circular detection: Track resolution stack, error if path revisited
- Binary values: Represent as `Vec<u8>` in Rust, `bytes` in Python

### Security Considerations

- HTTP resolver disabled by default to prevent SSRF
- Local file access sandboxed by default
- URL allowlists for HTTP resolver in production
- Log resolver calls for audit trail (opt-in)
- Transformation resolvers inherit sensitivity from input to prevent accidental exposure
- All resolvers support `sensitive` keyword for explicit marking

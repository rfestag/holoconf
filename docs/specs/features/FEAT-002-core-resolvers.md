# FEAT-002: Core Resolvers

## Status

Implemented

## Changelog

- 2026-01-17: Marked as Implemented (v0.2.0)

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

Reads content from local files. Supports RFC 8089 file: URI syntax.

**Arguments:**

| Position | Name | Required | Description |
|----------|------|----------|-------------|
| 1 | path | Yes | Local file path (relative to config file directory) or RFC 8089 file: URI |

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
- Paths are relative to the config file's directory by default
- Supports RFC 8089 file: URI syntax for explicit path specifications:
  - `file:///path/to/file` - Absolute path with empty authority (localhost)
  - `file://localhost/path/to/file` - Absolute path with explicit localhost
  - `file:/path/to/file` - Absolute path (minimal form)
  - `file://remote/path` - Remote file URIs are **rejected** with an error
- Plain paths (no `file://` prefix) work as before:
  - Relative paths: `./config.yaml`, `../shared/data.json`
  - Absolute paths: `/etc/app/config.yaml` (subject to `file_roots` security)
- If file doesn't exist and no default provided, raises `ResolverError`
- If file doesn't exist and default is provided, returns default
- When `parse=auto`, format is detected by file extension
- Parsed content (YAML/JSON) returns a Config object for nested access
- Text content returns a string
- Binary content returns raw bytes (useful for certificates, keys, images)
- Not sensitive by default; use `sensitive=true` for secrets

**Examples:**
```yaml
# Relative paths (traditional syntax)
config: ${file:./extra.yaml}
shared: ${file:../shared/common.yaml}

# RFC 8089 file: URI syntax (absolute paths)
system_config: ${file:///etc/myapp/config.yaml}
local_file: ${file://localhost/var/lib/myapp/data.json}
minimal_form: ${file:/opt/app/settings.yaml}

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

**Security (Path Traversal Protection):**

By default, file access is restricted to the config file's parent directory to prevent path traversal attacks:

```python
# Auto-allowed: files in same directory as config
config = Config.load("/app/config.yaml")
# Can read: /app/data.txt, /app/subdir/file.txt
# BLOCKED: /etc/passwd, /other/path/file.txt
```

The restriction applies to both relative and absolute paths:
- Relative paths are resolved relative to the config file's directory
- Absolute paths must be within allowed roots
- Symlinks are resolved and checked against allowed roots

To access files outside the config directory, explicitly allow additional roots:

```python
# Allow access to multiple directories
config = Config.load(
    "/app/config.yaml",
    file_roots=["/etc/myapp", "/var/lib/myapp"]
)
```

When loading from a string with `loads()`, specify `base_path` to set the sandbox root:

```python
config = Config.loads(
    yaml_string,
    base_path="/app/config",  # Files resolved relative to this
    file_roots=["/etc/myapp"]  # Additional allowed roots
)
```

When merging configs, file_roots are combined (union):

```python
config1 = Config.load("/app/config.yaml")  # Allows /app
config2 = Config.load("/etc/config.yaml", file_roots=["/var/lib"])  # Allows /etc, /var/lib
config1.merge(config2)  # Now allows /app, /etc, /var/lib
```

### 4. HTTP Resolver (`http`)

Fetches content from remote HTTP URLs.

**Arguments:**

| Position | Name | Required | Description |
|----------|------|----------|-------------|
| 1 | url | Yes | HTTP URL (auto-prepends `http://` if not present) |

**Keyword Arguments:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `default` | any | none | Default value if request fails |
| `parse` | string | `auto` | How to interpret content: `auto`, `yaml`, `json`, `text`, `binary` |
| `encoding` | string | `utf-8` | Text encoding: `utf-8`, `ascii`, `latin-1` (ignored for `binary`) |
| `timeout` | int | 30 | Request timeout in seconds |
| `header` | string | - | HTTP header to include as `Name:Value` (repeatable) |
| `sensitive` | bool | `false` | Mark the resolved value as sensitive |
| `proxy` | string | - | HTTP or SOCKS proxy URL (overrides config-level `http_proxy`) |
| `ca_bundle` | string | - | CA bundle file path or PEM content (replaces default roots, overrides `http_ca_bundle`) |
| `extra_ca_bundle` | string | - | Extra CA file path or PEM content (adds to default roots, overrides `http_extra_ca_bundle`) |
| `client_cert` | string/bytes | - | Client cert file path, PEM content, or P12/PFX binary |
| `client_key` | string | - | Client key file path or PEM content (not needed for P12/PFX) |
| `key_password` | string | - | Password for encrypted key or P12/PFX |
| `insecure` | bool | `false` | Skip TLS verification (DANGEROUS, dev only) |

**Parse Modes:**

| Mode | Return Type | Description |
|------|-------------|-------------|
| `auto` | varies | Detect by Content-Type header or URL extension |
| `yaml` | structured data | Parse as YAML, accessible via dot notation |
| `json` | structured data | Parse as JSON, accessible via dot notation |
| `text` | string | Return raw text content |
| `binary` | bytes | Return raw bytes (`bytes` in Python, `Vec<u8>` in Rust) |

**Behavior:**
- Auto-prepends `http://` scheme to the URL argument
- Strips any existing `http://` or `https://` prefix before prepending
- Strips leading `//` if present (e.g., `${http://example.com}` → `http://example.com`)
- If request fails and no default provided, raises `ResolverError`
- If request fails and default is provided, returns default
- When `parse=auto`, format is detected by Content-Type header or URL extension
- Parsed content (YAML/JSON) returns a Config object for nested access
- Text content returns a string
- Binary content returns raw bytes (useful for certificates, images)
- Not sensitive by default; use `sensitive=true` for secrets

**URL Normalization Examples:**
```yaml
# Clean syntax (recommended) - auto-prepends http://
remote_config: ${http:example.com/config.yaml}
# Resolves to: http://example.com/config.yaml

# With protocol prefix (backwards compatible) - strips and re-prepends
remote_config: ${http:http://example.com/config.yaml}
# Resolves to: http://example.com/config.yaml

# With double slashes (backwards compatible)
remote_config: ${http://example.com/config.yaml}
# Resolves to: http://example.com/config.yaml
```

**Examples:**
```yaml
# Fetch remote config (auto-detect format)
remote_config: ${http:example.com/shared.yaml}

# With default if request fails
remote_config: ${http:example.com/shared.yaml,default={}}

# With timeout
remote_config: ${http:example.com/shared.yaml,timeout=60}

# With authentication header
remote_config: ${http:api.example.com/config,header=Authorization:Bearer ${env:API_TOKEN}}

# Parse JSON response using transformation resolver
api_config: ${json:${http:api.example.com/config}}

# Binary content (certificate from URL)
ca_cert: ${http:pki.internal/ca.pem,parse=binary}

# Mark as sensitive
secret_config: ${http:vault.internal/config,sensitive=true}
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

**TLS/Proxy Configuration:**

Config-level options apply to all HTTP requests. Per-request kwargs override config-level settings.

| Option | Type | Description |
|--------|------|-------------|
| `http_proxy` | string | HTTP or SOCKS proxy URL (e.g., `http://proxy:8080`, `socks5://proxy:1080`) |
| `http_proxy_from_env` | bool | Auto-detect proxy from `HTTP_PROXY`/`HTTPS_PROXY` environment variables |
| `http_ca_bundle` | str/bytes | CA bundle file path or PEM content (replaces default root certificates) |
| `http_extra_ca_bundle` | str/bytes | Extra CA file path or PEM content (adds to default root certificates) |
| `http_client_cert` | str/bytes | Client cert file path, PEM content, or P12/PFX binary for mTLS |
| `http_client_key` | str/bytes | Client key file path or PEM content for mTLS (not needed for P12/PFX) |
| `http_client_key_password` | string | Password for encrypted private key or P12/PFX bundle |
| `http_insecure` | bool | Skip TLS verification (DANGEROUS, dev only) |

**Supported Key/Certificate Formats:**
- Unencrypted PEM certificate and key files (file paths or content)
- Encrypted PKCS#8 PEM private keys (password protected, file paths or content)
- P12/PFX bundles containing certificate and key (password protected, file paths or binary content)

**Certificate/Key Input Types:**
- **File paths** (str): `/path/to/cert.pem`, `./relative/key.pem`, `/path/to/identity.p12`
- **PEM content** (str): String containing `-----BEGIN CERTIFICATE-----` or `-----BEGIN PRIVATE KEY-----`
- **P12/PFX binary** (bytes): Binary P12/PFX data (Python only via `bytes` type)

**Auto-Detection:**
- String with `-----BEGIN` → Parsed as PEM content
- String ending in `.p12`/`.pfx` → Read as P12 file
- Bytes → Parsed as P12 binary
- Otherwise → Read as file path

```python
# === Traditional File Paths ===

# mTLS with PEM files
config = Config.load(
    "config.yaml",
    allow_http=True,
    http_client_cert="/path/to/client.pem",
    http_client_key="/path/to/client-key.pem"
)

# mTLS with encrypted key
config = Config.load(
    "config.yaml",
    allow_http=True,
    http_client_cert="/path/to/client.pem",
    http_client_key="/path/to/client-key.pem",
    http_client_key_password="secret"
)

# mTLS with P12/PFX bundle
config = Config.load(
    "config.yaml",
    allow_http=True,
    http_client_cert="/path/to/identity.p12",
    http_client_key_password="secret"
)

# Custom CA for internal services
config = Config.load(
    "config.yaml",
    allow_http=True,
    http_extra_ca_bundle="/etc/ssl/certs/internal-ca.pem"
)

# === Certificate Variables (PEM Content) ===

import os

# PEM certificate from environment variable
cert_pem = os.getenv("CLIENT_CERT_PEM")  # Contains -----BEGIN CERTIFICATE-----
key_pem = os.getenv("CLIENT_KEY_PEM")    # Contains -----BEGIN PRIVATE KEY-----

config = Config.load(
    "config.yaml",
    allow_http=True,
    http_client_cert=cert_pem,  # Auto-detected as PEM content
    http_client_key=key_pem      # Auto-detected as PEM content
)

# CA bundle from environment variable
ca_bundle_pem = os.getenv("INTERNAL_CA_BUNDLE")
config = Config.load(
    "config.yaml",
    allow_http=True,
    http_extra_ca_bundle=ca_bundle_pem  # Auto-detected as PEM content
)

# === Certificate Variables (P12 Binary) ===

# P12 from binary (Python only)
with open("/path/to/identity.p12", "rb") as f:
    p12_bytes = f.read()

config = Config.load(
    "config.yaml",
    allow_http=True,
    http_client_cert=p12_bytes,  # Auto-detected as P12 binary
    http_client_key_password="secret"
)

# Proxy configuration
config = Config.load(
    "config.yaml",
    allow_http=True,
    http_proxy="http://proxy.corp.com:8080"
)
```

**Per-request overrides in YAML:**

```yaml
# Override proxy for specific request
value: ${https:api.example.com/config,proxy=http://proxy:8080}

# mTLS for specific request (file paths)
value: ${https:secure.corp/config,client_cert=/path/cert.pem,client_key=/path/key.pem}

# Custom CA for specific request
value: ${https:internal.corp/config,extra_ca_bundle=/path/to/ca.pem}

# === Certificate Variables ===

# mTLS with PEM from environment variables
secure_api: ${https:api.corp.com/config,client_cert=${env:CLIENT_CERT_PEM},client_key=${env:CLIENT_KEY_PEM}}

# CA bundle from file resolver
internal_config: ${https:internal.corp/config,ca_bundle=${file:./ca-bundle.pem,parse=text}}

# P12 binary from file resolver (Python only)
identity_data: ${https:secure.example.com/data,client_cert=${file:./identity.p12,parse=binary},key_password=${env:P12_PASSWORD}}

# Mixed: PEM cert from env + key from file path
mixed_mode: ${https:api.example.com/config,client_cert=${env:CERT_PEM},client_key=/etc/ssl/private/key.pem}
```

### 5. HTTPS Resolver (`https`)

Fetches content from remote HTTPS URLs. This resolver is nearly identical to the `http` resolver but auto-prepends `https://` instead of `http://`.

**Arguments:**

| Position | Name | Required | Description |
|----------|------|----------|-------------|
| 1 | url | Yes | HTTPS URL (auto-prepends `https://` if not present) |

**Keyword Arguments:**

Same as the `http` resolver - supports all the same kwargs for parsing, timeouts, headers, TLS configuration, proxies, and sensitivity.

**Behavior:**
- Auto-prepends `https://` scheme to the URL argument
- Strips any existing `http://` or `https://` prefix before prepending
- Strips leading `//` if present (e.g., `${https://example.com}` → `https://example.com`)
- All other behavior identical to `http` resolver (defaults, parsing, etc.)

**URL Normalization Examples:**
```yaml
# Clean syntax (recommended) - auto-prepends https://
remote_config: ${https:api.example.com/config.yaml}
# Resolves to: https://api.example.com/config.yaml

# With protocol prefix (backwards compatible) - strips and re-prepends
remote_config: ${https:https://api.example.com/config.yaml}
# Resolves to: https://api.example.com/config.yaml

# With double slashes (backwards compatible)
remote_config: ${https://api.example.com/config.yaml}
# Resolves to: https://api.example.com/config.yaml

# Even strips wrong protocol and uses https
remote_config: ${https:http://api.example.com/config.yaml}
# Resolves to: https://api.example.com/config.yaml
```

**Examples:**
```yaml
# Fetch remote config (auto-detect format)
remote_config: ${https:config.example.com/shared.yaml}

# With default if request fails
remote_config: ${https:api.example.com/config.json,default={}}

# With timeout
remote_config: ${https:api.example.com/config,timeout=60}

# With authentication header
api_token: ${https:vault.corp.com/token,header=Authorization:Bearer ${env:VAULT_TOKEN}}

# Parse YAML response using transformation resolver
settings: ${yaml:${https:config.internal/app.yaml}}

# Mark as sensitive (for secrets)
database_password: ${https:secrets.internal/db-pass,sensitive=true}

# mTLS with file paths
secure_config: ${https:api.corp.com/config,client_cert=/path/cert.pem,client_key=/path/key.pem}

# mTLS with certificate variables
secure_api: ${https:api.corp.com/data,client_cert=${env:CLIENT_CERT_PEM},client_key=${env:CLIENT_KEY_PEM}}
```

**Security:**
- Same security model as `http` resolver
- **Disabled by default** to prevent SSRF attacks
- Must be explicitly enabled with `allow_http=True` (despite the name, this enables both http and https)
- URL allowlists apply to both http and https resolvers

```python
# Enable HTTPS resolver (same as HTTP)
config = Config.load("config.yaml", allow_http=True)

# With URL allowlist (recommended for production)
config = Config.load(
    "config.yaml",
    allow_http=True,
    http_allowlist=["https://config.internal/*", "https://api.example.com/*"]
)
```

**Note:** While the http and https resolvers are separate, they share the same configuration settings (`allow_http`, `http_allowlist`, `http_proxy`, TLS options, etc.). The only difference is which protocol scheme is prepended to the URL.

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
config: ${https:example.com/config.yaml}
```

```
ResolverError: Failed to fetch remote configuration
  Resolver: https
  Key: example.com/config.yaml
  Path: config
  Cause: HTTP 404 Not Found
  Help: Check the URL is correct and accessible
```

### HTTP Resolver Disabled

```yaml
config: ${https:example.com/config.yaml}
```

```
ResolverError: HTTPS resolver is disabled
  Resolver: https
  Key: example.com/config.yaml
  Path: config
  Help: Enable HTTPS resolver with Config.load(..., allow_http=True)
```

### HTTP URL Not in Allowlist

```yaml
config: ${https:untrusted.com/config.yaml}
```

```
ResolverError: URL not in allowlist
  Resolver: https
  Key: untrusted.com/config.yaml
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
shared: ${https:config.internal/shared/v1.yaml}

# Override with local values
local:
  feature_flags: ${https:config.internal/flags/${env:APP_ENV}.json}
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

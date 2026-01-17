# Resolvers

## Overview

Resolvers are the mechanism HoloConf uses to dynamically compute configuration values. Each resolver handles a specific type of value source.

All resolvers support two framework-level keyword arguments:

- **`default=value`** - Fallback value if resolution fails
- **`sensitive=true`** - Mark value as sensitive for redaction

## Built-in Resolvers

### Environment Variables (`env`)

Reads values from environment variables.

```yaml
database:
  host: ${env:DB_HOST}
  port: ${env:DB_PORT,default=5432}
  password: ${env:DB_PASSWORD,sensitive=true}
```

=== "Python"

    ```python
    import os
    from holoconf import Config

    os.environ["DB_HOST"] = "production-db.example.com"
    config = Config.from_file("config.yaml")

    host = config.get("database.host")  # "production-db.example.com"
    port = config.get("database.port")  # "5432" (default, since DB_PORT not set)
    ```

=== "Rust"

    ```rust
    use holoconf::Config;
    use std::env;

    fn main() -> Result<(), holoconf::Error> {
        env::set_var("DB_HOST", "production-db.example.com");
        let config = Config::from_file("config.yaml")?;

        let host: String = config.get("database.host")?;
        let port: String = config.get("database.port")?;

        Ok(())
    }
    ```

### Self-References

Reference other values within the same configuration.

```yaml
base_url: https://api.example.com

endpoints:
  users: ${base_url}/users
  orders: ${base_url}/orders
```

#### Relative References

Use `.` for sibling references and `..` to go up levels:

```yaml
database:
  host: localhost
  port: 5432
  connection:
    # Reference sibling 'host' (same level)
    url: postgres://${.host}:${.port}/mydb
    # Reference parent's sibling
    timeout: ${..defaults.timeout}
```

### File Include (`file`)

Include content from other files.

```yaml
# main.yaml
app:
  name: my-app
  secrets: ${file:./secrets.yaml}
  # With default if file doesn't exist
  optional_config: ${file:./local.yaml,default={}}
```

#### File Resolver Options

| Option | Description | Example |
|--------|-------------|---------|
| `parse=yaml` | Parse as YAML | `${file:data.txt,parse=yaml}` |
| `parse=json` | Parse as JSON | `${file:data.txt,parse=json}` |
| `parse=text` | Read as plain text | `${file:data.json,parse=text}` |
| `parse=auto` | Auto-detect from extension (default) | `${file:config.yaml}` |
| `encoding=utf-8` | UTF-8 encoding (default) | `${file:data.txt}` |
| `encoding=base64` | Base64 encode contents | `${file:cert.pem,encoding=base64}` |
| `encoding=binary` | Return raw bytes | `${file:image.png,encoding=binary}` |

### HTTP (`http`)

!!! warning "Security"
    HTTP resolver is disabled by default for security. Enable it explicitly in your configuration options.

Fetch configuration from HTTP endpoints.

```yaml
feature_flags: ${http:https://config.example.com/flags.json}
# With fallback if request fails
remote_config: ${http:https://api.example.com/config,default={}}
# With authentication header
private_config: ${http:https://api.example.com/config,header=Authorization:Bearer token}
# With explicit parse mode
raw_data: ${http:https://api.example.com/data,parse=text}
```

#### Enabling HTTP Resolver

=== "Python"

    ```python
    from holoconf import Config

    # Enable HTTP resolver
    config = Config.from_file("config.yaml", allow_http=True)

    # With URL allowlist for additional security
    config = Config.from_file(
        "config.yaml",
        allow_http=True,
        http_allowlist=["https://config.example.com/*", "https://*.internal.com/*"]
    )
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    let config = Config::builder()
        .allow_http(true)
        .http_allowlist(vec!["https://config.example.com/*"])
        .load("config.yaml")?;
    ```

#### HTTP Resolver Options

| Option | Description | Example |
|--------|-------------|---------|
| `parse=auto` | Auto-detect from Content-Type or URL extension (default) | `${http:https://example.com/config}` |
| `parse=yaml` | Parse response as YAML | `${http:https://example.com/config,parse=yaml}` |
| `parse=json` | Parse response as JSON | `${http:https://example.com/config,parse=json}` |
| `parse=text` | Return response as text | `${http:https://example.com/data,parse=text}` |
| `parse=binary` | Return response as raw bytes | `${http:https://example.com/cert,parse=binary}` |
| `timeout=30` | Request timeout in seconds (default: 30) | `${http:https://example.com/config,timeout=60}` |
| `header=Name:Value` | Add custom HTTP header | `${http:https://api.com/config,header=Authorization:Bearer token}` |

#### URL Allowlist Patterns

The URL allowlist supports glob-style patterns:

| Pattern | Matches |
|---------|---------|
| `https://example.com/*` | Any path on example.com |
| `https://*.example.com/*` | Any subdomain of example.com |
| `https://api.example.com/config/*` | Specific path prefix |

#### Security Best Practices

1. **Keep HTTP disabled by default** - Only enable when needed
2. **Use URL allowlist** - Restrict which URLs can be fetched
3. **Use HTTPS** - Always use HTTPS for sensitive configuration
4. **Set appropriate timeouts** - Prevent hanging on slow responses
5. **Use authentication** - Add authorization headers for private endpoints

#### Proxy Configuration

Configure HTTP/SOCKS proxy for requests:

=== "Python"

    ```python
    from holoconf import Config

    # Explicit proxy
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_proxy="http://proxy.corp.com:8080"
    )

    # SOCKS proxy
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_proxy="socks5://proxy.corp.com:1080"
    )

    # Auto-detect from environment (HTTP_PROXY, HTTPS_PROXY)
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_proxy_from_env=True
    )
    ```

Per-request proxy override in YAML:

```yaml
value: ${http:https://api.example.com/config,proxy=http://proxy:8080}
```

#### Custom CA Certificates

For internal/corporate CAs or self-signed certificates:

=== "Python"

    ```python
    from holoconf import Config

    # Replace default roots with custom CA bundle
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_ca_bundle="/etc/ssl/certs/internal-ca.pem"
    )

    # Add extra CA to default roots
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_extra_ca_bundle="/etc/ssl/certs/extra-ca.pem"
    )
    ```

Per-request CA override in YAML:

```yaml
# Replace CA bundle
value: ${http:https://internal.corp/config,ca_bundle=/path/to/ca.pem}

# Add extra CA
value: ${http:https://api.example.com/config,extra_ca_bundle=/path/to/extra.pem}
```

#### Mutual TLS (mTLS) / Client Certificates

For services requiring client certificate authentication:

=== "Python"

    ```python
    from holoconf import Config

    # PEM certificate and key
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_client_cert="/path/to/client.pem",
        http_client_key="/path/to/client-key.pem"
    )

    # Encrypted private key
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_client_cert="/path/to/client.pem",
        http_client_key="/path/to/client-key-encrypted.pem",
        http_client_key_password="secret"
    )

    # P12/PFX bundle (includes both cert and key)
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_client_cert="/path/to/identity.p12",
        http_client_key_password="secret"
    )
    ```

Per-request mTLS in YAML:

```yaml
# PEM files
value: ${http:https://secure.corp/config,client_cert=/path/cert.pem,client_key=/path/key.pem}

# With encrypted key
value: ${http:https://secure.corp/config,client_cert=/path/cert.pem,client_key=/path/key.pem,key_password=secret}

# P12/PFX bundle
value: ${http:https://secure.corp/config,client_cert=/path/identity.p12,key_password=secret}
```

#### Disabling TLS Verification

!!! danger "http_insecure is dangerous"
    Setting `http_insecure=true` disables ALL TLS certificate verification.
    This exposes your application to man-in-the-middle attacks.

    **Never use in production.** Only for local development with self-signed certs.

=== "Python"

    ```python
    from holoconf import Config

    # DANGEROUS: For development only
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_insecure=True  # DO NOT USE IN PRODUCTION
    )
    ```

Per-request insecure mode in YAML:

```yaml
# DANGEROUS: Skip TLS verification
value: ${http:https://dev.local/config,insecure=true}
```

## Lazy Resolution

Resolvers are invoked **lazily** - values are only resolved when accessed, not when the configuration is loaded. This means:

- Environment variables are read at access time
- Files are read at access time
- HTTP requests are made at access time
- Default values are only resolved if the primary resolver fails

See [ADR-005 Resolver Timing](../adr/ADR-005-resolver-timing.md) for the design rationale.

## Sensitive Values

Mark values as sensitive to prevent them from appearing in logs or dumps:

```yaml
api:
  key: ${env:API_KEY,sensitive=true}
  secret: ${env:API_SECRET,default=dev-secret,sensitive=true}
```

When dumping configuration with `redact=True`:

```python
config = Config.from_file("config.yaml")
print(config.to_yaml(redact=True))
# api:
#   key: '[REDACTED]'
#   secret: '[REDACTED]'
```

## AWS Resolvers (`holoconf-aws`)

The `holoconf-aws` package provides resolvers for AWS services. Install separately:

```bash
pip install holoconf-aws
```

Resolvers are automatically registered when holoconf is imported.

### SSM Parameter Store (`ssm`)

Fetch values from AWS Systems Manager Parameter Store.

```yaml
database:
  host: ${ssm:/app/prod/db-host}
  password: ${ssm:/app/prod/db-password}
  # With region override
  config: ${ssm:/app/config,region=us-west-2}
  # With profile override
  secret: ${ssm:/app/secret,profile=production}
```

| Parameter Type | Behavior |
|---------------|----------|
| String | Returned as-is |
| SecureString | Automatically marked as sensitive |
| StringList | Returned as a Python list |

### CloudFormation Outputs (`cfn`)

Fetch outputs from CloudFormation stacks.

```yaml
infrastructure:
  endpoint: ${cfn:my-database-stack/DatabaseEndpoint}
  bucket: ${cfn:my-storage-stack/BucketName}
  # With region/profile
  vpc_id: ${cfn:shared-infra/VpcId,region=us-west-2}
```

### S3 Objects (`s3`)

Fetch and parse objects from S3.

```yaml
# Auto-parsed based on file extension
shared_config: ${s3:my-bucket/configs/shared.yaml}
feature_flags: ${s3:my-bucket/settings/flags.json}

# Explicit parse mode
raw_text: ${s3:my-bucket/docs/README.md,parse=text}
binary_data: ${s3:my-bucket/certs/cert.pem,parse=binary}
```

#### S3 Parse Modes

| Mode | Description |
|------|-------------|
| `auto` (default) | Detect by file extension or Content-Type |
| `yaml` | Parse as YAML |
| `json` | Parse as JSON |
| `text` | Return raw text |
| `binary` | Return raw bytes (`Value::Bytes`) |

### AWS Authentication

All AWS resolvers use the standard AWS credential chain:

1. Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
2. Shared credential file (`~/.aws/credentials`)
3. AWS config file (`~/.aws/config`)
4. IAM role (EC2/ECS/Lambda)

## Custom Resolvers

You can register custom resolvers in Python to integrate with any data source.

### Function Resolvers

The simplest form is a function that takes a key and optional keyword arguments:

```python
import holoconf

def my_lookup(key, region=None, **kwargs):
    """Custom resolver that looks up values from an internal service."""
    result = internal_api.get(key, region=region)
    return result

holoconf.register_resolver("lookup", my_lookup)
```

Now use it in your config:

```yaml
database:
  host: ${lookup:db-host,region=us-east-1}
```

### Async Resolvers

Async functions work automatically - HoloConf will await them:

```python
import holoconf

async def fetch_secret(key, **kwargs):
    """Async resolver that fetches secrets from a remote service."""
    async with aiohttp.ClientSession() as session:
        async with session.get(f"https://secrets.example.com/{key}") as resp:
            return await resp.text()

holoconf.register_resolver("secret", fetch_secret)
```

### Returning Sensitive Values

For values that should be redacted in output, return a `ResolvedValue`:

```python
from holoconf import register_resolver, ResolvedValue

def vault_resolver(path, **kwargs):
    """Resolver for HashiCorp Vault secrets."""
    secret = vault_client.read(path)
    # Mark all Vault values as sensitive
    return ResolvedValue(secret["data"]["value"], sensitive=True)

register_resolver("vault", vault_resolver)
```

```yaml
api:
  key: ${vault:secret/data/api-key}  # Will show as [REDACTED]
```

### Return Types

Resolvers can return various types:

| Return Type | Result |
|------------|--------|
| `str`, `int`, `float`, `bool` | Scalar value |
| `list` | List/array value |
| `dict` | Dictionary/mapping value |
| `ResolvedValue(value, sensitive=True)` | Value marked for redaction |

### Error Handling

Raise `KeyError` to indicate a resource wasn't found (enables `default=` fallback):

```python
def my_resolver(key, **kwargs):
    result = cache.get(key)
    if result is None:
        raise KeyError(f"Key not found: {key}")
    return result

register_resolver("cache", my_resolver)
```

```yaml
# Uses default if key not found
value: ${cache:my-key,default=fallback}
```

### Callable Classes

For resolvers that need initialization or state, use a callable class:

```python
from holoconf import register_resolver, ResolvedValue

class VaultResolver:
    def __init__(self, vault_addr):
        self.client = hvac.Client(url=vault_addr)

    def __call__(self, path, **kwargs):
        secret = self.client.secrets.kv.read_secret_version(path=path)
        return ResolvedValue(secret["data"]["data"], sensitive=True)

register_resolver("vault", VaultResolver("https://vault.example.com"))
```

## See Also

- [ADR-002 Resolver Architecture](../adr/ADR-002-resolver-architecture.md) - Design rationale
- [FEAT-002 Core Resolvers](../specs/features/FEAT-002-core-resolvers.md) - Full specification
- [FEAT-007 AWS Resolvers](../specs/features/FEAT-007-aws-resolvers.md) - AWS resolver specification
- [Interpolation](interpolation.md) - Interpolation syntax details

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

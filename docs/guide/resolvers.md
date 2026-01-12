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

## See Also

- [FEAT-002 Core Resolvers](../specs/features/FEAT-002-core-resolvers.md) - Full specification
- [Interpolation](interpolation.md) - Interpolation syntax details

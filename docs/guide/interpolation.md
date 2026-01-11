# Interpolation

## Overview

HoloConf supports interpolation syntax to dynamically resolve values at access time. The general syntax is:

```
${resolver:argument}
${resolver:argument,default=value}
${resolver:argument,sensitive=true}
```

## Quick Reference

| Syntax | Description | Example |
|--------|-------------|---------|
| `${env:VAR}` | Environment variable | `${env:DATABASE_URL}` |
| `${env:VAR,default=value}` | Environment variable with default | `${env:PORT,default=8080}` |
| `${env:VAR,sensitive=true}` | Mark as sensitive (redacted in output) | `${env:API_KEY,sensitive=true}` |
| `${path.to.value}` | Self-reference (absolute) | `${database.host}` |
| `${.sibling}` | Self-reference (relative) | `${.port}` |
| `${..parent.value}` | Self-reference (parent) | `${..shared.timeout}` |
| `${file:path}` | Include file content | `${file:./secrets.yaml}` |
| `${http:url}` | Fetch from HTTP endpoint | `${http:https://config.example.com/settings}` |

## Keyword Arguments

All resolvers support two framework-level keyword arguments:

- **`default=value`** - Fallback value if the resolver fails (e.g., env var not set, file not found)
- **`sensitive=true`** - Mark value as sensitive for redaction in output

These can be combined:

```yaml
api_key: ${env:API_KEY,default=dev-key,sensitive=true}
```

!!! note "Lazy Default Resolution"
    Default values are resolved lazily - if the primary resolver succeeds, the default is never evaluated. This allows defaults to contain other resolvers without causing errors.

## Examples

### Environment Variables

=== "Python"

    ```python
    # config.yaml:
    # database:
    #   url: ${env:DATABASE_URL,default=postgres://localhost/mydb}

    from holoconf import Config
    import os

    os.environ["DATABASE_URL"] = "postgres://prod-server/mydb"
    config = Config.from_file("config.yaml")

    url = config.get("database.url")
    # Returns: "postgres://prod-server/mydb"
    ```

=== "Rust"

    ```rust
    use holoconf::Config;
    use std::env;

    fn main() -> Result<(), holoconf::Error> {
        env::set_var("DATABASE_URL", "postgres://prod-server/mydb");
        let config = Config::from_file("config.yaml")?;

        let url: String = config.get("database.url")?;
        // Returns: "postgres://prod-server/mydb"

        Ok(())
    }
    ```

### Self-References

```yaml
defaults:
  timeout: 30
  retries: 3

database:
  timeout: ${defaults.timeout}
  connection_retries: ${defaults.retries}

cache:
  timeout: ${defaults.timeout}
```

### Sensitive Values

Mark values as sensitive to automatically redact them in dumps:

```yaml
database:
  host: ${env:DB_HOST,default=localhost}
  password: ${env:DB_PASSWORD,sensitive=true}
```

When dumping with redaction:

```python
config = Config.from_file("config.yaml")
print(config.to_yaml(redact=True))
# database:
#   host: localhost
#   password: '[REDACTED]'
```

### Nested Defaults

Default values can contain other resolvers:

```yaml
# Falls back to FALLBACK_VAR if PRIMARY_VAR is not set
value: ${env:PRIMARY_VAR,default=${env:FALLBACK_VAR,default=final-fallback}}
```

## Escaping

To include a literal `${` in your configuration, escape it with a backslash:

```yaml
template: "Hello \${name}"  # Literal ${name}, not interpolated
```

## See Also

- [ADR-011 Interpolation Syntax](../adr/ADR-011-interpolation-syntax.md) - Technical details and design rationale
- [Resolvers](resolvers.md) - Detailed resolver documentation

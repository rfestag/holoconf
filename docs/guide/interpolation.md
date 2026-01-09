# Interpolation

!!! note "Coming Soon"
    This page is under construction. See [ADR-011 Interpolation Syntax](../adr/ADR-011-interpolation-syntax.md) for technical details.

## Overview

HoloConf supports interpolation syntax to dynamically resolve values at access time. The general syntax is:

```
${resolver:argument}
${resolver:argument,default}
```

## Quick Reference

| Syntax | Description | Example |
|--------|-------------|---------|
| `${env:VAR}` | Environment variable | `${env:DATABASE_URL}` |
| `${env:VAR,default}` | Environment variable with default | `${env:PORT,8080}` |
| `${path.to.value}` | Self-reference (absolute) | `${database.host}` |
| `${.sibling}` | Self-reference (relative) | `${.port}` |
| `${..parent.value}` | Self-reference (parent) | `${..shared.timeout}` |
| `${file:path}` | Include file content | `${file:./secrets.yaml}` |
| `${http:url}` | Fetch from HTTP endpoint | `${http:https://config.example.com/settings}` |

## Examples

### Environment Variables

=== "Python"

    ```python
    # config.yaml:
    # database:
    #   url: ${env:DATABASE_URL,postgres://localhost/mydb}

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

## Escaping

To include a literal `${` in your configuration, escape it with a backslash:

```yaml
template: "Hello \${name}"  # Literal ${name}, not interpolated
```

# Resolvers

!!! note "Coming Soon"
    This page is under construction. See [FEAT-002 Core Resolvers](../specs/features/FEAT-002-core-resolvers.md) for the full specification.

## Overview

Resolvers are the mechanism holoconf uses to dynamically compute configuration values. Each resolver handles a specific type of value source.

## Built-in Resolvers

### Environment Variables (`env`)

Reads values from environment variables.

```yaml
database:
  host: ${env:DB_HOST}
  port: ${env:DB_PORT,5432}
  password: ${env:DB_PASSWORD}
```

=== "Python"

    ```python
    import os
    from holoconf import Config

    os.environ["DB_HOST"] = "production-db.example.com"
    config = Config.from_file("config.yaml")

    host = config.get("database.host")  # "production-db.example.com"
    port = config.get("database.port")  # 5432 (default, since DB_PORT not set)
    ```

=== "Rust"

    ```rust
    use holoconf::Config;
    use std::env;

    fn main() -> Result<(), holoconf::Error> {
        env::set_var("DB_HOST", "production-db.example.com");
        let config = Config::from_file("config.yaml")?;

        let host: String = config.get("database.host")?;
        let port: i64 = config.get("database.port")?;

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

### File Include (`file`)

Include content from other files.

```yaml
# main.yaml
app:
  name: my-app
  secrets: ${file:./secrets.yaml}
```

### HTTP (`http`)

!!! warning "Security"
    HTTP resolver is disabled by default for security. Enable it explicitly in your configuration.

Fetch configuration from HTTP endpoints.

```yaml
feature_flags: ${http:https://config.example.com/flags.json}
```

## Lazy Resolution

Resolvers are invoked **lazily** - values are only resolved when accessed, not when the configuration is loaded. This means:

- Environment variables are read at access time
- Files are read at access time
- HTTP requests are made at access time

See [ADR-005 Resolver Timing](../adr/ADR-005-resolver-timing.md) for the design rationale.

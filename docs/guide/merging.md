# Configuration Merging

!!! note "Coming Soon"
    This page is under construction. See [FEAT-003 Config Merging](../specs/features/FEAT-003-config-merging.md) for the full specification.

## Overview

HoloConf allows you to merge multiple configuration files, enabling patterns like:

- Base configuration with environment-specific overrides
- Shared defaults with local customizations
- Modular configuration split across multiple files

## Basic Merging

=== "Python"

    ```python
    from holoconf import Config

    # Load base configuration, then merge overrides
    config = Config.from_files([
        "config/base.yaml",
        "config/production.yaml"
    ])

    # Later files override earlier ones
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    fn main() -> Result<(), holoconf::Error> {
        let config = Config::from_files(&[
            "config/base.yaml",
            "config/production.yaml",
        ])?;

        Ok(())
    }
    ```

## Merge Behavior

When merging configurations:

| Type | Behavior |
|------|----------|
| Scalars (string, int, bool) | Later value replaces earlier |
| Objects/Maps | Deep merge (keys are merged recursively) |
| Arrays | Later array replaces earlier (no concatenation) |

### Example

```yaml
# base.yaml
database:
  host: localhost
  port: 5432
  pool_size: 10

logging:
  level: info
```

```yaml
# production.yaml
database:
  host: prod-db.example.com
  pool_size: 50

logging:
  level: warning
```

Result after merging:

```yaml
database:
  host: prod-db.example.com  # overridden
  port: 5432                  # from base
  pool_size: 50               # overridden

logging:
  level: warning              # overridden
```

## Common Patterns

### Environment-based Configuration

```
config/
├── base.yaml
├── development.yaml
├── staging.yaml
└── production.yaml
```

=== "Python"

    ```python
    import os
    from holoconf import Config

    env = os.environ.get("APP_ENV", "development")
    config = Config.from_files([
        "config/base.yaml",
        f"config/{env}.yaml"
    ])
    ```

=== "Rust"

    ```rust
    use holoconf::Config;
    use std::env;

    fn main() -> Result<(), holoconf::Error> {
        let env = env::var("APP_ENV").unwrap_or_else(|_| "development".into());
        let config = Config::from_files(&[
            "config/base.yaml".into(),
            format!("config/{}.yaml", env),
        ])?;

        Ok(())
    }
    ```

See [ADR-004 Config Merging](../adr/ADR-004-config-merging.md) for the design rationale.

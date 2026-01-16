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
    base = Config.load("config/base.yaml")
    production = Config.load("config/production.yaml")
    base.merge(production)

    # Now 'base' contains the merged result
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    fn main() -> Result<(), holoconf::Error> {
        let mut config = Config::load("config/base.yaml")?;
        let production = Config::load("config/production.yaml")?;
        config.merge(production);

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

    config = Config.load("config/base.yaml")
    env_config = Config.load(f"config/{env}.yaml")
    config.merge(env_config)
    ```

=== "Rust"

    ```rust
    use holoconf::Config;
    use std::env;

    fn main() -> Result<(), holoconf::Error> {
        let env = env::var("APP_ENV").unwrap_or_else(|_| "development".into());

        let mut config = Config::load("config/base.yaml")?;
        let env_config = Config::load(&format!("config/{}.yaml", env))?;
        config.merge(env_config);

        Ok(())
    }
    ```

## Optional Files

Sometimes you want to load configuration files that may or may not exist. For example, a `local.yaml` file that developers can create for local overrides but isn't committed to version control.

Use `Config.optional()` to load files that might not exist:

=== "Python"

    ```python
    from holoconf import Config

    # Config.optional() returns empty config if file doesn't exist
    config = Config.load("config/base.yaml")  # Required - errors if missing
    local = Config.optional("config/local.yaml")  # Optional - empty if missing
    config.merge(local)

    # Symmetry with Config.required() (alias for load)
    config = Config.required("config/base.yaml")  # Same as Config.load()
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    fn main() -> Result<(), holoconf::Error> {
        // Config::optional() returns empty config if file doesn't exist
        let mut config = Config::load("config/base.yaml")?;  // Required
        let local = Config::optional("config/local.yaml")?;  // Optional
        config.merge(local);

        // Config::required() is an alias for load()
        let config = Config::required("config/base.yaml")?;

        Ok(())
    }
    ```

### Behavior

- **`Config.load(path)`** / **`Config.required(path)`**: Must exist. Returns an error if the file is not found.
- **`Config.optional(path)`**: Returns an empty Config if the file doesn't exist. Loads normally if present.

### Common Pattern: Local Overrides

A typical setup with optional local overrides:

```
config/
├── base.yaml           # Committed, shared defaults
├── production.yaml     # Committed, production settings
└── local.yaml          # .gitignored, developer-specific overrides
```

=== "Python"

    ```python
    from holoconf import Config
    import os

    env = os.environ.get("APP_ENV", "development")

    # Load and merge in order: base → environment → local overrides
    config = Config.load("config/base.yaml")
    env_config = Config.load(f"config/{env}.yaml")
    config.merge(env_config)

    # Local overrides are optional
    local = Config.optional("config/local.yaml")
    config.merge(local)
    ```

=== "Rust"

    ```rust
    use holoconf::Config;
    use std::env;

    fn main() -> Result<(), holoconf::Error> {
        let env = env::var("APP_ENV").unwrap_or_else(|_| "development".into());

        // Load and merge in order
        let mut config = Config::load("config/base.yaml")?;
        let env_config = Config::load(&format!("config/{}.yaml", env))?;
        config.merge(env_config);

        // Local overrides are optional
        let local = Config::optional("config/local.yaml")?;
        config.merge(local);

        Ok(())
    }
    ```

## Glob Patterns

When loading configurations from multiple files, you can use glob patterns to automatically match and merge files:

=== "Python"

    ```python
    from holoconf import Config

    # Load all YAML files in config/ directory
    config = Config.load("config/*.yaml")

    # Load recursively from nested directories
    config = Config.load("config/**/*.yaml")
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    fn main() -> Result<(), holoconf::Error> {
        // Load all YAML files in config/ directory
        let config = Config::load("config/*.yaml")?;

        // Load recursively from nested directories
        let config = Config::load("config/**/*.yaml")?;

        Ok(())
    }
    ```

### Supported Patterns

| Pattern | Matches |
|---------|---------|
| `*` | Any sequence of characters (except `/`) |
| `**` | Any sequence of directories |
| `?` | Any single character |
| `[abc]` | Any character in the set |
| `[a-z]` | Any character in the range |

### Merge Order

Files matching a glob pattern are **sorted alphabetically** before merging. This means:

- `00-base.yaml` is loaded before `10-override.yaml`
- `a.yaml` is loaded before `b.yaml`
- `config/base.yaml` is loaded before `config/sub/override.yaml`

Use numeric prefixes to control the merge order:

```
config/
├── 00-base.yaml       # Loaded first (base settings)
├── 10-database.yaml   # Loaded second
├── 20-logging.yaml    # Loaded third
└── 99-local.yaml      # Loaded last (highest priority)
```

### Required vs Optional Globs

- **`Config.load("pattern")`**: At least one file must match. Returns an error if no files match.
- **`Config.optional("pattern")`**: Returns an empty config if no files match.

=== "Python"

    ```python
    from holoconf import Config

    # Error if no files match
    config = Config.load("config/*.yaml")

    # Empty config if no files match
    overrides = Config.optional("overrides/*.yaml")
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    fn main() -> Result<(), holoconf::Error> {
        // Error if no files match
        let config = Config::load("config/*.yaml")?;

        // Empty config if no files match
        let overrides = Config::optional("overrides/*.yaml")?;

        Ok(())
    }
    ```

See [ADR-004 Config Merging](../adr/ADR-004-config-merging.md) for the design rationale.

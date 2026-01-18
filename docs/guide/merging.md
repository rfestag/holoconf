# Configuration Merging

Real applications rarely use a single configuration file. You need different settings for development, staging, and production. Your team might have shared defaults, but individual developers need their own local overrides. This is where configuration merging shines.

Let's learn how to split your configuration intelligently and merge it back together.

## Why Split Configuration?

Imagine you're working on a web application. You have:
- Base settings that everyone shares (app name, API endpoints structure)
- Production settings (production database, external services)
- Your local development overrides (point database at localhost, enable debug mode)

You could put everything in one big file with lots of conditional logic. But that gets messy fast. Instead, let's split it across multiple files and merge them together.

## Your First Merge: Base and Environment

Let's start with two files. First, create `config/base.yaml` with shared defaults:

```yaml
# config/base.yaml
app:
  name: my-application
  debug: false

database:
  host: localhost
  port: 5432
  pool_size: 10

logging:
  level: info
  format: json
```

Now create `config/production.yaml` with production-specific overrides:

```yaml
# config/production.yaml
database:
  host: prod-db.example.com
  pool_size: 50

logging:
  level: warning
```

Notice production only includes what's different. Let's merge them:

=== "Python"

    ```python
    from holoconf import Config

    # Load base configuration
    config = Config.load("config/base.yaml")

    # Load production overrides
    production = Config.load("config/production.yaml")

    # Merge production into base
    config.merge(production)

    # Now config contains the merged result
    db_host = config.get("database.host")
    print(f"Database: {db_host}")
    # Database: prod-db.example.com

    db_port = config.get("database.port")
    print(f"Port: {db_port}")
    # Port: 5432 (from base.yaml)

    pool_size = config.get("database.pool_size")
    print(f"Pool size: {pool_size}")
    # Pool size: 50 (overridden by production.yaml)
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    // Load base configuration
    let mut config = Config::load("config/base.yaml")?;

    // Load production overrides
    let production = Config::load("config/production.yaml")?;

    // Merge production into base
    config.merge(production);

    let db_host: String = config.get("database.host")?;
    println!("Database: {}", db_host);
    // Database: prod-db.example.com
    ```

=== "CLI"

    ```bash
    # The CLI doesn't support merge directly, but you can use glob patterns
    # (we'll cover this later)
    ```

Let's understand what happened:
- `database.host` was overridden to `prod-db.example.com`
- `database.port` kept its value from base (`5432`) because production didn't override it
- `database.pool_size` was overridden to `50`
- `logging.level` was overridden to `warning`
- Everything else (`app.name`, `app.debug`, `logging.format`) stayed from base

## How Merging Works

When you merge configurations, HoloConf uses these rules:

| Type | Behavior |
|------|----------|
| Scalars (string, int, bool) | Later value replaces earlier |
| Objects/Maps | Deep merge (keys merged recursively) |
| Arrays | Later array replaces earlier (no concatenation) |

This means merging is "deep" for nested objects but "shallow" for arrays. Let's see an example:

```yaml
# base.yaml
features:
  auth:
    enabled: true
    providers: [github, google]
  search:
    enabled: true
```

```yaml
# override.yaml
features:
  auth:
    providers: [local]  # This replaces the entire array
  analytics:
    enabled: true
```

After merging:

```yaml
features:
  auth:
    enabled: true           # Kept from base
    providers: [local]      # Replaced by override
  search:
    enabled: true           # Kept from base
  analytics:
    enabled: true           # Added by override
```

## Environment-Based Configuration

Now let's build a pattern you'll use all the time: environment-based configuration. Your directory structure:

```
config/
├── base.yaml
├── development.yaml
├── staging.yaml
└── production.yaml
```

Load the right config based on environment:

=== "Python"

    ```python
    import os
    from holoconf import Config

    # Get environment from environment variable
    env = os.environ.get("APP_ENV", "development")

    # Load base config
    config = Config.load("config/base.yaml")

    # Merge environment-specific config
    env_config = Config.load(f"config/{env}.yaml")
    config.merge(env_config)

    # Now use the merged config
    db_host = config.get("database.host")
    print(f"Running in {env} with database {db_host}")
    ```

=== "Rust"

    ```rust
    use holoconf::Config;
    use std::env;

    // Get environment from environment variable
    let env_name = env::var("APP_ENV")
        .unwrap_or_else(|_| "development".into());

    // Load base config
    let mut config = Config::load("config/base.yaml")?;

    // Merge environment-specific config
    let env_config = Config::load(&format!("config/{}.yaml", env_name))?;
    config.merge(env_config);

    let db_host: String = config.get("database.host")?;
    println!("Running in {} with database {}", env_name, db_host);
    ```

This pattern gives you:
- Shared defaults in `base.yaml`
- Environment-specific overrides in `development.yaml`, `production.yaml`, etc.
- One simple switch (`APP_ENV`) to control which config is loaded

## Optional Files: Local Overrides

What about files that might not exist? For example, you want developers to be able to create a `local.yaml` file for their personal overrides, but you don't want to commit it to git.

If you try to load a missing file with `Config.load()`, you get an error:

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config/base.yaml")

    # This will error if local.yaml doesn't exist
    local = Config.load("config/local.yaml")  # Error!
    config.merge(local)
    ```

Instead, use `Config.optional()`:

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config/base.yaml")

    # This returns an empty config if the file doesn't exist
    local = Config.optional("config/local.yaml")
    config.merge(local)

    # Now the merge works whether or not local.yaml exists!
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    let mut config = Config::load("config/base.yaml")?;

    // Returns empty config if file doesn't exist
    let local = Config::optional("config/local.yaml")?;
    config.merge(local);
    ```

Now developers can create `config/local.yaml` with their personal settings:

```yaml
# config/local.yaml (not committed to git)
app:
  debug: true

database:
  host: localhost

logging:
  level: debug
```

And add it to `.gitignore`:

```
# .gitignore
config/local.yaml
```

!!! tip "Common Pattern: Three-Layer Configuration"
    A robust pattern uses three layers:

    1. **Base** - Shared defaults (committed)
    2. **Environment** - Environment-specific (committed)
    3. **Local** - Developer overrides (gitignored, optional)

    ```python
    config = Config.load("config/base.yaml")
    env_config = Config.load(f"config/{env}.yaml")
    config.merge(env_config)
    local = Config.optional("config/local.yaml")
    config.merge(local)
    ```

## Glob Patterns: Automatic Merging

Sometimes you have many config files and you want to merge them all automatically. Use glob patterns:

=== "Python"

    ```python
    from holoconf import Config

    # Load and merge all YAML files in config/ directory
    config = Config.load("config/*.yaml")

    # Load recursively from nested directories
    config = Config.load("config/**/*.yaml")
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    // Load all YAML files in config/ directory
    let config = Config::load("config/*.yaml")?;

    // Load recursively from nested directories
    let config = Config::load("config/**/*.yaml")?;
    ```

=== "CLI"

    ```bash
    # Glob patterns work in the CLI too!
    holoconf dump "config/*.yaml" --resolve
    ```

Supported patterns:

| Pattern | Matches |
|---------|---------|
| `*` | Any sequence of characters (except `/`) |
| `**` | Any sequence of directories |
| `?` | Any single character |
| `[abc]` | Any character in the set |
| `[a-z]` | Any character in the range |

### Merge Order

Files matching a glob are **sorted alphabetically** before merging:

```
config/
├── 00-base.yaml       # Loaded first
├── 10-database.yaml   # Loaded second
├── 20-logging.yaml    # Loaded third
└── 99-local.yaml      # Loaded last (highest priority)
```

This lets you control merge order with numeric prefixes. The file loaded last wins for any conflicting values.

Let's see this in action:

```yaml
# 00-base.yaml
app:
  name: myapp
  timeout: 30
```

```yaml
# 10-database.yaml
database:
  host: localhost
  port: 5432
```

```yaml
# 99-local.yaml
app:
  timeout: 60  # Overrides 00-base.yaml
```

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config/*.yaml")

    # Files merged in order: 00-base, 10-database, 99-local
    timeout = config.get("app.timeout")
    print(f"Timeout: {timeout}")
    # Timeout: 60 (from 99-local.yaml)
    ```

!!! tip "Numeric Prefixes"
    Use numeric prefixes to make merge order explicit:

    - `00-` Base configuration
    - `10-`, `20-`, `30-` Feature-specific configs
    - `99-` Local overrides (highest priority)

### Optional Globs

What if no files match your pattern? By default, `Config.load()` errors:

=== "Python"

    ```python
    from holoconf import Config

    # Error if no files match
    config = Config.load("config/*.yaml")
    # Error: No files matched pattern config/*.yaml
    ```

Use `Config.optional()` to return an empty config instead:

=== "Python"

    ```python
    from holoconf import Config

    # Returns empty config if no files match
    overrides = Config.optional("overrides/*.yaml")

    # Safe to merge even if no overrides exist
    config = Config.load("config/base.yaml")
    config.merge(overrides)
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    // Returns empty config if no files match
    let overrides = Config::optional("overrides/*.yaml")?;

    let mut config = Config::load("config/base.yaml")?;
    config.merge(overrides);
    ```

## Putting It All Together

Here's a complete example using everything we've learned:

```
config/
├── 00-base.yaml           # Base defaults
├── 10-database.yaml       # Database config
├── 20-logging.yaml        # Logging config
├── environments/
│   ├── development.yaml
│   ├── staging.yaml
│   └── production.yaml
└── local.yaml             # .gitignored, optional
```

=== "Python"

    ```python
    import os
    from holoconf import Config

    # Get environment
    env = os.environ.get("APP_ENV", "development")

    # Step 1: Load and merge base configs
    config = Config.load("config/0*.yaml")  # All files starting with 0, 1, 2

    # Step 2: Merge environment-specific config
    env_config = Config.load(f"config/environments/{env}.yaml")
    config.merge(env_config)

    # Step 3: Merge optional local overrides
    local = Config.optional("config/local.yaml")
    config.merge(local)

    # Now use the fully merged config
    print(f"Running in {env} environment")
    print(f"Database: {config.get('database.host')}")
    print(f"Log level: {config.get('logging.level')}")
    ```

=== "Rust"

    ```rust
    use holoconf::Config;
    use std::env;

    let env_name = env::var("APP_ENV")
        .unwrap_or_else(|_| "development".into());

    // Load and merge base configs
    let mut config = Config::load("config/0*.yaml")?;

    // Merge environment-specific
    let env_config = Config::load(&format!("config/environments/{}.yaml", env_name))?;
    config.merge(env_config);

    // Merge optional local
    let local = Config::optional("config/local.yaml")?;
    config.merge(local);

    println!("Running in {} environment", env_name);
    ```

This gives you maximum flexibility:
- Shared defaults in numbered files
- Environment-specific overrides
- Personal local overrides
- All merged automatically

!!! tip "Try It Yourself"
    Set up a multi-file configuration:

    1. Create `config/00-base.yaml` with basic settings
    2. Create `config/10-database.yaml` with database config
    3. Create `config/environments/development.yaml` with dev settings
    4. Create `config/local.yaml` with your personal overrides
    5. Load and merge them all!

## What You've Learned

You now understand:

- How to merge two configurations together
- Deep merge behavior for objects vs shallow for arrays
- Environment-based configuration patterns
- Optional files for local overrides
- Glob patterns for automatic merging
- Controlling merge order with numeric prefixes
- Building robust multi-layer configuration systems

## Next Steps

- **[Validation](validation.md)** - Validate your merged configuration with JSON Schema
- **[ADR-004 Config Merging](../adr/ADR-004-config-merging.md)** - Design rationale for merge behavior

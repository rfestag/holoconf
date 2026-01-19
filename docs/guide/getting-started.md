# Getting Started

Let's build your first hierarchical configuration from scratch. By the end of this guide, you'll understand why HoloConf exists and how to use it to make your configuration flexible and secure.

## Why Hierarchical Configuration?

Imagine you're building an application. You hardcode the database host as `localhost` during development. Everything works great. Then you deploy to production and... nothing works. You forgot to change the database host.

So you use environment variables instead. Now your configuration is scattered across environment variable declarations, deployment scripts, and documentation. New team members struggle to understand what variables are needed.

HoloConf solves this by letting you write configuration files that:

- Show all possible settings in one place (self-documenting)
- Pull values from environment variables and external sources (deployment flexibility)
- Reference other config values to avoid duplication (DRY)
- Merge multiple files for environment-specific overrides (layered configuration)
- Validate against schemas to catch errors early (reliability)

Let's see how this works in practice.

## Installation

First, let's install HoloConf:

=== "Python"

    ```bash
    pip install holoconf
    ```

    Requires Python 3.8 or later.

=== "Rust"

    Add to your `Cargo.toml`:

    ```toml
    [dependencies]
    holoconf = "0.3"
    ```

=== "CLI"

    The CLI is included with the Python package:

    ```bash
    pip install holoconf
    holoconf --help
    ```

    Or install the standalone Rust binary:

    ```bash
    cargo install holoconf-cli
    ```

## Your First Configuration: Static Values

Let's start with the simplest possible configuration - just static values. Create a file named `config.yaml`:

```yaml
app:
  name: my-application

database:
  host: localhost
  port: 5432
```

Now let's load and read this configuration:

=== "Python"

    ```python
    from holoconf import Config

    # Load from file
    config = Config.load("config.yaml")

    # Access values using dot notation (recommended)
    app_name = config.app.name
    db_host = config.database.host

    print(f"App: {app_name}")
    print(f"Database: {db_host}")
    # App: my-application
    # Database: localhost
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    fn main() -> Result<(), holoconf::Error> {
        // Load from file
        let config = Config::load("config.yaml")?;

        // Access values using dot notation
        let app_name: String = config.get("app.name")?;
        let db_host: String = config.get("database.host")?;

        println!("App: {}", app_name);
        println!("Database: {}", db_host);
        // App: my-application
        // Database: localhost

        Ok(())
    }
    ```

=== "CLI"

    ```bash
    # Get a specific value
    $ holoconf get config.yaml app.name
    my-application

    # Get database host
    $ holoconf get config.yaml database.host
    localhost
    ```

This works, but the configuration is completely static. Let's make it dynamic!

### Three Ways to Access Values

By the way, Python supports three different ways to access values. Here they are for reference:

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config.yaml")

    # Dot notation (recommended - most Pythonic)
    host = config.database.host

    # Dict-like access (alternative)
    host = config["database"]["host"]

    # Explicit get() method (alternative)
    host = config.get("database.host")

    # All three return the same value!
    ```

We'll use dot notation throughout the guide because it's the most readable and Pythonic.

## Interpolation: Dynamic Values

Real configurations need to reference other values and pull data from external sources. HoloConf uses **interpolation** with the `${...}` syntax to make this easy.

Let's update our configuration to:
- Pull the database host from an environment variable
- Reference that host value to build a connection URL

```yaml
app:
  name: my-application

database:
  host: ${env:DB_HOST}
  port: 5432
  url: postgres://${.host}:${.port}/mydb  # References .host and .port
```

Notice two things here:
- `${env:DB_HOST}` - Gets a value from an **external source** (environment variable)
- `${.host}` - References a **sibling value** in the same config (keeps things DRY)

Now let's use it:

=== "Python"

    ```python
    import os
    from holoconf import Config

    # Set the environment variable
    os.environ["DB_HOST"] = "prod-db.example.com"

    config = Config.load("config.yaml")

    print(config.database.host)
    # prod-db.example.com

    print(config.database.url)
    # postgres://prod-db.example.com:5432/mydb
    ```

=== "Rust"

    ```rust
    use holoconf::Config;
    use std::env;

    env::set_var("DB_HOST", "prod-db.example.com");
    let config = Config::load("config.yaml")?;

    let host: String = config.get("database.host")?;
    println!("{}", host);
    // prod-db.example.com

    let url: String = config.get("database.url")?;
    println!("{}", url);
    // postgres://prod-db.example.com:5432/mydb
    ```

=== "CLI"

    ```bash
    $ export DB_HOST="prod-db.example.com"
    $ holoconf get config.yaml database.url
    postgres://prod-db.example.com:5432/mydb
    ```

HoloConf supports many types of interpolation:
- **Self-references** - `${database.host}` (absolute) or `${.sibling}` (relative)
- **Environment variables** - `${env:VAR_NAME}`
- **File contents** - `${file:path/to/file}`
- **HTTP requests** - `${https://api.example.com/config}`
- **AWS resources** - `${ssm:/param/path}`, `${cfn:stack.Output}`
- **Custom resolvers** - Write your own in Python!

You can also provide defaults, mark values as sensitive, create fallback chains, and more.

**Learn more:** [Interpolation Guide](interpolation.md) | [Resolvers Guide](resolvers.md)

## Merging: Environment-Specific Configuration

Most applications need different settings for development, staging, and production. Instead of maintaining separate complete configuration files, you can create a base configuration and merge environment-specific overrides.

Create a base configuration (`base.yaml`):

```yaml
app:
  name: my-application
  debug: false

database:
  host: ${env:DB_HOST}
  port: 5432
```

And a development override (`dev.yaml`):

```yaml
app:
  debug: true

database:
  host: localhost
```

Now merge them:

=== "Python"

    ```python
    from holoconf import Config

    base = Config.load("base.yaml")
    dev = Config.load("dev.yaml")

    # Merge dev settings into base
    base.merge(dev)

    print(base.app.debug)      # true (from dev)
    print(base.database.host)  # localhost (from dev)
    print(base.database.port)  # 5432 (from base)
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    let mut base = Config::load("base.yaml")?;
    let dev = Config::load("dev.yaml")?;

    base.merge(dev)?;

    let debug: bool = base.get("app.debug")?;
    println!("{}", debug);  // true (from dev)
    ```

=== "CLI"

    ```bash
    # Merge by listing files (later files override earlier ones)
    $ holoconf get base.yaml dev.yaml app.debug
    true
    ```

This lets you:
- Keep common settings in one place
- Override only what changes per environment
- Use optional files that may not exist (like `local.yaml`)
- Layer multiple configurations together

**Learn more:** [Merging Guide](merging.md)

## Validation: Catch Errors Early

Typos in configuration can cause runtime failures. JSON Schema validation catches these errors before your application starts.

Create a schema (`schema.json`):

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["app", "database"],
  "properties": {
    "app": {
      "type": "object",
      "properties": {
        "name": { "type": "string" },
        "debug": { "type": "boolean", "default": false }
      }
    },
    "database": {
      "type": "object",
      "required": ["host", "port"],
      "properties": {
        "host": { "type": "string" },
        "port": { "type": "integer", "default": 5432 }
      }
    }
  }
}
```

Now load your config with the schema:

=== "Python"

    ```python
    from holoconf import Config

    # Load with schema - validates automatically
    config = Config.load("config.yaml", schema="schema.json")

    # Schema provides defaults for missing values
    debug = config.app.debug
    print(debug)  # false (from schema default)
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    let config = Config::load_with_schema("config.yaml", "schema.json")?;

    let debug: bool = config.get("app.debug")?;
    println!("{}", debug);  // false (from schema default)
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml app.debug --schema schema.json
    false
    ```

Schemas help you:
- Catch typos and missing required fields
- Enforce type constraints (string, integer, etc.)
- Provide default values
- Document your configuration structure

**Learn more:** [Validation Guide](validation.md)

## What You've Learned

You now know how to:

- **Load configuration** from YAML files
- **Access values** with dot notation
- **Use interpolation** to reference config values and pull from external sources
- **Merge configurations** for environment-specific overrides
- **Validate** with JSON Schema to catch errors early

This covers the core concepts of HoloConf. Each of these topics has much more depth - check out the detailed guides to learn more!

## Next Steps

Dive deeper into specific topics:

- **[Interpolation](interpolation.md)** - Self-references, relative paths, and the `${...}` syntax
- **[Resolvers](resolvers.md)** - Environment variables, file includes, HTTP fetching, AWS resources, and custom resolvers
- **[Merging](merging.md)** - Layered configurations, optional files, and precedence rules
- **[Validation](validation.md)** - Schema basics, default values, and validation modes

Or explore the API reference for your language:
- [Python API](../api/python/index.md)
- [Rust API](../api/rust/index.md)
- [CLI Reference](../api/cli/index.md)

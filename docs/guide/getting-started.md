# Getting Started

Configuration management gets messy quickly. You copy-paste the same database host into multiple places, then forget to update one when it changes. You maintain separate config files for each environment with 90% duplicated content. Your team struggles to understand what settings are available and which ones are required.

HoloConf solves these problems:

- **Stay DRY** - Reference values instead of repeating them. Change the database host once, and every connection string updates automatically.
- **Merge configurations** - Define common settings once, then layer environment-specific overrides on top. Your dev, staging, and production configs share a single base.
- **Pull from external sources** - Read values from environment variables, files, web APIs, or cloud services like AWS Parameter Store. Keep secrets out of your config files.
- **Validate with schemas** - Define what your configuration should look like. Catch typos and missing values before deployment, not in production.

Let's see how it works.

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

    # Access values - Python supports three ways:
    app_name = config.app.name                  # Dot notation (recommended)
    db_host = config["database"]["host"]        # Dict-like access
    db_port = config.get("database.port")       # get() method

    # All three work! We'll use dot notation throughout this guide.
    print(f"App: {app_name}")
    print(f"Database: {db_host}:{db_port}")
    # App: my-application
    # Database: localhost:5432
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    fn main() -> Result<(), holoconf::Error> {
        // Load from file
        let config = Config::load("config.yaml")?;

        // Access values with get() - supports type conversion
        let app_name: String = config.get("app.name")?;
        let db_host: String = config.get("database.host")?;
        let db_port: i64 = config.get("database.port")?;

        println!("App: {}", app_name);
        println!("Database: {}:{}", db_host, db_port);
        // App: my-application
        // Database: localhost:5432

        Ok(())
    }
    ```

=== "CLI"

    ```bash
    # Get specific values with dot notation
    $ holoconf get config.yaml app.name
    my-application

    $ holoconf get config.yaml database.host
    localhost

    # Or dump the entire config
    $ holoconf dump config.yaml
    app:
      name: my-application
    database:
      host: localhost
      port: 5432
    ```

This works, but the configuration is completely static. Let's make it dynamic!

## Interpolation: Dynamic Values

Real configurations need to reference other values (using absolute or relative paths) and pull data from external sources. HoloConf uses **interpolation** with the `${...}` syntax to make this easy.

Let's update our configuration to show different types of interpolation:

```yaml
app:
  name: my-application

database:
  host: ${env:DB_HOST}
  port: 5432
  name: ${app.name}                          # Absolute reference to app.name
  url: postgres://${.host}:${.port}/${.name}  # Relative references (.host, .port, .name)
```

Notice three things here:

- `${env:DB_HOST}` - Gets a value from an **external source** (via the `env` resolver)
- `${app.name}` - **Absolute reference** to a value elsewhere in the config
- `${.host}` - **Relative reference** to a sibling value (keeps things DRY)

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

    print(config.database.name)
    # my-application

    print(config.database.url)
    # postgres://prod-db.example.com:5432/my-application
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

    let db_name: String = config.get("database.name")?;
    println!("{}", db_name);
    // my-application

    let url: String = config.get("database.url")?;
    println!("{}", url);
    // postgres://prod-db.example.com:5432/my-application
    ```

=== "CLI"

    ```bash
    $ export DB_HOST="prod-db.example.com"
    $ holoconf get config.yaml database.url
    postgres://prod-db.example.com:5432/my-application
    ```

**Learn more:** [Interpolation Guide](interpolation.md) | [Resolvers Guide](resolvers.md)

## Merging: Environment-Specific Configuration

Many applications have configuration overrides for different environments. With HoloConf, you can merge configs.

`base.yaml`:

```yaml
app:
  name: my-application
  debug: false

database:
  host: ${env:DB_HOST}
  port: 5432
```

`dev.yaml`:

```yaml
app:
  debug: true

database:
  host: localhost
```

Merge them with the CLI:

```bash
$ holoconf dump base.yaml dev.yaml
app:
  name: my-application
  debug: true          # From dev.yaml
database:
  host: localhost      # From dev.yaml
  port: 5432           # From base.yaml
```

Later files override earlier ones, letting you layer environment-specific settings on top of a base configuration.

**Learn more:** [Merging Guide](merging.md)

## Validation: Catch Errors Early

Typos in configuration can cause runtime failures. JSON Schema validation catches these errors before your application starts.

Create a schema in JSON or YAML:

=== "JSON"

    `schema.json`:
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

=== "YAML"

    `schema.yaml`:
    ```yaml
    $schema: http://json-schema.org/draft-07/schema#
    type: object
    required: [app, database]
    properties:
      app:
        type: object
        properties:
          name:
            type: string
          debug:
            type: boolean
            default: false
      database:
        type: object
        required: [host, port]
        properties:
          host:
            type: string
          port:
            type: integer
            default: 5432
    ```

Load your config with the schema:

```bash
$ holoconf get config.yaml app.debug --schema schema.yaml
false
```

Schemas catch typos and enforce type constraints before your application starts.

**Learn more:** [Validation Guide](validation.md)

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

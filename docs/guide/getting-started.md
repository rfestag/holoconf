# Getting Started

Let's build your first hierarchical configuration from scratch. By the end of this guide, you'll understand why HoloConf exists and how to use it to make your configuration flexible and secure.

## Why Hierarchical Configuration?

Imagine you're building an application. You hardcode the database host as `localhost` during development. Everything works great. Then you deploy to production and... nothing works. You forgot to change the database host.

So you use environment variables instead. Now your configuration is scattered across environment variable declarations, deployment scripts, and documentation. New team members struggle to understand what variables are needed.

HoloConf solves this by letting you write configuration files that:

- Show all possible settings in one place (self-documenting)
- Pull values from environment variables when needed (deployment flexibility)
- Provide sensible defaults (works everywhere)
- Mark sensitive values for redaction (security)

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
    holoconf = "0.1"
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

This works, but it has a problem: the database host is hardcoded to `localhost`. Let's fix that.

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

## Adding Environment Variables

We want the database host to come from an environment variable so we can change it in production without editing the file. Let's update our configuration:

```yaml
app:
  name: my-application

database:
  host: ${env:DB_HOST}
  port: 5432
```

Now let's try to use it:

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config.yaml")
    db_host = config.database.host
    # Error: ResolverError: Environment variable DB_HOST is not set
    ```

=== "Rust"

    ```rust
    let config = Config::load("config.yaml")?;
    let db_host: String = config.get("database.host")?;
    // Error: ResolverError: Environment variable DB_HOST is not set
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml database.host
    Error: Environment variable DB_HOST is not set
    ```

Oops! We got an error because we haven't set the `DB_HOST` environment variable. This is actually good - it means we won't accidentally use a wrong value. But it also means our configuration doesn't work in development without setting the variable every time.

## Adding Defaults

Let's add a default value that will be used when the environment variable isn't set:

```yaml
app:
  name: my-application

database:
  host: ${env:DB_HOST,default=localhost}
  port: 5432
```

Now try it without setting the environment variable:

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config.yaml")
    db_host = config.database.host
    print(f"Database: {db_host}")
    # Database: localhost
    ```

=== "Rust"

    ```rust
    let config = Config::load("config.yaml")?;
    let db_host: String = config.get("database.host")?;
    println!("Database: {}", db_host);
    // Database: localhost
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml database.host
    localhost
    ```

Perfect! Now let's set the environment variable and see it override the default:

=== "Python"

    ```python
    import os
    from holoconf import Config

    os.environ["DB_HOST"] = "prod-db.example.com"
    config = Config.load("config.yaml")
    db_host = config.database.host
    print(f"Database: {db_host}")
    # Database: prod-db.example.com
    ```

=== "Rust"

    ```rust
    use std::env;

    env::set_var("DB_HOST", "prod-db.example.com");
    let config = Config::load("config.yaml")?;
    let db_host: String = config.get("database.host")?;
    println!("Database: {}", db_host);
    // Database: prod-db.example.com
    ```

=== "CLI"

    ```bash
    $ DB_HOST=prod-db.example.com holoconf get config.yaml database.host
    prod-db.example.com
    ```

!!! tip "Best Practice: Always Provide Defaults"
    For environment variables, always provide a default that works for local development. This makes your configuration self-contained and easier for new developers to use. Production can override with real environment variables.

## Marking Sensitive Values

Now let's add a database password. This should come from an environment variable and should never be shown in logs or dumps. Let's add it to our configuration:

```yaml
app:
  name: my-application

database:
  host: ${env:DB_HOST,default=localhost}
  port: 5432
  password: ${env:DB_PASSWORD,default=dev-password,sensitive=true}
```

The `sensitive=true` flag tells HoloConf to redact this value when dumping the configuration:

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config.yaml")

    # Dump with redaction (default behavior)
    print(config.to_yaml(redact=True))
    # app:
    #   name: my-application
    # database:
    #   host: localhost
    #   port: 5432
    #   password: '[REDACTED]'

    # You can still access the actual value
    password = config.database.password
    print(f"Password: {password}")
    # Password: dev-password
    ```

=== "CLI"

    ```bash
    # Dump with sensitive values redacted (default)
    $ holoconf dump config.yaml --resolve
    app:
      name: my-application
    database:
      host: localhost
      port: 5432
      password: '[REDACTED]'

    # Get the actual value (use carefully!)
    $ holoconf get config.yaml database.password
    dev-password
    ```

!!! warning "Security Best Practice"
    Always mark secrets and passwords as `sensitive=true`. This prevents them from accidentally appearing in logs, error messages, or debug output.

## Working with Nested Values

HoloConf makes it easy to work with deeply nested configuration. Let's expand our example:

```yaml
app:
  name: my-application
  debug: false

database:
  host: ${env:DB_HOST,default=localhost}
  port: 5432
  password: ${env:DB_PASSWORD,default=dev-password,sensitive=true}

logging:
  level: ${env:LOG_LEVEL,default=info}
  format: json
```

You can access nested values with dot notation:

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config.yaml")

    # Get a specific nested value
    log_level = config.logging.level
    print(f"Log level: {log_level}")
    # Log level: info

    # Get an entire subsection as a dict
    db_config = config.database
    print(db_config)
    # {'host': 'localhost', 'port': 5432, 'password': 'dev-password'}
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    let config = Config::load("config.yaml")?;

    // Get a specific nested value
    let log_level: String = config.get("logging.level")?;
    println!("Log level: {}", log_level);
    // Log level: info

    // Get an entire subsection
    let db_config = config.get_section("database")?;
    ```

=== "CLI"

    ```bash
    # Get a specific nested value
    $ holoconf get config.yaml logging.level
    info

    # Dump entire resolved configuration
    $ holoconf dump config.yaml --resolve
    app:
      name: my-application
      debug: false
    database:
      host: localhost
      port: 5432
      password: '[REDACTED]'
    logging:
      level: info
      format: json
    ```

## Handling Errors

What happens when you try to access a value that doesn't exist? HoloConf gives you clear error messages:

=== "Python"

    ```python
    from holoconf import Config, PathNotFoundError

    config = Config.load("config.yaml")

    try:
        value = config.get("nonexistent.path")
    except PathNotFoundError as e:
        print(f"Oops: {e}")
        # Oops: Path not found: nonexistent.path
    ```

=== "Rust"

    ```rust
    use holoconf::{Config, Error};

    let config = Config::load("config.yaml")?;

    match config.get::<String>("nonexistent.path") {
        Ok(value) => println!("Value: {}", value),
        Err(Error::PathNotFound { path, .. }) => {
            println!("Oops: Path not found: {}", path);
        }
        Err(e) => println!("Other error: {}", e),
    }
    ```

!!! tip "Try It Yourself"
    Create your own `config.yaml` file with a few values. Try:

    - Adding more environment variables with defaults
    - Marking different values as sensitive
    - Nesting configuration deeper (3-4 levels)
    - Accessing values that don't exist to see error messages

## What You've Learned

You now know how to:

- Create a configuration file with static values
- Use environment variables with `${env:VAR_NAME}`
- Provide defaults with `${env:VAR_NAME,default=value}`
- Mark sensitive values with `sensitive=true`
- Access nested values with dot notation
- Handle missing values gracefully

This covers the basics of HoloConf. But there's much more you can do! Let's explore some more powerful features.

## Next Steps

Now that you understand the basics, let's dive deeper:

- **[Interpolation](interpolation.md)** - Learn about all the ways to create dynamic values, including nested defaults and escaping special characters
- **[Resolvers](resolvers.md)** - Explore file includes, HTTP fetching, self-references, and custom resolvers
- **[Merging](merging.md)** - Combine multiple configuration files for environment-specific settings
- **[Validation](validation.md)** - Use JSON Schema to catch configuration errors before they reach production

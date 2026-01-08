# Getting Started

This guide walks you through installing holoconf and creating your first configuration.

## Installation

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

## Your First Configuration

Create a file named `config.yaml`:

```yaml
app:
  name: my-application
  debug: false

database:
  host: ${env:DB_HOST,localhost}
  port: ${env:DB_PORT,5432}
  name: ${env:DB_NAME,myapp}

logging:
  level: ${env:LOG_LEVEL,info}
  format: json
```

This configuration demonstrates several holoconf features:

- **Nested structure** - Values organized hierarchically
- **Environment variables** - `${env:VAR_NAME}` resolves to environment variable values
- **Default values** - `${env:VAR_NAME,default}` provides fallbacks when variables aren't set

## Loading Configuration

=== "Python"

    ```python
    from holoconf import Config

    # Load from file
    config = Config.from_file("config.yaml")

    # Access values using dot notation
    app_name = config.get("app.name")
    db_host = config.get("database.host")

    print(f"App: {app_name}")
    print(f"Database: {db_host}")
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    fn main() -> Result<(), holoconf::Error> {
        // Load from file
        let config = Config::from_file("config.yaml")?;

        // Access values using dot notation
        let app_name: String = config.get("app.name")?;
        let db_host: String = config.get("database.host")?;

        println!("App: {}", app_name);
        println!("Database: {}", db_host);

        Ok(())
    }
    ```

=== "CLI"

    ```bash
    # Get a specific value
    $ holoconf get config.yaml app.name
    my-application

    # Get database host (resolves environment variable)
    $ DB_HOST=prod-db.example.com holoconf get config.yaml database.host
    prod-db.example.com

    # Dump entire resolved configuration
    $ holoconf dump config.yaml --resolve
    app:
      name: my-application
      debug: false
    database:
      host: localhost
      port: 5432
      name: myapp
    logging:
      level: info
      format: json
    ```

## Working with Nested Values

holoconf supports deep nesting and provides convenient access patterns:

=== "Python"

    ```python
    from holoconf import Config

    config = Config.from_file("config.yaml")

    # Get nested values with dot notation
    log_level = config.get("logging.level")

    # Get a subsection as a dict
    db_config = config.get("database")
    print(db_config)
    # {'host': 'localhost', 'port': 5432, 'name': 'myapp'}

    # Check if a path exists
    if config.get("app.debug"):
        print("Debug mode enabled")
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    fn main() -> Result<(), holoconf::Error> {
        let config = Config::from_file("config.yaml")?;

        // Get nested values with dot notation
        let log_level: String = config.get("logging.level")?;

        // Get a subsection
        let db_config = config.get_section("database")?;

        // Check if a path exists
        if config.get::<bool>("app.debug")? {
            println!("Debug mode enabled");
        }

        Ok(())
    }
    ```

## Error Handling

holoconf provides descriptive errors to help you debug configuration issues:

=== "Python"

    ```python
    from holoconf import Config, PathNotFoundError, ResolverError

    config = Config.from_file("config.yaml")

    try:
        value = config.get("nonexistent.path")
    except PathNotFoundError as e:
        print(f"Path not found: {e}")

    try:
        # If REQUIRED_VAR is not set and has no default
        value = config.get("some.required.value")
    except ResolverError as e:
        print(f"Failed to resolve: {e}")
    ```

=== "Rust"

    ```rust
    use holoconf::{Config, Error};

    fn main() {
        let config = Config::from_file("config.yaml").unwrap();

        match config.get::<String>("nonexistent.path") {
            Ok(value) => println!("Value: {}", value),
            Err(Error::PathNotFound { path, .. }) => {
                println!("Path not found: {}", path);
            }
            Err(e) => println!("Other error: {}", e),
        }
    }
    ```

## Next Steps

Now that you have the basics, explore these topics:

- [Interpolation](interpolation.md) - Variable substitution syntax and escaping
- [Resolvers](resolvers.md) - Environment, file, HTTP, and self-reference resolvers
- [Merging](merging.md) - Combine multiple configuration files
- [Validation](validation.md) - Validate configuration with JSON Schema

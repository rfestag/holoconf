# Interpolation

Configuration files often need values that change based on where they run. You don't want to hardcode `localhost` for your database if it needs to be `prod-db.example.com` in production. That's where interpolation comes in.

Let's explore all the ways you can make your configuration dynamic and flexible.

## The Basics: Environment Variables

We saw this in the getting started guide, but let's dive deeper. The simplest form of interpolation pulls values from environment variables:

```yaml
database:
  host: ${env:DB_HOST}
```

The syntax is straightforward: `${resolver:argument}` where:

- `resolver` is the type of value to fetch (like `env` for environment variables)
- `argument` is what to fetch (like the variable name `DB_HOST`)

Let's try using this:

=== "Python"

    ```python
    import os
    from holoconf import Config

    # Create config.yaml with: database.host = ${env:DB_HOST}
    os.environ["DB_HOST"] = "production.example.com"
    config = Config.load("config.yaml")

    host = config.database.host
    print(f"Host: {host}")
    # Host: production.example.com
    ```

=== "Rust"

    ```rust
    use holoconf::Config;
    use std::env;

    env::set_var("DB_HOST", "production.example.com");
    let config = Config::load("config.yaml")?;

    let host: String = config.get("database.host")?;
    println!("Host: {}", host);
    // Host: production.example.com
    ```

=== "CLI"

    ```bash
    $ export DB_HOST="production.example.com"
    $ holoconf get config.yaml database.host
    production.example.com
    ```

But what happens if the environment variable isn't set? Let's find out:

=== "Python"

    ```python
    from holoconf import Config, ResolverError

    # DB_HOST is not set
    config = Config.load("config.yaml")

    try:
        host = config.database.host
    except ResolverError as e:
        print(f"Error: {e}")
        # Error: Environment variable DB_HOST is not set
    ```

=== "Rust"

    ```rust
    let config = Config::load("config.yaml")?;
    let host: String = config.get("database.host")?;
    // Error: Environment variable DB_HOST is not set
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml database.host
    Error: Environment variable DB_HOST is not set
    ```

We got an error! This is actually helpful because it prevents us from using incorrect values. But we also want our configuration to work during development without requiring every environment variable to be set. That's where defaults come in.

## Adding Defaults

Let's add a default value that will be used when the environment variable isn't set:

```yaml
database:
  host: ${env:DB_HOST,default=localhost}
  port: ${env:DB_PORT,default=5432}
```

Now the configuration works whether or not the environment variables are set:

=== "Python"

    ```python
    from holoconf import Config

    # Without environment variables
    config = Config.load("config.yaml")
    host = config.database.host
    print(f"Host: {host}")
    # Host: localhost

    # With environment variables
    import os
    os.environ["DB_HOST"] = "prod-db.example.com"
    config = Config.load("config.yaml")
    host = config.database.host
    print(f"Host: {host}")
    # Host: prod-db.example.com
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    // Without environment variables
    let config = Config::load("config.yaml")?;
    let host: String = config.get("database.host")?;
    println!("Host: {}", host);
    // Host: localhost

    // With environment variables
    use std::env;
    env::set_var("DB_HOST", "prod-db.example.com");
    let config = Config::load("config.yaml")?;
    let host: String = config.get("database.host")?;
    println!("Host: {}", host);
    // Host: prod-db.example.com
    ```

=== "CLI"

    ```bash
    # Without environment variable
    $ holoconf get config.yaml database.host
    localhost

    # With environment variable
    $ DB_HOST=prod-db.example.com holoconf get config.yaml database.host
    prod-db.example.com
    ```

!!! tip "When to Use Defaults"
    Provide defaults for values that should "just work" in development, like local database hosts or sensible timeouts. Don't provide defaults for secrets or production-specific values - let those fail if not configured.

## Nested Defaults: Fallback Chains

Here's something powerful: default values can themselves contain interpolation. This lets you create fallback chains:

```yaml
api:
  # Try PRIMARY_URL first, fall back to SECONDARY_URL, then to localhost
  url: ${env:PRIMARY_URL,default=${env:SECONDARY_URL,default=http://localhost:8000}}
```

Let's see how this behaves in different scenarios:

=== "Python"

    ```python
    import os
    from holoconf import Config

    # Scenario 1: Neither variable set - uses final default
    config = Config.load("config.yaml")
    url = config.api.url
    print(f"URL: {url}")
    # URL: http://localhost:8000

    # Scenario 2: Only secondary set - uses secondary
    os.environ["SECONDARY_URL"] = "http://backup.example.com"
    config = Config.load("config.yaml")
    url = config.api.url
    print(f"URL: {url}")
    # URL: http://backup.example.com

    # Scenario 3: Primary set - uses primary (ignores secondary and default)
    os.environ["PRIMARY_URL"] = "http://primary.example.com"
    config = Config.load("config.yaml")
    url = config.api.url
    print(f"URL: {url}")
    # URL: http://primary.example.com
    ```

=== "Rust"

    ```rust
    use holoconf::Config;
    use std::env;

    // Neither variable set
    let config = Config::load("config.yaml")?;
    let url: String = config.get("api.url")?;
    println!("URL: {}", url);
    // URL: http://localhost:8000

    // Only secondary set
    env::set_var("SECONDARY_URL", "http://backup.example.com");
    let config = Config::load("config.yaml")?;
    let url: String = config.get("api.url")?;
    println!("URL: {}", url);
    // URL: http://backup.example.com
    ```

=== "CLI"

    ```bash
    # Neither variable set
    $ holoconf get config.yaml api.url
    http://localhost:8000

    # Only secondary set
    $ SECONDARY_URL=http://backup.example.com holoconf get config.yaml api.url
    http://backup.example.com

    # Primary set
    $ PRIMARY_URL=http://primary.example.com holoconf get config.yaml api.url
    http://primary.example.com
    ```

!!! note "Lazy Evaluation"
    Default values are only evaluated if needed. If `PRIMARY_URL` is set, HoloConf never even looks at `SECONDARY_URL` or the final default. This is efficient and allows defaults to reference values that might not exist.

## Marking Sensitive Values

Some configuration values should never appear in logs or debug output - like passwords, API keys, or tokens. Mark these as sensitive:

```yaml
api:
  key: ${env:API_KEY,sensitive=true}
  secret: ${env:API_SECRET,default=dev-secret,sensitive=true}
```

When you dump the configuration, sensitive values are automatically redacted:

=== "Python"

    ```python
    import os
    from holoconf import Config

    os.environ["API_KEY"] = "super-secret-key-12345"
    config = Config.load("config.yaml")

    # Dump with redaction (default)
    print(config.to_yaml(redact=True))
    # api:
    #   key: '[REDACTED]'
    #   secret: '[REDACTED]'

    # But you can still access the actual values when needed
    key = config.api.key
    print(f"Key length: {len(key)}")
    # Key length: 21
    ```

=== "CLI"

    ```bash
    $ export API_KEY="super-secret-key-12345"
    $ holoconf dump config.yaml --resolve
    api:
      key: '[REDACTED]'
      secret: '[REDACTED]'

    # Access actual value (use carefully!)
    $ holoconf get config.yaml api.key
    super-secret-key-12345
    ```

!!! warning "Security Best Practice"
    Always mark secrets, passwords, tokens, and API keys as `sensitive=true`. This prevents them from accidentally leaking into logs, error messages, or monitoring systems.

## Combining Options

You can combine `default` and `sensitive` together:

```yaml
database:
  password: ${env:DB_PASSWORD,default=dev-password,sensitive=true}
```

This gives you:

- A working default for development
- Automatic redaction in dumps
- Production can override via environment variable

## Escaping Literal Dollars

What if you actually need a literal `${` in your configuration (like documenting the syntax)? Escape it with a backslash:

```yaml
documentation:
  example: "Use \${env:VAR_NAME} to reference environment variables"
  template: "Hello \${name}, welcome to \${app}!"
```

The backslash tells HoloConf not to interpret this as interpolation:

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config.yaml")
    example = config.documentation.example
    print(example)
    # Use ${env:VAR_NAME} to reference environment variables
    ```

=== "Rust"

    ```rust
    let config = Config::load("config.yaml")?;
    let example: String = config.get("documentation.example")?;
    println!("{}", example);
    // Use ${env:VAR_NAME} to reference environment variables
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml documentation.example
    Use ${env:VAR_NAME} to reference environment variables
    ```

## Quick Reference

Here's a handy table of all interpolation syntax you'll commonly use:

| Syntax | Description | Example |
|--------|-------------|---------|
| `${env:VAR}` | Environment variable | `${env:DATABASE_URL}` |
| `${env:VAR,default=value}` | With default | `${env:PORT,default=8080}` |
| `${env:VAR,sensitive=true}` | Mark as sensitive | `${env:API_KEY,sensitive=true}` |
| `${path.to.value}` | Self-reference (absolute) | `${database.host}` |
| `${.sibling}` | Self-reference (relative) | `${.port}` |
| `${..parent.value}` | Self-reference (parent) | `${..shared.timeout}` |
| `${file:path}` | Include file content | `${file:./secrets.yaml}` |
| `${http:url}` | Fetch from HTTP | `${http:https://config.example.com/settings}` |
| `\${literal}` | Escape interpolation | `\${not_interpolated}` |

!!! tip "Try It Yourself"
    Create a `config.yaml` file and experiment:

    - Add an environment variable with nested defaults
    - Mark a value as sensitive and dump the config
    - Create a template string with escaped `${` characters
    - Try accessing the config with and without setting environment variables

## What You've Learned

You now understand:

- How interpolation syntax works: `${resolver:argument}`
- Using environment variables with `${env:VAR_NAME}`
- Providing fallback values with `default=value`
- Creating fallback chains with nested defaults
- Protecting secrets with `sensitive=true`
- Combining multiple options together
- Escaping literal `${` with backslashes

## Next Steps

We've focused on environment variables here because they're the most common use case. But HoloConf supports many other resolvers:

- **[Resolvers](resolvers.md)** - Explore all available resolvers: file includes, HTTP fetching, self-references, and custom resolvers
- **[Merging](merging.md)** - Combine multiple configuration files
- **[Validation](validation.md)** - Validate configuration with JSON Schema

## See Also

- [ADR-011 Interpolation Syntax](../adr/ADR-011-interpolation-syntax.md) - Technical details and design rationale

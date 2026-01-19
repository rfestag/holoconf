# Interpolation

Configuration files often need values that change based on where they run or reference other configuration values. You don't want to hardcode `localhost` for your database if it needs to be `prod-db.example.com` in production. That's where interpolation comes in.

Interpolation lets you insert dynamic values into your configuration using a simple syntax: `${...}`. Let's explore how it works.

## Self-References: Avoiding Duplication

The simplest form of interpolation references other values in the same configuration. This helps you avoid duplicating information:

```yaml
server:
  host: api.example.com
  port: 8080
  url: https://${server.host}:${server.port}
```

When you access `server.url`, HoloConf automatically resolves the references:

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config.yaml")
    url = config.server.url
    print(url)
    # https://api.example.com:8080
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    let config = Config::load("config.yaml")?;
    let url: String = config.get("server.url")?;
    println!("{}", url);
    // https://api.example.com:8080
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml server.url
    https://api.example.com:8080
    ```

### Absolute References (Default)

By default, references are **absolute** - they specify the full path from the root of the configuration:

```yaml
database:
  host: db.example.com
  port: 5432

app:
  connection: postgres://${database.host}:${database.port}/mydb
```

### Relative References

You can also use **relative references** with a dot prefix. This is especially useful for referencing siblings or parent values:

```yaml
database:
  host: localhost
  port: 5432
  url: postgres://${.host}:${.port}/db  # References siblings
```

The dot syntax works like filesystem paths:

- **`.sibling`** - Reference a sibling at the current level
- **`..parent`** - Go up one level (parent)
- **`...grandparent`** - Go up two levels (grandparent)
- And so on...

Here's a more complex example:

```yaml
company:
  name: Example Corp
  domain: example.com

  engineering:
    email_domain: ${..domain}  # Parent: company.domain

    backend:
      team_name: ${...name} Backend Team  # Grandparent: company.name
      contact: backend@${..email_domain}  # Parent: engineering.email_domain
```

When accessed:

=== "Python"

    ```python
    config = Config.load("config.yaml")

    print(config.company.engineering.backend.team_name)
    # Example Corp Backend Team

    print(config.company.engineering.backend.contact)
    # backend@example.com
    ```

=== "Rust"

    ```rust
    let config = Config::load("config.yaml")?;

    let team: String = config.get("company.engineering.backend.team_name")?;
    println!("{}", team);
    // Example Corp Backend Team

    let contact: String = config.get("company.engineering.backend.contact")?;
    println!("{}", contact);
    // backend@example.com
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml company.engineering.backend.team_name
    Example Corp Backend Team

    $ holoconf get config.yaml company.engineering.backend.contact
    backend@example.com
    ```

!!! tip "When to Use Relative vs Absolute"
    Use **absolute** references when you need a specific value regardless of where you are in the config. Use **relative** references when you want to keep sections self-contained and easier to refactor.

## Understanding the Interpolation Syntax

Self-references are just one type of interpolation. The general syntax is:

```
${resolver:argument,param1=value1,param2=value2}
```

Let's break this down:

- **`resolver`** - Where to get the value from (like `env` for environment variables, `file` for files, etc.)
- **`argument`** - What to fetch (like a variable name or file path)
- **`param=value`** - Optional parameters that modify the behavior

For self-references, the resolver is implicit (no prefix), and the argument is just the path.

## Framework-Level Parameters

HoloConf automatically implements semantics for certain parameters so each resolver doesn't have to. These work with **any** resolver:

### Default Values

Add a fallback value that's used when resolution fails:

```yaml
database:
  host: ${env:DB_HOST,default=localhost}
  port: ${env:DB_PORT,default=5432}
```

Now your configuration works whether or not the environment variables are set:

=== "Python"

    ```python
    import os
    from holoconf import Config

    # Without environment variables - uses defaults
    config = Config.load("config.yaml")
    print(config.database.host)
    # localhost

    # With environment variables - uses actual values
    os.environ["DB_HOST"] = "prod-db.example.com"
    config = Config.load("config.yaml")
    print(config.database.host)
    # prod-db.example.com
    ```

=== "Rust"

    ```rust
    use holoconf::Config;
    use std::env;

    // Without environment variables
    let config = Config::load("config.yaml")?;
    let host: String = config.get("database.host")?;
    println!("{}", host);
    // localhost

    // With environment variables
    env::set_var("DB_HOST", "prod-db.example.com");
    let config = Config::load("config.yaml")?;
    let host: String = config.get("database.host")?;
    println!("{}", host);
    // prod-db.example.com
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

#### Nested Defaults: Fallback Chains

Default values can themselves contain interpolation, creating fallback chains:

```yaml
api:
  # Try PRIMARY_URL first, fall back to SECONDARY_URL, then to localhost
  url: ${env:PRIMARY_URL,default=${env:SECONDARY_URL,default=http://localhost:8000}}
```

=== "Python"

    ```python
    import os
    from holoconf import Config

    # Neither variable set - uses final default
    config = Config.load("config.yaml")
    print(config.api.url)
    # http://localhost:8000

    # Only secondary set
    os.environ["SECONDARY_URL"] = "http://backup.example.com"
    config = Config.load("config.yaml")
    print(config.api.url)
    # http://backup.example.com

    # Primary set - uses primary (ignores secondary)
    os.environ["PRIMARY_URL"] = "http://primary.example.com"
    config = Config.load("config.yaml")
    print(config.api.url)
    # http://primary.example.com
    ```

=== "Rust"

    ```rust
    use holoconf::Config;
    use std::env;

    // Neither set - uses final default
    let config = Config::load("config.yaml")?;
    let url: String = config.get("api.url")?;
    println!("{}", url);
    // http://localhost:8000

    // Only secondary set
    env::set_var("SECONDARY_URL", "http://backup.example.com");
    let config = Config::load("config.yaml")?;
    let url: String = config.get("api.url")?;
    println!("{}", url);
    // http://backup.example.com
    ```

=== "CLI"

    ```bash
    # Neither set
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
    Default values are only evaluated if needed. If the primary value succeeds, HoloConf never evaluates the default. This is efficient and allows defaults to reference values that might not exist.

### Sensitive Values

Mark values that should never appear in logs or debug output:

```yaml
api:
  key: ${env:API_KEY,sensitive=true}
  secret: ${env:API_SECRET,sensitive=true}
```

When you dump the configuration, sensitive values are automatically redacted:

=== "Python"

    ```python
    import os
    from holoconf import Config

    os.environ["API_KEY"] = "super-secret-key-12345"
    os.environ["API_SECRET"] = "super-secret-value"
    config = Config.load("config.yaml")

    # Dump with redaction
    print(config.to_yaml(redact=True))
    # api:
    #   key: '[REDACTED]'
    #   secret: '[REDACTED]'

    # But you can still access actual values when needed
    key = config.api.key
    print(f"Key length: {len(key)}")
    # Key length: 21
    ```

=== "Rust"

    ```rust
    use holoconf::Config;
    use std::env;

    env::set_var("API_KEY", "super-secret-key-12345");
    env::set_var("API_SECRET", "super-secret-value");
    let config = Config::load("config.yaml")?;

    // Dump with redaction
    let yaml = config.to_yaml(true, true)?;  // resolve=true, redact=true
    println!("{}", yaml);
    // api:
    //   key: '[REDACTED]'
    //   secret: '[REDACTED]'

    // But you can still access actual values
    let key: String = config.get("api.key")?;
    println!("Key length: {}", key.len());
    // Key length: 21
    ```

=== "CLI"

    ```bash
    $ export API_KEY="super-secret-key-12345"
    $ export API_SECRET="super-secret-value"

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

### Combining Parameters

You can use multiple framework-level parameters together:

```yaml
database:
  password: ${env:DB_PASSWORD,default=dev-password,sensitive=true}
```

This gives you:

- A working default for development
- Automatic redaction in dumps
- Production can override via environment variable

## Example: Environment Variables

Let's see a concrete example using the `env` resolver to pull values from environment variables:

```yaml
database:
  host: ${env:DB_HOST,default=localhost}
  port: ${env:DB_PORT,default=5432}
  password: ${env:DB_PASSWORD,sensitive=true}
  url: postgres://${.host}:${.port}/mydb
```

This configuration combines several interpolation techniques:

- **Environment variables** with defaults (`env:DB_HOST,default=localhost`)
- **Sensitive values** for secrets (`env:DB_PASSWORD,sensitive=true`)
- **Self-references** to build URLs (`${.host}`)

=== "Python"

    ```python
    import os
    from holoconf import Config

    # Set production values
    os.environ["DB_HOST"] = "prod-db.example.com"
    os.environ["DB_PORT"] = "5433"
    os.environ["DB_PASSWORD"] = "super-secret"

    config = Config.load("config.yaml")

    # Access values
    print(config.database.host)      # prod-db.example.com
    print(config.database.port)      # 5433
    print(config.database.url)       # postgres://prod-db.example.com:5433/mydb

    # Dump config (password is redacted)
    print(config.to_yaml(redact=True))
    # database:
    #   host: prod-db.example.com
    #   port: 5433
    #   password: '[REDACTED]'
    #   url: postgres://prod-db.example.com:5433/mydb
    ```

=== "Rust"

    ```rust
    use holoconf::Config;
    use std::env;

    // Set production values
    env::set_var("DB_HOST", "prod-db.example.com");
    env::set_var("DB_PORT", "5433");
    env::set_var("DB_PASSWORD", "super-secret");

    let config = Config::load("config.yaml")?;

    // Access values
    let host: String = config.get("database.host")?;
    let port: i64 = config.get("database.port")?;
    let url: String = config.get("database.url")?;

    println!("{}", host);  // prod-db.example.com
    println!("{}", port);  // 5433
    println!("{}", url);   // postgres://prod-db.example.com:5433/mydb
    ```

=== "CLI"

    ```bash
    $ export DB_HOST="prod-db.example.com"
    $ export DB_PORT="5433"
    $ export DB_PASSWORD="super-secret"

    $ holoconf get config.yaml database.host
    prod-db.example.com

    $ holoconf get config.yaml database.url
    postgres://prod-db.example.com:5433/mydb

    $ holoconf dump config.yaml --resolve
    database:
      host: prod-db.example.com
      port: 5433
      password: '[REDACTED]'
      url: postgres://prod-db.example.com:5433/mydb
    ```

## Other Resolvers

HoloConf supports many types of resolvers beyond environment variables and self-references:

- **`file`** - Include content from files
- **`http/https`** - Fetch from HTTP endpoints
- **`env`** - Environment variables (shown above)
- **`ssm`** - AWS Systems Manager Parameter Store
- **`cfn`** - CloudFormation stack outputs
- **Custom resolvers** - Write your own!

All resolvers support the framework-level parameters (`default=` and `sensitive=`), so you can use the same patterns everywhere.

For detailed information on each resolver and resolver-specific parameters, see the [Resolvers](resolvers.md) section.

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

| Syntax | Description | Example |
|--------|-------------|---------|
| `${path.to.value}` | Self-reference (absolute) | `${database.host}` |
| `${.sibling}` | Self-reference (current level) | `${.port}` |
| `${..parent}` | Self-reference (parent level) | `${..shared.timeout}` |
| `${...grandparent}` | Self-reference (grandparent level) | `${...company.name}` |
| `${resolver:arg}` | Call a resolver | `${env:DB_HOST}` |
| `${resolver:arg,default=val}` | With default | `${env:PORT,default=8080}` |
| `${resolver:arg,sensitive=true}` | Mark as sensitive | `${env:API_KEY,sensitive=true}` |
| `\${literal}` | Escape interpolation | `\${not_interpolated}` |

## What You've Learned

You now understand:

- **Self-references** - Referencing other config values (absolute and relative)
- **Interpolation syntax** - `${resolver:argument,param=value}`
- **Framework-level parameters** - `default=` and `sensitive=` work with all resolvers
- **Relative references** - Using `.`, `..`, `...` to navigate the config tree
- **Fallback chains** - Nested defaults for resilience
- **Security** - Marking sensitive values for automatic redaction
- **Escaping** - Using `\$` for literal dollar signs

## Next Steps

- **[Resolvers](resolvers.md)** - Explore all available resolvers and their specific features
- **[Merging](merging.md)** - Combine multiple configuration files with layered overrides
- **[Validation](validation.md)** - Catch configuration errors early with JSON Schema

## See Also

- [ADR-011 Interpolation Syntax](../adr/ADR-011-interpolation-syntax.md) - Technical details and design rationale

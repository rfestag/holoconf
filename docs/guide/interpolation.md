# Interpolation

As your configuration grows, you'll find yourself repeating the same values in multiple places. You might define a hostname once and then need to use it in several URLs. Or you might want to build connection strings from smaller pieces. Repeating these values makes your configuration harder to maintain and error-prone.

Interpolation solves this by letting you reference values within your configuration using a simple syntax: `${...}`. Let's explore how it works.

## Self-References: Keeping Configuration DRY

The most common use of interpolation is referencing other values in the same configuration file. This keeps your configuration DRY (Don't Repeat Yourself):

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

Now when you need to change the hostname, you only update it in one place.

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

You can also use **relative references** with a dot prefix. This is especially useful for keeping configuration sections self-contained:

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

Here's a more complex example showing how relative references make sections more modular:

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

## Optional Values with Defaults

Sometimes you want to reference a configuration value that might not exist - like optional feature flags or environment-specific settings. By default, referencing a missing path throws an error:

```yaml
features:
  beta: true
  # No 'experimental' flag defined

app:
  experimental_enabled: ${features.experimental}  # ERROR: Path not found!
```

To handle optional values gracefully, use the `default=` parameter:

```yaml
features:
  beta: true

app:
  # Returns 'false' if features.experimental doesn't exist
  experimental_enabled: ${features.experimental,default=false}

  # Returns default even if the value is null
  timeout: ${custom.timeout,default=30}
```

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config.yaml")

    print(config.app.experimental_enabled)
    # false (from default, since features.experimental doesn't exist)

    print(config.app.timeout)
    # 30 (from default)
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    let config = Config::load("config.yaml")?;

    let enabled: bool = config.get("app.experimental_enabled")?;
    println!("{}", enabled);
    // false

    let timeout: i64 = config.get("app.timeout")?;
    println!("{}", timeout);
    // 30
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml app.experimental_enabled
    false

    $ holoconf get config.yaml app.timeout
    30
    ```

### When Defaults Are Used

The `default=` parameter is used when:

- The path doesn't exist in the configuration
- The value exists but is explicitly `null`

If the value exists and is not null, the actual value is used (the default is ignored).

### Defaults Can Reference Other Values

Your default value can itself be an interpolation:

```yaml
defaults:
  timeout: 30
  retry_count: 3

service:
  # Uses service-specific timeout if defined, otherwise falls back to default
  timeout: ${service.custom_timeout,default=${defaults.timeout}}
  retries: ${service.custom_retries,default=${defaults.retry_count}}
```

### Explicit `ref:` Resolver

Under the hood, self-references like `${path}` are actually using a special `ref:` resolver. You can write this explicitly if you prefer:

```yaml
# These are equivalent:
value1: ${database.host,default=localhost}
value2: ${ref:database.host,default=localhost}
```

The explicit form can be clearer when you're already using other resolvers and want consistency.

### Marking References as Sensitive

Like other resolvers, `ref` supports the `sensitive` flag to mark values as sensitive (they'll be redacted in logs and output):

```yaml
secrets:
  api_key: prod-key-12345

# Mark the referenced value as sensitive
app:
  key: ${secrets.api_key,sensitive=true}
  # Can combine with default
  backup_key: ${secrets.backup_key,default=dev-key,sensitive=true}
```

This is useful when referencing secrets stored elsewhere in your configuration.

!!! tip "When to Use Defaults"
    Use defaults for:

    - **Optional feature flags** that may not be defined in all environments
    - **Environment-specific overrides** that only exist in some deployments
    - **Graceful degradation** when optional configuration sections are missing

    Avoid defaults for **required configuration** - let errors surface early!

## Resolvers: Getting Values from External Sources

Self-references are powerful, but sometimes you need values from **outside** your configuration file - like environment variables, files, external services, or cloud provider APIs. This is where **resolvers** come in.

A resolver is a mechanism for fetching values from external sources. The syntax extends what you've already learned:

```
${resolver:argument}
```

For example, to pull a value from an environment variable:

```yaml
database:
  host: ${env:DB_HOST}
  port: ${env:DB_PORT}
  url: postgres://${.host}:${.port}/db  # Combines env values with self-reference
```

=== "Python"

    ```python
    import os
    from holoconf import Config

    os.environ["DB_HOST"] = "prod-db.example.com"
    os.environ["DB_PORT"] = "5432"

    config = Config.load("config.yaml")
    print(config.database.url)
    # postgres://prod-db.example.com:5432/db
    ```

=== "Rust"

    ```rust
    use holoconf::Config;
    use std::env;

    env::set_var("DB_HOST", "prod-db.example.com");
    env::set_var("DB_PORT", "5432");

    let config = Config::load("config.yaml")?;
    let url: String = config.get("database.url")?;
    println!("{}", url);
    // postgres://prod-db.example.com:5432/db
    ```

=== "CLI"

    ```bash
    $ export DB_HOST="prod-db.example.com"
    $ export DB_PORT="5432"
    $ holoconf get config.yaml database.url
    postgres://prod-db.example.com:5432/db
    ```

HoloConf includes several built-in resolvers for common use cases:

- **`env`** - Environment variables
- **`file`** - File contents
- **`http`/`https`** - Remote content via HTTP
- **`ssm`** - AWS Systems Manager Parameter Store (via plugin)
- **`cfn`** - CloudFormation stack outputs (via plugin)

You can even write your own custom resolvers in Python!

For detailed information on all available resolvers, their specific features, and advanced capabilities like defaults and sensitive value handling, see the [Resolvers](resolvers.md) section.

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
| `${path,default=value}` | Self-reference with default | `${features.beta,default=false}` |
| `${path,sensitive=true}` | Mark reference as sensitive | `${secrets.key,sensitive=true}` |
| `${.sibling}` | Self-reference (current level) | `${.port}` |
| `${..parent}` | Self-reference (parent level) | `${..shared.timeout}` |
| `${...grandparent}` | Self-reference (grandparent level) | `${...company.name}` |
| `${resolver:arg}` | External value via resolver | `${env:DB_HOST}` |
| `${resolver:arg,default=value}` | Resolver with default | `${env:API_KEY,default=dev-key}` |
| `${ref:path}` | Explicit self-reference | `${ref:database.host}` |
| `\${literal}` | Escape interpolation | `\${not_interpolated}` |

## What You've Learned

You now understand:

- **Interpolation basics** - Using `${...}` to avoid repeating values
- **Self-references** - Referencing other config values (absolute and relative)
- **Relative references** - Using `.`, `..`, `...` to navigate the config tree
- **Optional values** - Using `default=` to handle missing or null values gracefully
- **Resolvers** - Fetching values from external sources like environment variables
- **Escaping** - Using `\${` for literal dollar signs

## Next Steps

- **[Resolvers](resolvers.md)** - Deep dive into all available resolvers, defaults, sensitive values, and custom resolvers
- **[Merging](merging.md)** - Combine multiple configuration files with layered overrides
- **[Validation](validation.md)** - Catch configuration errors early with JSON Schema

## See Also

- [ADR-011 Interpolation Syntax](../adr/ADR-011-interpolation-syntax.md) - Technical details and design rationale

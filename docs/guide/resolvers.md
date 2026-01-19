# Resolvers

Configuration often needs to pull values from different sources: environment variables, other files, HTTP endpoints, cloud services, or even other parts of the configuration itself. This is where resolvers come in.

## What Are Resolvers?

A resolver is a plugin that knows how to fetch a value from a specific source. When you write something like:

```yaml
database:
  password: ${env:DB_PASSWORD}
```

The `env` part is the resolver name, and `DB_PASSWORD` is the argument to that resolver. HoloConf sees this syntax and calls the `env` resolver to fetch the value.

Let's see a simple example:

=== "Python"

    ```python
    from holoconf import Config

    # config.yaml contains: database.host = ${env:DB_HOST,default=localhost}
    config = Config.load("config.yaml")

    # When you access this value, the env resolver runs
    host = config.database.host
    print(f"Host: {host}")
    # Host: localhost (from the default)
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    let config = Config::load("config.yaml")?;

    // When you access this value, the env resolver runs
    let host: String = config.get("database.host")?;
    println!("Host: {}", host);
    // Host: localhost
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml database.host
    localhost
    ```

## Resolver Syntax

All resolvers use the same syntax:

```
${resolver:argument,option=value,option2=value2}
```

Breaking this down:

- `${...}` - Marks this as interpolation
- `resolver` - The resolver name (like `env`, `file`, `http`)
- `argument` - What to fetch (like a variable name, file path, or URL)
- `option=value` - Optional parameters (like `default`, `sensitive`, `timeout`)

## Framework-Level Options

Some options work with ALL resolvers, not just specific ones:

### default - Fallback Values

If the resolver can't find the value, use this instead:

```yaml
database:
  host: ${env:DB_HOST,default=localhost}
  port: ${env:DB_PORT,default=5432}
```

=== "Python"

    ```python
    from holoconf import Config

    # DB_HOST not set in environment
    config = Config.load("config.yaml")
    host = config.database.host
    print(f"Host: {host}")
    # Host: localhost (from default)
    ```

=== "Rust"

    ```rust
    let config = Config::load("config.yaml")?;
    let host: String = config.get("database.host")?;
    println!("Host: {}", host);
    // Host: localhost
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml database.host
    localhost
    ```

### sensitive - Automatic Redaction

Mark values that should never appear in logs or dumps:

```yaml
api:
  key: ${env:API_KEY,sensitive=true}
  secret: ${env:API_SECRET,default=dev-secret,sensitive=true}
```

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config.yaml")

    # Value is accessible
    key = config.api.key
    print(f"Key length: {len(key)}")
    # Key length: 10

    # But redacted in dumps
    print(config.to_yaml(redact=True))
    # api:
    #   key: '[REDACTED]'
    #   secret: '[REDACTED]'
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml api.key
    dev-secret

    $ holoconf dump config.yaml --resolve
    api:
      key: '[REDACTED]'
      secret: '[REDACTED]'
    ```

### Nested Defaults - Fallback Chains

Default values can themselves use resolvers, creating fallback chains:

```yaml
api:
  # Try PRIMARY_URL, fall back to SECONDARY_URL, then to localhost
  url: ${env:PRIMARY_URL,default=${env:SECONDARY_URL,default=http://localhost:8000}}
```

=== "Python"

    ```python
    import os
    from holoconf import Config

    # Neither variable set - uses final default
    config = Config.load("config.yaml")
    url = config.api.url
    print(f"URL: {url}")
    # URL: http://localhost:8000

    # Only secondary set - uses secondary
    os.environ["SECONDARY_URL"] = "http://backup.example.com"
    config = Config.load("config.yaml")
    url = config.api.url
    print(f"URL: {url}")
    # URL: http://backup.example.com
    ```

=== "Rust"

    ```rust
    use std::env;
    use holoconf::Config;

    // Neither variable set
    let config = Config::load("config.yaml")?;
    let url: String = config.get("api.url")?;
    println!("URL: {}", url);
    // URL: http://localhost:8000
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml api.url
    http://localhost:8000

    $ SECONDARY_URL=http://backup.example.com holoconf get config.yaml api.url
    http://backup.example.com
    ```

!!! note "Lazy Evaluation"
    Default values are only evaluated if needed. If `PRIMARY_URL` is set, HoloConf never even looks at `SECONDARY_URL` or the final default.

## Lazy Resolution

Here's something important: resolvers are invoked **lazily** - values are only resolved when you access them, not when the configuration is loaded.

```yaml
expensive:
  data: ${http:https://slow-api.example.com/data}  # Not fetched during load
cached:
  value: ${env:CACHE_KEY}  # Not read during load
```

=== "Python"

    ```python
    from holoconf import Config

    # This is fast - no resolvers run yet
    config = Config.load("config.yaml", allow_http=True)

    # Only now does the HTTP resolver run
    data = config.expensive.data  # Fetches from HTTP

    # If you never access cached.value, CACHE_KEY is never read
    ```

This makes HoloConf faster and more flexible:
- Values you don't access aren't resolved
- Default values are only evaluated if the primary resolver fails
- You can load configuration without requiring all resources to be available

## Available Resolvers

HoloConf provides several built-in resolvers:

### Core Resolvers

Always available, no extra installation needed:

- **[env](resolvers-core.md#environment-variables)** - Environment variables
- **[Self-references](resolvers-core.md#self-references-avoiding-duplication)** - Reference other config values (absolute and relative paths)
- **[file](resolvers-core.md#file-includes-splitting-large-configurations)** - Include content from files
- **[http/https](resolvers-core.md#httphttps-remote-configuration)** - Fetch from HTTP endpoints

### AWS Resolvers

Requires `pip install holoconf-aws`:

- **[ssm](resolvers-aws.md#ssm-parameter-store)** - AWS Systems Manager Parameter Store
- **[cfn](resolvers-aws.md#cloudformation-outputs)** - CloudFormation stack outputs
- **[s3](resolvers-aws.md#s3-objects)** - S3 object content

### Custom Resolvers

You can create your own resolvers to integrate with any data source:

- **[Custom Resolvers Guide](resolvers-custom.md)** - Learn how to write your own

## Quick Reference

Here's a handy table of the most common resolver patterns:

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
| `${ssm:/path}` | AWS SSM Parameter | `${ssm:/app/prod/db-password}` |
| `\${literal}` | Escape interpolation | `\${not_interpolated}` |

## What You've Learned

You now understand:

- What resolvers are and how they work
- The `${resolver:argument,option=value}` syntax
- Framework-level options: `default` and `sensitive`
- Nested defaults for fallback chains
- Lazy resolution behavior
- What resolvers are available

## Next Steps

Now dive into specific resolver types:

- **[Core Resolvers](resolvers-core.md)** - Environment variables, self-references, file includes, HTTP fetching
- **[AWS Resolvers](resolvers-aws.md)** - SSM, CloudFormation, S3 integration
- **[Custom Resolvers](resolvers-custom.md)** - Write your own resolvers

Or continue with other topics:

- **[Merging](merging.md)** - Combine multiple configuration files
- **[Validation](validation.md)** - Use JSON Schema to catch configuration errors

## See Also

- [ADR-002 Resolver Architecture](../adr/ADR-002-resolver-architecture.md) - Design rationale for resolvers
- [ADR-011 Interpolation Syntax](../adr/ADR-011-interpolation-syntax.md) - Technical details
- [FEAT-002 Core Resolvers](../specs/features/FEAT-002-core-resolvers.md) - Full resolver specification

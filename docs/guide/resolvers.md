# Resolvers

Configuration often needs to pull values from different sources: environment variables, other files, HTTP endpoints, or even other parts of the configuration itself. Resolvers are how HoloConf makes this happen.

Let's explore each resolver type, starting simple and building up to more advanced use cases.

## Environment Variables: The Basics

We've already seen environment variables in the previous guides, but let's make sure we understand them completely. The `env` resolver reads values from environment variables:

```yaml
database:
  host: ${env:DB_HOST}
```

Let's see what happens when we try to use this:

=== "Python"

    ```python
    from holoconf import Config, ResolverError

    # Without setting the environment variable
    config = Config.load("config.yaml")

    try:
        host = config.get("database.host")
    except ResolverError as e:
        print(f"Error: {e}")
        # Error: Environment variable DB_HOST is not set
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    let config = Config::load("config.yaml")?;
    let host: String = config.get("database.host")?;
    // Error: ResolverError: Environment variable DB_HOST is not set
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml database.host
    Error: Environment variable DB_HOST is not set
    ```

The error is good - it prevents us from using incorrect values. But now let's add a default so it works during development:

```yaml
database:
  host: ${env:DB_HOST,default=localhost}
  port: ${env:DB_PORT,default=5432}
```

Now it works everywhere:

=== "Python"

    ```python
    import os
    from holoconf import Config

    # Without environment variables - uses defaults
    config = Config.load("config.yaml")
    host = config.get("database.host")
    print(f"Host: {host}")
    # Host: localhost

    # With environment variables - overrides defaults
    os.environ["DB_HOST"] = "prod-db.example.com"
    config = Config.load("config.yaml")
    host = config.get("database.host")
    print(f"Host: {host}")
    # Host: prod-db.example.com
    ```

=== "Rust"

    ```rust
    use holoconf::Config;
    use std::env;

    // Without environment variables
    let config = Config::load("config.yaml")?;
    let host: String = config.get("database.host")?;
    println!("Host: {}", host);
    // Host: localhost

    // With environment variables
    env::set_var("DB_HOST", "prod-db.example.com");
    let config = Config::load("config.yaml")?;
    let host: String = config.get("database.host")?;
    println!("Host: {}", host);
    // Host: prod-db.example.com
    ```

=== "CLI"

    ```bash
    # Uses default
    $ holoconf get config.yaml database.host
    localhost

    # Overrides with environment variable
    $ DB_HOST=prod-db.example.com holoconf get config.yaml database.host
    prod-db.example.com
    ```

Finally, let's add a password and mark it as sensitive so it never appears in logs:

```yaml
database:
  host: ${env:DB_HOST,default=localhost}
  port: ${env:DB_PORT,default=5432}
  password: ${env:DB_PASSWORD,default=dev-password,sensitive=true}
```

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config.yaml")

    # The password is accessible
    password = config.get("database.password")
    print(f"Password length: {len(password)}")
    # Password length: 12

    # But it's redacted in dumps
    print(config.to_yaml(redact=True))
    # database:
    #   host: localhost
    #   port: 5432
    #   password: '[REDACTED]'
    ```

=== "CLI"

    ```bash
    # Can access it directly
    $ holoconf get config.yaml database.password
    dev-password

    # But it's redacted in dumps
    $ holoconf dump config.yaml --resolve
    database:
      host: localhost
      port: 5432
      password: '[REDACTED]'
    ```

!!! tip "Environment Variable Best Practices"
    - Always provide defaults for non-sensitive values (like hosts and ports)
    - Always mark sensitive values with `sensitive=true`
    - Use uppercase names for environment variables (convention)
    - Don't provide defaults for production secrets - let them fail if not configured

## Self-References: Reusing Values

Sometimes you want to reference other values in your configuration. This helps you avoid repetition and keep related values in sync.

### Absolute References

Reference any value in your configuration using its full path:

```yaml
api:
  base_url: https://api.example.com

endpoints:
  users: ${api.base_url}/users
  orders: ${api.base_url}/orders
  products: ${api.base_url}/products
```

Let's see this in action:

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config.yaml")

    users_endpoint = config.get("endpoints.users")
    print(f"Users: {users_endpoint}")
    # Users: https://api.example.com/users

    orders_endpoint = config.get("endpoints.orders")
    print(f"Orders: {orders_endpoint}")
    # Orders: https://api.example.com/orders
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    let config = Config::load("config.yaml")?;

    let users: String = config.get("endpoints.users")?;
    println!("Users: {}", users);
    // Users: https://api.example.com/users
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml endpoints.users
    https://api.example.com/users
    ```

Now when you need to change the base URL, you only update it in one place!

### Relative References

Use `.` to reference sibling values (same level) and `..` to go up levels:

```yaml
defaults:
  timeout: 30
  retries: 3

database:
  host: localhost
  port: 5432
  connection:
    # Reference siblings (same level as 'connection')
    url: postgres://${..host}:${..port}/mydb
    # Reference from defaults (parent's sibling)
    timeout: ${...defaults.timeout}
    retries: ${...defaults.retries}
```

Let's break down what's happening:
- `${..host}` means "go up one level (from `connection` to `database`) and get `host`"
- `${...defaults.timeout}` means "go up to root and get `defaults.timeout`"

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config.yaml")

    url = config.get("database.connection.url")
    print(f"URL: {url}")
    # URL: postgres://localhost:5432/mydb

    timeout = config.get("database.connection.timeout")
    print(f"Timeout: {timeout}")
    # Timeout: 30
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    let config = Config::load("config.yaml")?;

    let url: String = config.get("database.connection.url")?;
    println!("URL: {}", url);
    // URL: postgres://localhost:5432/mydb
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml database.connection.url
    postgres://localhost:5432/mydb
    ```

!!! tip "When to Use Self-References"
    Self-references are perfect for:

    - Building URLs from base URLs and paths
    - Constructing connection strings from individual components
    - Sharing common values (like timeouts) across multiple sections
    - Avoiding duplication in your configuration

## File Includes: Splitting Configuration

As your configuration grows, you might want to split it across multiple files. The `file` resolver lets you include content from other files:

```yaml
# main.yaml
app:
  name: my-application
  secrets: ${file:./secrets.yaml}
  database: ${file:./database.yaml}
```

Let's create those files and see it work:

=== "Python"

    ```python
    from holoconf import Config

    # Assuming secrets.yaml contains: api_key: secret-123
    # And database.yaml contains: host: localhost, port: 5432

    config = Config.load("main.yaml")

    api_key = config.get("app.secrets.api_key")
    print(f"API Key: {api_key}")
    # API Key: secret-123

    db_host = config.get("app.database.host")
    print(f"Database: {db_host}")
    # Database: localhost
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    let config = Config::load("main.yaml")?;

    let api_key: String = config.get("app.secrets.api_key")?;
    println!("API Key: {}", api_key);
    // API Key: secret-123
    ```

=== "CLI"

    ```bash
    $ holoconf get main.yaml app.secrets.api_key
    secret-123
    ```

!!! warning "Security: Path Traversal Protection"
    By default, file access is restricted to the configuration file's directory. Attempts to read files outside this directory (like `${file:/etc/passwd}`) will be blocked. This prevents malicious configuration files from accessing sensitive system files.

### Security: Allowed Directories

HoloConf automatically allows reading files from the same directory as your config file. If you need to read files from other directories, you must explicitly allow them:

=== "Python"

    ```python
    from holoconf import Config

    # Auto-allowed: files in same directory as config
    config = Config.load("/app/config.yaml")
    # ✓ Can read: /app/data.yaml, /app/subdir/file.yaml
    # ✗ Blocked: /etc/passwd, /other/path/file.yaml

    # Allow additional directories
    config = Config.load(
        "/app/config.yaml",
        file_roots=["/etc/myapp", "/var/lib/myapp"]
    )
    # ✓ Can now read: /app/*, /etc/myapp/*, /var/lib/myapp/*
    ```

=== "Rust"

    ```rust
    use holoconf::{Config, ConfigOptions};
    use std::path::PathBuf;

    // Auto-allowed: files in same directory
    let config = Config::load("/app/config.yaml")?;

    // Allow additional directories
    let mut options = ConfigOptions::default();
    options.file_roots = vec![
        PathBuf::from("/etc/myapp"),
        PathBuf::from("/var/lib/myapp")
    ];
    let config = Config::load_with_options("/app/config.yaml", options)?;
    ```

=== "CLI"

    ```bash
    # File access limited to config directory by default
    holoconf get app.secrets /app/config.yaml
    # ✓ Reads /app/secrets.yaml
    # ✗ Would block /etc/passwd reference
    ```

### Optional Files

What if you want to include a file that might not exist (like local developer overrides)? Use a default:

```yaml
app:
  name: my-application
  # If local.yaml doesn't exist, use empty object
  local_config: ${file:./local.yaml,default={}}
```

### Parse Modes

By default, HoloConf detects the file format from the extension. You can override this:

| Option | Description | Example |
|--------|-------------|---------|
| `parse=auto` | Auto-detect from extension (default) | `${file:config.yaml}` |
| `parse=yaml` | Parse as YAML | `${file:data.txt,parse=yaml}` |
| `parse=json` | Parse as JSON | `${file:data.txt,parse=json}` |
| `parse=text` | Read as plain text | `${file:template.yaml,parse=text}` |
| `encoding=utf-8` | UTF-8 encoding (default) | `${file:data.txt}` |
| `encoding=base64` | Base64 encode contents | `${file:cert.pem,encoding=base64}` |

## HTTP Fetching: Remote Configuration

Sometimes configuration lives on a remote server - feature flags from a service, shared settings from a central configuration server, or secrets from a vault. The `http` resolver can fetch these.

!!! danger "Security: HTTP Resolver Disabled by Default"
    The HTTP resolver is **disabled by default** for security. You must explicitly enable it when loading your configuration.

### Enabling HTTP Resolver

First, let's see what happens if we try to use HTTP without enabling it:

=== "Python"

    ```python
    from holoconf import Config

    # config.yaml contains: feature_flags: ${http:https://config.example.com/flags.json}
    config = Config.load("config.yaml")
    # Error: HTTP resolver is disabled. Enable with allow_http=True
    ```

Now let's enable it properly:

=== "Python"

    ```python
    from holoconf import Config

    # Enable HTTP resolver
    config = Config.load("config.yaml", allow_http=True)

    # Now it works!
    flags = config.get("feature_flags")
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    let config = Config::builder()
        .allow_http(true)
        .load("config.yaml")?;
    ```

=== "CLI"

    ```bash
    # Must use --allow-http flag
    holoconf get feature_flags config.yaml --allow-http
    ```

### URL Allowlist

For additional security, restrict which URLs can be fetched:

=== "Python"

    ```python
    from holoconf import Config

    # Only allow specific URLs
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_allowlist=[
            "https://config.example.com/*",
            "https://*.internal.com/*"
        ]
    )
    # ✓ Can fetch: https://config.example.com/anything
    # ✓ Can fetch: https://api.internal.com/config
    # ✗ Blocked: https://evil.com/malicious
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    let config = Config::builder()
        .allow_http(true)
        .http_allowlist(vec!["https://config.example.com/*"])
        .load("config.yaml")?;
    ```

=== "CLI"

    ```bash
    holoconf dump config.yaml --resolve --allow-http \
      --http-allowlist 'https://config.example.com/*'
    ```

Allowlist patterns support glob-style wildcards:

| Pattern | Matches |
|---------|---------|
| `https://example.com/*` | Any path on example.com |
| `https://*.example.com/*` | Any subdomain of example.com |
| `https://api.example.com/config/*` | Specific path prefix |

### HTTP Resolver Options

```yaml
# Basic usage
feature_flags: ${http:https://config.example.com/flags.json}

# With fallback if request fails
remote_config: ${http:https://api.example.com/config,default={}}

# With authentication header
private_config: ${http:https://api.example.com/config,header=Authorization:Bearer token123}

# With timeout (in seconds)
slow_endpoint: ${http:https://slow.example.com/data,timeout=60}

# Parse as text instead of JSON/YAML
raw_data: ${http:https://api.example.com/data,parse=text}
```

All available options:

| Option | Description | Example |
|--------|-------------|---------|
| `parse=auto` | Auto-detect from Content-Type or URL (default) | `${http:https://example.com/config}` |
| `parse=yaml` | Parse response as YAML | `${http:https://example.com/config,parse=yaml}` |
| `parse=json` | Parse response as JSON | `${http:https://example.com/config,parse=json}` |
| `parse=text` | Return response as text | `${http:https://example.com/data,parse=text}` |
| `timeout=30` | Request timeout in seconds (default: 30) | `${http:https://example.com/config,timeout=60}` |
| `header=Name:Value` | Add custom HTTP header | `${http:https://api.com/config,header=Authorization:Bearer token}` |

### Proxy Configuration

If your network requires a proxy:

=== "Python"

    ```python
    from holoconf import Config

    # Explicit HTTP proxy
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_proxy="http://proxy.corp.com:8080"
    )

    # SOCKS proxy
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_proxy="socks5://proxy.corp.com:1080"
    )

    # Auto-detect from environment (HTTP_PROXY, HTTPS_PROXY)
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_proxy_from_env=True
    )
    ```

Per-request proxy override:

```yaml
value: ${http:https://api.example.com/config,proxy=http://proxy:8080}
```

### Custom CA Certificates

For internal or corporate CAs, or self-signed certificates in development:

=== "Python"

    ```python
    from holoconf import Config

    # Replace default CA bundle with custom bundle
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_ca_bundle="/etc/ssl/certs/internal-ca.pem"
    )

    # Add extra CA to default roots (doesn't replace them)
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_extra_ca_bundle="/etc/ssl/certs/extra-ca.pem"
    )
    ```

Per-request CA override:

```yaml
# Replace CA bundle for this request
internal: ${http:https://internal.corp/config,ca_bundle=/path/to/ca.pem}

# Add extra CA for this request
secure: ${http:https://api.example.com/config,extra_ca_bundle=/path/to/extra.pem}
```

### Mutual TLS (Client Certificates)

For services requiring client certificate authentication:

=== "Python"

    ```python
    from holoconf import Config

    # PEM certificate and key
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_client_cert="/path/to/client.pem",
        http_client_key="/path/to/client-key.pem"
    )

    # Encrypted private key
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_client_cert="/path/to/client.pem",
        http_client_key="/path/to/client-key-encrypted.pem",
        http_client_key_password="secret"
    )

    # P12/PFX bundle (includes both cert and key)
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_client_cert="/path/to/identity.p12",
        http_client_key_password="secret"
    )
    ```

Per-request client certificate:

```yaml
# PEM files
secure: ${http:https://secure.corp/config,client_cert=/path/cert.pem,client_key=/path/key.pem}

# With encrypted key
secure: ${http:https://secure.corp/config,client_cert=/path/cert.pem,client_key=/path/key.pem,key_password=secret}

# P12/PFX bundle
secure: ${http:https://secure.corp/config,client_cert=/path/identity.p12,key_password=secret}
```

### Disabling TLS Verification (Development Only)

!!! danger "CRITICAL SECURITY WARNING"
    Disabling TLS verification exposes you to man-in-the-middle attacks where an attacker can intercept and modify your configuration data.

    **NEVER use this in production.** Only use for local development with self-signed certificates.

    **As of v0.3.0**, the global `http_insecure=True` parameter has been removed for security. You can only disable verification per-request, and HoloConf will display a prominent warning.

Per-request insecure mode (shows obnoxious warning):

```yaml
# DANGEROUS: Skip TLS verification for this request
# This will print a warning to stderr every time it's resolved
dev_config: ${http:https://dev.local/config,insecure=true}
```

**Better alternative** - Use proper CA configuration instead:

=== "Python"

    ```python
    from holoconf import Config

    # RECOMMENDED: Add your development CA certificate
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_extra_ca_bundle="/path/to/dev-ca.pem"
    )
    ```

This approach:
- Maintains security (validates certificates)
- Works with your self-signed certs
- Doesn't trigger warnings
- Can be safely used in all environments

!!! warning "Migration from v0.2.x"
    If you were using `http_insecure=True` globally in v0.2.x:

    **Old (v0.2.x, removed):**
    ```python
    config = Config.load("config.yaml", http_insecure=True)  # ✗ No longer exists
    ```

    **New (v0.3.0+) - Per-request:**
    ```yaml
    # Shows warning on every access
    value: ${http:https://dev.local/config,insecure=true}
    ```

    **Better (v0.3.0+) - Use CA bundle:**
    ```python
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_extra_ca_bundle="/path/to/dev-ca.pem"  # ✓ Secure and quiet
    )
    ```

### HTTP Security Best Practices

1. **Keep HTTP disabled by default** - Only enable with `allow_http=True` when needed
2. **Use URL allowlist** - Restrict which URLs can be fetched
3. **Always use HTTPS** - Never use `http://` for sensitive configuration
4. **Set appropriate timeouts** - Prevent hanging on slow responses
5. **Use authentication** - Add authorization headers for private endpoints
6. **Prefer CA bundles over insecure mode** - For self-signed certs, add your CA rather than disabling verification
7. **Use per-request options sparingly** - Global configuration is usually safer and easier to audit

## AWS Resolvers

The `holoconf-aws` package provides resolvers for AWS services. Install separately:

```bash
pip install holoconf-aws
```

Resolvers are automatically registered when imported.

### SSM Parameter Store

Fetch values from AWS Systems Manager Parameter Store:

```yaml
database:
  host: ${ssm:/app/prod/db-host}
  password: ${ssm:/app/prod/db-password}
  # With region override
  config: ${ssm:/app/config,region=us-west-2}
  # With profile override
  secret: ${ssm:/app/secret,profile=production}
```

| Parameter Type | Behavior |
|---------------|----------|
| String | Returned as-is |
| SecureString | Automatically marked as sensitive |
| StringList | Returned as a Python list |

### CloudFormation Outputs

Fetch outputs from CloudFormation stacks:

```yaml
infrastructure:
  endpoint: ${cfn:my-database-stack/DatabaseEndpoint}
  bucket: ${cfn:my-storage-stack/BucketName}
  # With region/profile
  vpc_id: ${cfn:shared-infra/VpcId,region=us-west-2}
```

### S3 Objects

Fetch and parse objects from S3:

```yaml
# Auto-parsed based on file extension
shared_config: ${s3:my-bucket/configs/shared.yaml}
feature_flags: ${s3:my-bucket/settings/flags.json}

# Explicit parse mode
raw_text: ${s3:my-bucket/docs/README.md,parse=text}
binary_data: ${s3:my-bucket/certs/cert.pem,parse=binary}
```

## Custom Resolvers

You can create your own resolvers in Python to integrate with any data source.

### Simple Function Resolver

The easiest way is a function that takes a key and optional keyword arguments:

```python
import holoconf

def my_lookup(key, region=None, **kwargs):
    """Custom resolver that looks up values from an internal service."""
    result = internal_api.get(key, region=region)
    return result

holoconf.register_resolver("lookup", my_lookup)
```

Now use it in your config:

```yaml
database:
  host: ${lookup:db-host,region=us-east-1}
```

### Async Resolvers

Async functions work automatically:

```python
import holoconf
import aiohttp

async def fetch_secret(key, **kwargs):
    """Async resolver that fetches secrets from a remote service."""
    async with aiohttp.ClientSession() as session:
        async with session.get(f"https://secrets.example.com/{key}") as resp:
            return await resp.text()

holoconf.register_resolver("secret", fetch_secret)
```

### Returning Sensitive Values

For values that should be redacted in output:

```python
from holoconf import register_resolver, ResolvedValue

def vault_resolver(path, **kwargs):
    """Resolver for HashiCorp Vault secrets."""
    secret = vault_client.read(path)
    # Mark all Vault values as sensitive
    return ResolvedValue(secret["data"]["value"], sensitive=True)

register_resolver("vault", vault_resolver)
```

```yaml
api:
  key: ${vault:secret/data/api-key}  # Will show as [REDACTED]
```

### Error Handling

Raise `KeyError` to indicate a resource wasn't found (enables `default=` fallback):

```python
def my_resolver(key, **kwargs):
    result = cache.get(key)
    if result is None:
        raise KeyError(f"Key not found: {key}")
    return result

register_resolver("cache", my_resolver)
```

```yaml
# Uses default if key not found
value: ${cache:my-key,default=fallback}
```

## Lazy Resolution

Here's something important to understand: resolvers are invoked **lazily** - values are only resolved when you access them, not when the configuration is loaded.

This means:
- Environment variables are read when you call `config.get()`, not when you call `Config.load()`
- Files are read when you access the value
- HTTP requests are made when you access the value
- Default values are only resolved if the primary resolver fails

Why does this matter? It makes HoloConf faster and more flexible. If you never access a value, its resolver never runs.

!!! tip "Try It Yourself"
    Create a configuration with different resolvers:

    - Mix environment variables, file includes, and self-references
    - Try optional files with defaults
    - Build URLs using self-references
    - Set up an HTTP endpoint (if available) and fetch configuration from it

## What You've Learned

You now understand:

- How to use environment variables with defaults and sensitivity markers
- Absolute and relative self-references for reusing values
- File includes with security restrictions
- HTTP fetching with security controls
- AWS resolvers for SSM, CloudFormation, and S3
- Creating custom resolvers
- Lazy resolution behavior

## Next Steps

- **[Merging](merging.md)** - Combine multiple configuration files for environment-specific settings
- **[Validation](validation.md)** - Use JSON Schema to catch configuration errors
- **[ADR-002 Resolver Architecture](../adr/ADR-002-resolver-architecture.md)** - Design rationale for resolvers
- **[FEAT-002 Core Resolvers](../specs/features/FEAT-002-core-resolvers.md)** - Full resolver specification

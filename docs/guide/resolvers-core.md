# Core Resolvers

HoloConf ships with essential resolvers that cover the most common configuration needs. Let's explore each one and see how they solve real-world problems.

## Config References (`ref`)

The `ref` resolver lets you reference other values within your configuration. While you typically use the shorthand `${path}` syntax, you can also write it explicitly as `${ref:path}`.

### Basic Usage

```yaml
defaults:
  timeout: 30
  retries: 3

service_a:
  timeout: ${defaults.timeout}    # Shorthand
  retries: ${ref:defaults.retries}  # Explicit

service_b:
  timeout: ${defaults.timeout}
  retries: ${defaults.retries}
```

Both forms work identically - use whichever feels clearer in your configuration.

### Optional References with Defaults

The real power of `ref` comes from the `default=` parameter. This lets you gracefully handle optional configuration that might not exist:

```yaml
features:
  beta: true
  # No 'experimental' flag defined

app:
  # Returns false if features.experimental doesn't exist
  experimental_enabled: ${features.experimental,default=false}

  # Returns 30 if custom_timeout is missing or null
  timeout: ${config.custom_timeout,default=30}
```

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config.yaml")

    # No error - uses default since path doesn't exist
    enabled = config.app.experimental_enabled
    print(f"Experimental: {enabled}")
    # Experimental: false

    timeout = config.app.timeout
    print(f"Timeout: {timeout}")
    # Timeout: 30
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    let config = Config::load("config.yaml")?;

    let enabled: bool = config.get("app.experimental_enabled")?;
    println!("Experimental: {}", enabled);
    // Experimental: false

    let timeout: i64 = config.get("app.timeout")?;
    println!("Timeout: {}", timeout);
    // Timeout: 30
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml app.experimental_enabled
    false

    $ holoconf get config.yaml app.timeout
    30
    ```

### When Defaults Are Applied

The `default=` parameter is used when:

- **Path doesn't exist** in the configuration
- **Value is explicitly `null`**

If the value exists and is not null, the actual value is used (default is ignored).

### Defaults Can Be Interpolations

Your default value can itself reference other configuration:

```yaml
defaults:
  timeout: 30
  host: localhost

service_a:
  # Uses service_a.custom_host if defined, otherwise defaults.host
  host: ${service_a.custom_host,default=${defaults.host}}
  # Uses service_a.custom_timeout if defined, otherwise defaults.timeout
  timeout: ${service_a.custom_timeout,default=${defaults.timeout}}

service_b:
  custom_host: prod.example.com
  # service_b has custom host, falls back to default timeout
  host: ${service_b.custom_host,default=${defaults.host}}
  timeout: ${service_b.custom_timeout,default=${defaults.timeout}}
```

!!! tip "Use Cases for Optional References"
    The `default=` parameter is perfect for:

    - **Feature flags** that may not be defined in all environments
    - **Optional overrides** that only exist in specific deployments
    - **Environment-specific settings** with sensible defaults
    - **Graceful degradation** when configuration sections are optional

!!! warning "Required vs Optional Configuration"
    Don't use defaults for truly **required** configuration - let errors surface early! Only use `default=` when a missing value is genuinely acceptable.

### Marking References as Sensitive

The `ref` resolver supports the `sensitive=true` flag to mark values as sensitive:

```yaml
secrets:
  database_password: super-secret-password
  api_key: prod-api-key-12345

app:
  # Mark referenced secrets as sensitive (redacted in logs)
  db_pass: ${secrets.database_password,sensitive=true}
  api_key: ${secrets.api_key,sensitive=true}

  # Can combine with default
  backup_key: ${secrets.backup_key,default=dev-key,sensitive=true}
```

When marked as sensitive, values are redacted in log output and the `dump` command.

## Environment Variables

The most common use case for dynamic configuration is reading from environment variables. This lets you change settings between development, staging, and production without editing files.

Let's start with a simple example:

```yaml
database:
  host: ${env:DB_HOST}
  port: ${env:DB_PORT}
```

Now try to use it:

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config.yaml")
    host = config.database.host
    # Error: ResolverError: Environment variable DB_HOST is not set
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    let config = Config::load("config.yaml")?;
    let host: String = config.get("database.host")?;
    // Error: Environment variable DB_HOST is not set
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml database.host
    Error: Environment variable DB_HOST is not set
    ```

The error is helpful - it prevents us from accidentally using undefined values. But we want our configuration to work during development without setting every variable. Let's add defaults:

```yaml
database:
  host: ${env:DB_HOST,default=localhost}
  port: ${env:DB_PORT,default=5432}
```

Now it works both ways:

=== "Python"

    ```python
    import os
    from holoconf import Config

    # Without environment variables - uses defaults
    config = Config.load("config.yaml")
    host = config.database.host
    print(f"Host: {host}")
    # Host: localhost

    # With environment variables - uses env values
    os.environ["DB_HOST"] = "prod-db.example.com"
    config = Config.load("config.yaml")
    host = config.database.host
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
    # Without environment variable
    $ holoconf get config.yaml database.host
    localhost

    # With environment variable
    $ DB_HOST=prod-db.example.com holoconf get config.yaml database.host
    prod-db.example.com
    ```

Perfect! Now your configuration works in development (using defaults) and production (using environment variables).

## Self-References: Avoiding Duplication

As your configuration grows, you'll find yourself repeating values. Self-references let you define a value once and reuse it everywhere.

Let's say you have shared defaults:

```yaml
defaults:
  timeout: 30
  host: localhost

database:
  host: ${defaults.host}
  timeout: ${defaults.timeout}
  port: 5432

api:
  host: ${defaults.host}
  timeout: ${defaults.timeout}
  port: 8000
```

Now when you access these values, they resolve to the shared defaults:

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config.yaml")

    db_host = config.database.host
    api_host = config.api.host
    print(f"Database: {db_host}, API: {api_host}")
    # Database: localhost, API: localhost

    # Both reference the same value
    db_timeout = config.database.timeout
    api_timeout = config.api.timeout
    print(f"Timeouts: {db_timeout}, {api_timeout}")
    # Timeouts: 30, 30
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    let config = Config::load("config.yaml")?;

    let db_host: String = config.get("database.host")?;
    let api_host: String = config.get("api.host")?;
    println!("Database: {}, API: {}", db_host, api_host);
    // Database: localhost, API: localhost
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml database.host
    localhost

    $ holoconf get config.yaml api.host
    localhost
    ```

### Relative References

But what if you want to reference a value that's nearby in the configuration tree? Absolute paths like `${defaults.host}` work, but they're verbose. Use relative paths instead:

```yaml
database:
  host: localhost
  port: 5432
  # Reference sibling 'host' using dot prefix
  connection_string: "postgres://${.host}:${.port}/mydb"

services:
  database:
    host: prod-db.example.com

  api:
    # Reference parent's sibling using double-dot
    db_host: ${..database.host}
```

Let's see what these resolve to:

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config.yaml")

    # Sibling reference
    conn_str = config.database.connection_string
    print(f"Connection: {conn_str}")
    # Connection: postgres://localhost:5432/mydb

    # Parent's sibling reference
    api_db = config.services.api.db_host
    print(f"API uses DB: {api_db}")
    # API uses DB: prod-db.example.com
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    let config = Config::load("config.yaml")?;

    let conn_str: String = config.get("database.connection_string")?;
    println!("Connection: {}", conn_str);
    // Connection: postgres://localhost:5432/mydb
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml database.connection_string
    postgres://localhost:5432/mydb

    $ holoconf get config.yaml services.api.db_host
    prod-db.example.com
    ```

!!! tip "When to Use Relative vs Absolute References"
    - Use `${.sibling}` for values in the same section (like building connection strings)
    - Use `${..parent.sibling}` for nearby values (like services referencing shared settings)
    - Use `${absolute.path}` for shared defaults used across the entire config

### Preventing Circular References

What happens if you create a circular reference?

```yaml
a:
  value: ${b.value}

b:
  value: ${a.value}
```

HoloConf detects this and gives you a clear error:

=== "Python"

    ```python
    from holoconf import Config, CircularReferenceError

    config = Config.load("config.yaml")

    try:
        value = config.a.value
    except CircularReferenceError as e:
        print(f"Error: {e}")
        # Error: Circular reference detected: a.value -> b.value -> a.value
    ```

=== "Rust"

    ```rust
    use holoconf::{Config, Error};

    let config = Config::load("config.yaml")?;
    match config.get::<String>("a.value") {
        Err(Error::CircularReference { path, .. }) => {
            println!("Circular reference at: {}", path);
        }
        _ => {}
    }
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml a.value
    Error: Circular reference detected: a.value -> b.value -> a.value
    ```

The error shows you the exact reference chain, making it easy to fix.

## File Includes: Splitting Large Configurations

Sometimes configuration is too large for a single value. Maybe you have a multi-line certificate, a JSON blob, or a YAML snippet. The `file` resolver lets you include content from external files.

Let's say you have a private key in a separate file:

```
config/
├── app.yaml
└── secrets/
    └── private-key.pem
```

```yaml
# app.yaml
ssl:
  certificate: ${file:secrets/cert.pem}
  private_key: ${file:secrets/private-key.pem}
```

When you access these values, the file content is included:

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config/app.yaml")

    # Returns the file content as a string
    private_key = config.ssl.private_key
    print(f"Key length: {len(private_key)} bytes")
    # Key length: 1704 bytes
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    let config = Config::load("config/app.yaml")?;
    let private_key: String = config.get("ssl.private_key")?;
    println!("Key length: {} bytes", private_key.len());
    ```

=== "CLI"

    ```bash
    $ holoconf get config/app.yaml ssl.private_key
    -----BEGIN PRIVATE KEY-----
    MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC...
    ...
    ```

### Including Structured Data

But what if the file contains YAML or JSON? HoloConf automatically parses it:

```yaml
# users.yaml (separate file)
- name: alice
  role: admin
- name: bob
  role: user
```

```yaml
# app.yaml
users: ${file:users.yaml}
admin_name: ${file:users.yaml[0].name}
```

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("app.yaml")

    # Returns parsed YAML as a list
    users = config.users
    print(f"Users: {users}")
    # Users: [{'name': 'alice', 'role': 'admin'}, {'name': 'bob', 'role': 'user'}]

    # Can reference into the structure
    admin = config.admin_name
    print(f"Admin: {admin}")
    # Admin: alice
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    let config = Config::load("app.yaml")?;
    let admin: String = config.get("admin_name")?;
    println!("Admin: {}", admin);
    // Admin: alice
    ```

=== "CLI"

    ```bash
    $ holoconf get app.yaml admin_name
    alice
    ```

### Relative File Paths

File paths are relative to the configuration file, not your current directory:

```
project/
├── config/
│   ├── app.yaml
│   └── database.yaml
└── secrets/
    └── db-password.txt
```

```yaml
# config/app.yaml
database: ${file:database.yaml}
password: ${file:../secrets/db-password.txt}
```

The paths `database.yaml` and `../secrets/db-password.txt` are resolved relative to `config/app.yaml`, not where you run your program.

### RFC 8089 File URI Syntax

HoloConf supports RFC 8089 file: URI syntax for explicit absolute paths. This is useful when you want to be explicit about absolute paths:

```yaml
# RFC 8089 file: URIs (all equivalent for absolute paths)
system_config: ${file:///etc/myapp/config.yaml}
explicit_local: ${file://localhost/etc/myapp/config.yaml}
minimal_form: ${file:/etc/myapp/config.yaml}
```

All three forms reference the same absolute path `/etc/myapp/config.yaml`. The resolver normalizes them to the appropriate path format.

**Remote file URIs are rejected:**

```yaml
# This will error - remote file URIs not supported
remote_file: ${file://server.example.com/path/to/file}
```

HoloConf only supports local file access for security reasons. Remote file URIs (with a non-localhost hostname) are rejected with a clear error message.

!!! tip "When to Use RFC 8089 Syntax"
    Use plain paths for most cases: `${file:./config.yaml}` or `${file:/etc/app/config.yaml}`.

    Use RFC 8089 syntax when:
    - Interoperating with systems that use file: URIs
    - You want to be explicit about the path being absolute
    - Your configuration documents the URI format for clarity

### Handling Missing Files

What if the file doesn't exist?

=== "Python"

    ```python
    from holoconf import Config, ResolverError

    # config.yaml contains: cert: ${file:missing.pem}
    config = Config.load("config.yaml")

    try:
        cert = config.cert
    except ResolverError as e:
        print(f"Error: {e}")
        # Error: File not found: missing.pem
    ```

=== "Rust"

    ```rust
    let config = Config::load("config.yaml")?;
    match config.get::<String>("cert") {
        Err(Error::ResolverError { message, .. }) => {
            println!("Error: {}", message);
        }
        _ => {}
    }
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml cert
    Error: File not found: missing.pem
    ```

You can provide a default instead:

```yaml
cert: ${file:cert.pem,default=selfsigned-cert-content}
```

## HTTP/HTTPS: Remote Configuration

For centralized configuration management, you can fetch values from HTTP endpoints. This is useful for:

- Fetching config from a configuration server
- Loading shared defaults from a central location
- Integrating with REST APIs

HoloConf provides separate `http` and `https` resolvers that auto-prepend the appropriate protocol, making your configs cleaner:

```yaml
# Clean syntax - resolver auto-prepends https://
feature_flags: ${https:config.example.com/flags.json}

# Also works with http
api_config: ${http:api.internal/config.json}
```

But wait - HTTP fetching is disabled by default for security. You need to explicitly enable it:

=== "Python"

    ```python
    from holoconf import Config

    # This will error
    config = Config.load("config.yaml")
    flags = config.feature_flags
    # Error: HTTP resolvers are disabled. Pass allow_http=True to enable.
    ```

Let's enable HTTP fetching:

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config.yaml", allow_http=True)

    # Now it works - fetches from the URL
    flags = config.feature_flags
    print(f"Flags: {flags}")
    # Flags: {'feature_a': True, 'feature_b': False}
    ```

=== "Rust"

    ```rust
    use holoconf::{Config, ConfigOptions};

    let options = ConfigOptions::default().allow_http(true);
    let config = Config::load_with_options("config.yaml", options)?;

    let flags: serde_json::Value = config.get("feature_flags")?;
    println!("Flags: {:?}", flags);
    ```

=== "CLI"

    ```bash
    # Enable HTTP with --allow-http flag
    $ holoconf get config.yaml feature_flags --allow-http
    feature_a: true
    feature_b: false
    ```

!!! warning "Security: HTTP Disabled by Default"
    HTTP fetching is disabled by default because:

    - It can leak sensitive configuration paths to network logs
    - It introduces network dependencies during config loading
    - It may expose your infrastructure to SSRF attacks

    Only enable it when you specifically need remote configuration.

### Handling Network Errors

What happens if the HTTP request fails?

```yaml
data: ${https:api.example.com/config}
```

=== "Python"

    ```python
    from holoconf import Config, ResolverError

    config = Config.load("config.yaml", allow_http=True)

    try:
        data = config.data
    except ResolverError as e:
        print(f"Error: {e}")
        # Error: HTTP request failed: connection timeout
    ```

=== "Rust"

    ```rust
    match config.get::<String>("data") {
        Err(Error::ResolverError { message, .. }) => {
            println!("HTTP error: {}", message);
        }
        _ => {}
    }
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml data --allow-http
    Error: HTTP request failed: connection timeout
    ```

You can provide a fallback:

```yaml
data: ${https:api.example.com/config,default={}}
```

Now if the HTTP request fails, it uses the empty object instead of erroring.

### Custom Request Timeouts

The default timeout is 30 seconds. For slower endpoints, increase it:

=== "Python"

    ```python
    from holoconf import Config

    # Set HTTP timeout to 60 seconds
    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_timeout=60
    )

    data = config.data  # Waits up to 60 seconds
    ```

=== "Rust"

    ```rust
    use holoconf::{Config, ConfigOptions};
    use std::time::Duration;

    let options = ConfigOptions::default()
        .allow_http(true)
        .http_timeout(Duration::from_secs(60));

    let config = Config::load_with_options("config.yaml", options)?;
    ```

=== "CLI"

    ```bash
    # Set timeout to 60 seconds
    $ holoconf get config.yaml data --allow-http --http-timeout 60
    ```

### Working with Internal Certificate Authorities

Many organizations use internal certificate authorities for HTTPS services. Let's see how to configure HoloConf to trust these.

First, let's try fetching from an internal service:

```yaml
config:
  data: ${https:internal.corp.com/config.json}
```

=== "Python"

    ```python
    config = Config.load("config.yaml", allow_http=True)
    data = config.data
    # Error: SSL certificate verification failed
    ```

The error occurs because your organization's CA isn't in the default trust store. We can fix this by providing your CA certificate:

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_ca_bundle="/etc/ssl/certs/internal-ca.pem"
    )

    data = config.data  # Now it works!
    ```

=== "Rust"

    ```rust
    use holoconf::{Config, ConfigOptions};

    let options = ConfigOptions::default()
        .allow_http(true)
        .http_ca_bundle("/etc/ssl/certs/internal-ca.pem");

    let config = Config::load_with_options("config.yaml", options)?;
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml data --allow-http --http-ca-bundle /etc/ssl/certs/internal-ca.pem
    ```

This **replaces** the default CA bundle with your custom one. But what if you need to trust BOTH public CAs (for external APIs) AND your internal CA? Use `http_extra_ca_bundle` instead:

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_extra_ca_bundle="/etc/ssl/certs/internal-ca.pem"  # Added to defaults
    )

    data = config.data  # Now trusts both public and internal CAs
    ```

=== "Rust"

    ```rust
    let options = ConfigOptions::default()
        .allow_http(true)
        .http_extra_ca_bundle("/etc/ssl/certs/internal-ca.pem");

    let config = Config::load_with_options("config.yaml", options)?;
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml data --allow-http --http-extra-ca-bundle /etc/ssl/certs/internal-ca.pem
    ```

This **adds** your CA to the existing trust store, so you can fetch from both internal and external HTTPS endpoints.

### Using Certificate Variables

Instead of storing certificates as files, you can load them from environment variables or other resolvers. This is useful for:

- Containerized environments where secrets are injected as environment variables
- Secret management systems that provide certificates dynamically
- CI/CD pipelines where certificates are stored in secure vaults

#### PEM Certificates from Environment Variables

Let's load client certificates for mTLS from environment variables:

```yaml
# config.yaml
secure_api:
  data: ${https:api.corp.com/config,client_cert=${env:CLIENT_CERT_PEM},client_key=${env:CLIENT_KEY_PEM}}
```

=== "Python"

    ```python
    import os
    from holoconf import Config

    # Set certificates in environment (in practice, these come from your secret management system)
    os.environ["CLIENT_CERT_PEM"] = """-----BEGIN CERTIFICATE-----
    MIICijCCAXICCQC1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789
    ...
    -----END CERTIFICATE-----"""

    os.environ["CLIENT_KEY_PEM"] = """-----BEGIN PRIVATE KEY-----
    MIICdgIBADANBgkqhkiG9w0BAQEFAASCAmAwggJcAgEAAoGBAK1234567890
    ...
    -----END PRIVATE KEY-----"""

    config = Config.load("config.yaml", allow_http=True)
    data = config.secure_api.data  # Uses certificates from environment
    ```

=== "Rust"

    ```rust
    use holoconf::Config;
    use std::env;

    // Set certificates in environment
    env::set_var("CLIENT_CERT_PEM", "-----BEGIN CERTIFICATE-----\n...");
    env::set_var("CLIENT_KEY_PEM", "-----BEGIN PRIVATE KEY-----\n...");

    let config = Config::load("config.yaml")?;
    let data = config.get("secure_api.data")?;
    ```

#### CA Bundle from File Resolver

You can also load CA bundles using the file resolver with `parse=text`:

```yaml
# config.yaml
internal_api:
  data: ${https:internal.corp.com/config,ca_bundle=${file:./certs/ca-bundle.pem,parse=text}}
```

This reads the CA bundle file and passes its PEM content directly to the HTTPS resolver.

#### P12/PFX Binary Certificates (Python)

For P12/PFX certificates (which contain both certificate and key), use `parse=binary`:

```yaml
# config.yaml
secure_data:
  value: ${https:secure.example.com/data,client_cert=${file:./certs/identity.p12,parse=binary},key_password=${env:P12_PASSWORD}}
```

=== "Python"

    ```python
    import os
    from holoconf import Config

    os.environ["P12_PASSWORD"] = "secret"

    config = Config.load("config.yaml", allow_http=True)
    value = config.secure_data.value  # Uses P12 certificate
    ```

**Auto-Detection**: HoloConf automatically detects whether you're providing:
- A **file path** (string without `-----BEGIN` marker)
- **PEM content** (string containing `-----BEGIN CERTIFICATE-----` or `-----BEGIN PRIVATE KEY-----`)
- **P12 binary** (bytes type in Python, detected by `.p12`/`.pfx` extension for paths)

This means you can mix and match:

```yaml
# Mixed mode: cert from environment, key from file path
api_config: ${https:api.example.com/config,client_cert=${env:CERT_PEM},client_key=/etc/ssl/private/key.pem}
```

### Authentication Headers

Some configuration endpoints require authentication. Add custom headers:

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load(
        "config.yaml",
        allow_http=True,
        http_headers={
            "Authorization": "Bearer YOUR_TOKEN_HERE",
            "X-Custom-Header": "value"
        }
    )

    data = config.data  # Includes auth headers in request
    ```

=== "Rust"

    ```rust
    use holoconf::{Config, ConfigOptions};
    use std::collections::HashMap;

    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer YOUR_TOKEN_HERE".to_string());

    let options = ConfigOptions::default()
        .allow_http(true)
        .http_headers(headers);

    let config = Config::load_with_options("config.yaml", options)?;
    ```

=== "CLI"

    ```bash
    # CLI doesn't support custom headers - use environment variables in the URL instead
    $ holoconf get config.yaml data --allow-http
    ```

!!! tip "Caching HTTP Responses"
    HTTP values are fetched every time you access them. For frequently accessed values:

    ```python
    # Fetch once and cache
    flags = config.feature_flags

    # Use cached value
    if flags['feature_a']:
        # ...
    ```

    This avoids repeated network requests.

## Transformation Resolvers: Parsing Structured Data

Sometimes you need to parse structured text into usable data - JSON from an API, YAML from a file, CSV data, or base64-encoded secrets. Transformation resolvers handle this by transforming string data into structured types.

### json - Parse JSON Data

Parse JSON strings into dictionaries, arrays, or other types:

```yaml
# Parse JSON from environment variable
api_config: ${json:${env:API_CONFIG}}

# Parse JSON from file
features: ${json:${file:features.json}}

# Parse JSON from HTTP endpoint
remote_data: ${json:${http:api.example.com/data}}
```

=== "Python"

    ```python
    from holoconf import Config
    import os

    # Set JSON in environment
    os.environ['API_CONFIG'] = '{"timeout": 30, "retries": 3}'

    config = Config.loads("""
    api: ${json:${env:API_CONFIG}}
    """)

    # Access as structured data
    print(config.api.timeout)  # 30
    print(config.api.retries)  # 3
    ```

=== "Rust"

    ```rust
    use holoconf::Config;
    use std::env;

    env::set_var("API_CONFIG", r#"{"timeout": 30, "retries": 3}"#);

    let config = Config::from_yaml(r#"
    api: ${json:${env:API_CONFIG}}
    "#)?;

    let timeout: i64 = config.get("api.timeout")?;
    println!("Timeout: {}", timeout);  // 30
    ```

The json resolver preserves native types - numbers stay as numbers, booleans stay as booleans.

### yaml - Parse YAML Data

Parse YAML strings into structured data:

```yaml
# Parse YAML from file
database: ${yaml:${file:database.yaml}}

# Parse YAML from HTTP
settings: ${yaml:${https:config.internal/app.yaml}}
```

=== "Python"

    ```python
    # database.yaml contains:
    # host: localhost
    # port: 5432
    # credentials:
    #   username: admin

    config = Config.load("config.yaml")

    print(config.database.host)  # localhost
    print(config.database.credentials.username)  # admin
    ```

=== "Rust"

    ```rust
    let config = Config::load("config.yaml")?;
    let host: String = config.get("database.host")?;
    println!("Host: {}", host);  // localhost
    ```

### split - Split Strings into Arrays

Split comma-separated (or custom delimiter) strings into arrays:

```yaml
# Default comma delimiter
tags: ${split:${env:FEATURE_TAGS}}

# Custom delimiter
path_dirs: ${split:${env:PATH},delim=:}

# With trim to remove whitespace
emails: ${split:${env:ADMIN_EMAILS},trim=true}

# Limit number of splits
parts: ${split:${env:DATA},limit=3}
```

=== "Python"

    ```python
    import os

    os.environ['FEATURE_TAGS'] = 'auth,api,logging'
    os.environ['PATH'] = '/usr/bin:/usr/local/bin:/home/user/bin'

    config = Config.loads("""
    tags: ${split:${env:FEATURE_TAGS}}
    paths: ${split:${env:PATH},delim=:}
    """)

    print(config.tags)  # ['auth', 'api', 'logging']
    print(config.paths[0])  # '/usr/bin'
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    env::set_var("FEATURE_TAGS", "auth,api,logging");

    let config = Config::from_yaml(r#"
    tags: ${split:${env:FEATURE_TAGS}}
    "#)?;

    let tags: Vec<String> = config.get("tags")?;
    println!("First tag: {}", tags[0]);  // auth
    ```

Options:

- `delim=,` - Delimiter character (default: `,`)
- `trim=true` - Remove whitespace from each item
- `limit=N` - Maximum number of items (remaining text goes in last item)

### csv - Parse CSV Data

Parse CSV text into arrays of objects (with headers) or arrays of arrays (without headers):

```yaml
# Parse CSV with headers (default)
users: ${csv:${file:users.csv}}

# Access by column name
first_user_email: ${users[0].email}

# Parse CSV without headers (array of arrays)
raw_data: ${csv:${file:data.csv},header=false}

# Custom delimiter (TSV)
records: ${csv:${file:data.tsv},delim=\t}
```

=== "Python"

    ```python
    # users.csv contains:
    # name,email,role
    # Alice,alice@example.com,admin
    # Bob,bob@example.com,user

    config = Config.loads("""
    users: ${csv:${file:users.csv}}
    """)

    print(config.users[0].name)   # Alice
    print(config.users[0].email)  # alice@example.com
    print(config.users[1].role)   # user
    ```

=== "Rust"

    ```rust
    let config = Config::load("config.yaml")?;
    let first_user: String = config.get("users[0].name")?;
    println!("First user: {}", first_user);  // Alice
    ```

Options:

- `header=true` - First row contains headers (default: `true`)
    - With headers: Returns array of objects
    - Without headers: Returns array of arrays
- `delim=,` - Field delimiter (default: `,`)
- `trim=true` - Trim whitespace from values

!!! note "CSV Values are Strings"
    All CSV values are returned as strings. Use schema validation to coerce types:

    ```yaml
    # users.csv: name,age
    # Alice,30
    users: ${csv:${file:users.csv}}
    ```

    ```python
    # Define schema to coerce age to integer
    schema = {
        "type": "object",
        "properties": {
            "users": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "age": {"type": "integer"}
                    }
                }
            }
        }
    }
    ```

### base64 - Decode Base64 Data

Decode base64-encoded strings:

```yaml
# Decode base64-encoded secret
api_key: ${base64:${env:API_KEY_B64}}

# Decode from file
certificate: ${base64:${file:cert.b64}}

# Chain with other transformations
# Base64-encoded JSON
config_data: ${json:${base64:${env:ENCODED_CONFIG}}}
```

=== "Python"

    ```python
    import os
    import base64

    # Encode a secret
    secret = "my-secret-api-key"
    encoded = base64.b64encode(secret.encode()).decode()
    os.environ['API_KEY_B64'] = encoded

    config = Config.loads("""
    api_key: ${base64:${env:API_KEY_B64}}
    """)

    print(config.api_key)  # my-secret-api-key
    ```

=== "Rust"

    ```rust
    use holoconf::Config;

    env::set_var("API_KEY_B64", "bXktc2VjcmV0LWFwaS1rZXk=");

    let config = Config::from_yaml(r#"
    api_key: ${base64:${env:API_KEY_B64}}
    "#)?;

    let key: String = config.get("api_key")?;
    println!("API Key: {}", key);  // my-secret-api-key
    ```

!!! tip "UTF-8 vs Binary"
    The base64 resolver automatically detects UTF-8 text and returns it as a string. Binary data (images, certificates) is returned as bytes.

### Chaining Transformations

You can chain multiple resolvers together by nesting them:

```yaml
# Common patterns
encoded_json: ${json:${base64:${env:ENCODED_CONFIG}}}
remote_csv: ${csv:${http:data.example.com/export.csv}}
split_from_file: ${split:${file:tags.txt}}

# Complex chain: fetch, decode, parse
api_data: ${json:${base64:${https:api.example.com/encrypted}}}
```

The resolvers execute from inside-out:

1. `${https:...}` fetches the data
2. `${base64:...}` decodes it
3. `${json:...}` parses the JSON

=== "Python"

    ```python
    import os
    import base64
    import json

    # Simulate encoded JSON config
    data = {"database": {"host": "prod-db", "port": 5432}}
    encoded = base64.b64encode(json.dumps(data).encode()).decode()
    os.environ['ENCODED_CONFIG'] = encoded

    config = Config.loads("""
    settings: ${json:${base64:${env:ENCODED_CONFIG}}}
    """)

    print(config.settings.database.host)  # prod-db
    print(config.settings.database.port)  # 5432
    ```

## Archive Extraction: Working with Compressed Files

The `extract` resolver allows you to extract specific files from ZIP, TAR, and TAR.GZ archives. This is useful for:

- Loading configuration from archived releases
- Extracting certificates or keys from secure bundles
- Processing backup archives
- Working with distributed configuration packages

### extract - Extract Files from Archives

=== "YAML"
    ```yaml
    # Extract a JSON config file from a ZIP archive
    release_config: ${json:${extract:${file:release-v1.0.0.zip,encoding=binary},path=config.json}}

    # Extract from TAR.GZ archive
    backup_data: ${yaml:${extract:${file:backup.tar.gz,encoding=binary},path=data/settings.yaml}}

    # Extract certificate from archive
    ca_cert: ${extract:${file:certificates.zip,encoding=binary},path=ca.pem}
    ```

=== "Python"
    ```python
    from holoconf import Config

    # Load config with archive extraction
    config = Config.load("config.yaml")

    # Access extracted and parsed JSON
    release_config = config.release_config
    print(f"App version: {release_config['version']}")

    # Access extracted YAML data
    backup_data = config.backup_data

    # Access raw extracted bytes (certificate)
    ca_cert_bytes = config.ca_cert
    ```

=== "Rust"
    ```rust
    use holoconf::Config;

    let config = Config::load("config.yaml")?;

    // Extracted JSON is automatically parsed
    let version: String = config.get("release_config.version")?;
    println!("App version: {}", version);

    // Access extracted YAML data
    let settings: serde_json::Value = config.get("backup_data")?;

    // Access raw bytes (certificate)
    let ca_cert: Vec<u8> = config.get("ca_cert")?;
    ```

=== "CLI"
    ```bash
    # Get value from extracted JSON
    $ holoconf get config.yaml release_config.version
    1.0.0

    # Dump extracted YAML
    $ holoconf get config.yaml backup_data --format json
    {"host": "localhost", "port": 5432}

    # Extract raw bytes (base64 encoded in output)
    $ holoconf get config.yaml ca_cert
    LS0tLS1CRUdJTiBDRVJUSUZJQ0FURS0tLS0t...
    ```

**Key Points:**

- The archive data must be provided as bytes using `encoding=binary`
- Specify the file to extract using the `path` kwarg
- Supports ZIP, TAR, and TAR.GZ formats (auto-detected)
- Returns extracted file contents as bytes
- Chain with transformation resolvers to parse extracted data
- Size limits: 10MB per file (protects against zip bombs)

### Extracting from Local Archives

```yaml
# Extract and parse JSON configuration
app_config:
  archive_path: ./releases/v1.0.0.zip
  config: ${json:${extract:${file:${.archive_path},encoding=binary},path=config.json}}
  readme: ${extract:${file:${.archive_path},encoding=binary},path=README.txt}

# Extract from TAR archive
backup:
  archive: ./backup-2024-01-20.tar
  database_config: ${yaml:${extract:${file:${.archive},encoding=binary},path=config/database.yaml}}
  app_settings: ${json:${extract:${file:${.archive},encoding=binary},path=config/app.json}}
```

### Extracting from Remote Archives

You can combine the `extract` resolver with `http` or `https` to process remote archives:

```yaml
# Download and extract from remote ZIP
remote_release:
  url: releases.example.com/myapp/v2.1.0.zip
  config: ${json:${extract:${https:${.url},parse=binary},path=config.json}}
  version: ${extract:${https:${.url},parse=binary},path=VERSION.txt}

# Extract from S3 (requires holoconf-aws)
s3_backup:
  bucket: my-backups
  key: backups/2024-01-20.tar.gz
  data: ${csv:${extract:${s3:${.bucket}/${.key},encoding=binary},path=export.csv}}
```

### Password-Protected Archives

ZIP archives can be password-protected. Use the `password` kwarg to provide the password:

```yaml
# Extract from encrypted ZIP
secure_archive:
  archive_password: ${env:ARCHIVE_PASSWORD,sensitive=true}

  # Extract encrypted file
  private_key: ${extract:${file:secure.zip,encoding=binary},path=private.key,password=${.archive_password}}

  # Some files in the ZIP might not be encrypted
  readme: ${extract:${file:secure.zip,encoding=binary},path=README.txt}
```

!!! warning "Security: ZIP Password Encryption"
    **ZipCrypto (Legacy Encryption):** Older password-protected ZIPs use ZipCrypto encryption, which is cryptographically weak:

    - **Vulnerability:** Passwords can be cracked in seconds using known-plaintext attacks
    - **Tools:** `bkcrack`, `pkcrack` can recover passwords from encrypted ZIPs
    - **Not recommended:** Do not rely on ZIP passwords for strong security

    **AES Encryption (Secure):** Newer ZIPs may use AES-256 encryption (created with 7-Zip, WinZip):

    - **Secure:** Industry-standard AES encryption
    - **Supported:** holoconf can extract AES-encrypted ZIPs
    - **Recommended:** Use AES-encrypted ZIPs for new archives

    **Better alternatives for sensitive data:**

    - **GPG:** Encrypt files before archiving: `gpg -c sensitive.tar.gz`
    - **Age:** Modern encryption: `age -e -o secrets.age < config.json`
    - **AWS KMS:** Cloud-based encryption for AWS environments

    **Creating AES-encrypted ZIPs:**
    ```bash
    # Using 7-Zip (AES-256)
    7z a -p -mem=AES256 secure.zip config.json

    # Using WinZip (AES-256) - GUI only
    # Select "AES Encryption" when creating archive
    ```

### Extracting Nested Paths

Archives can contain nested directory structures. Use the full path to the file:

```yaml
# Extract from nested structure
release:
  archive: release.tar.gz

  # Extract file from nested directory
  api_config: ${json:${extract:${file:${.archive},encoding=binary},path=configs/api/settings.json}}

  # Extract from deeply nested path
  db_schema: ${extract:${file:${.archive},encoding=binary},path=migrations/v1/schema.sql}
```

### Combining with Other Transformation Resolvers

The `extract` resolver returns bytes, which you can chain with any transformation resolver:

```yaml
extracted_data:
  # Extract and parse JSON
  json_config: ${json:${extract:${file:data.zip,encoding=binary},path=config.json}}

  # Extract and parse YAML
  yaml_settings: ${yaml:${extract:${file:data.tar.gz,encoding=binary},path=settings.yaml}}

  # Extract and parse CSV
  csv_data: ${csv:${extract:${file:exports.zip,encoding=binary},path=data.csv}}

  # Extract and base64 encode (useful for embedding binary data)
  cert_b64: ${base64:${extract:${file:certs.zip,encoding=binary},path=ca.pem}}

  # Extract and split (for line-delimited files)
  host_list: ${split:${extract:${file:config.tar,encoding=binary},path=hosts.txt},delim=\n}
```

### Error Handling

```yaml
# Handle missing files in archive with defaults
optional_config: ${json:${extract:${file:release.zip,encoding=binary},path=optional.json},default={}}

# The extract resolver itself doesn't support default, but you can wrap it:
safe_extract:
  # This will fail if the file doesn't exist in the archive
  # extracted: ${extract:${file:archive.zip,encoding=binary},path=missing.txt}

  # Instead, wrap the entire chain with a default
  config: ${json:${extract:${file:archive.zip,encoding=binary},path=config.json},default={"mode":"default"}}
```

### Supported Archive Formats

| Format | Extension | Compression | Notes |
|--------|-----------|-------------|-------|
| ZIP | `.zip` | Deflate | Supports password protection (ZipCrypto) |
| TAR | `.tar` | None | Uncompressed tape archive |
| TAR+GZIP | `.tar.gz`, `.tgz` | GZIP | Compressed tape archive |

Format detection is automatic based on file magic bytes (not extension).

### Feature Requirement

The `extract` resolver requires the `archive` feature to be enabled. This is included by default in the Python package but optional in the Rust crate:

```toml
# For Rust projects
[dependencies]
holoconf-core = { version = "0.4", features = ["archive"] }
```

## Quick Reference

Here's a summary of all core resolvers:

| Resolver | Syntax | Description | Example |
|----------|--------|-------------|---------|
| `env` | `${env:VAR}` | Environment variable | `${env:DB_HOST,default=localhost}` |
| Self-reference | `${path}` | Absolute reference | `${defaults.timeout}` |
| Self-reference | `${.key}` | Sibling reference | `${.host}` |
| Self-reference | `${..parent.key}` | Parent's sibling | `${..shared.timeout}` |
| `file` | `${file:path}` | File content or RFC 8089 URI | `${file:secrets/key.pem}` |
| `http` | `${http:url}` | HTTP GET (auto-prepends http://) | `${http:api.example.com/config}` |
| `https` | `${https:url}` | HTTPS GET (auto-prepends https://) | `${https:api.example.com/config}` |
| `json` | `${json:text}` | Parse JSON string | `${json:${env:CONFIG}}` |
| `yaml` | `${yaml:text}` | Parse YAML string | `${yaml:${file:config.yaml}}` |
| `split` | `${split:text}` | Split string into array | `${split:${env:TAGS}}` |
| `csv` | `${csv:text}` | Parse CSV into array | `${csv:${file:data.csv}}` |
| `base64` | `${base64:text}` | Decode base64 | `${base64:${env:SECRET_B64}}` |
| `extract` | `${extract:data,path=file}` | Extract file from archive | `${extract:${file:data.zip,encoding=binary},path=config.json}` |

All resolvers support:

- `default=value` - Fallback if resolver fails
- `sensitive=true` - Mark value for redaction

## What You've Learned

You now understand:

- Using `${env:VAR}` to read environment variables
- Providing defaults with `default=value`
- Referencing other config values with absolute and relative paths
- Preventing circular references
- Including file content with `${file:path}`
- Fetching remote config with `${http:url}` and `${https:url}`
- Configuring HTTP timeouts, CA bundles, and auth headers
- Parsing structured data with transformation resolvers (`json`, `yaml`, `split`, `csv`, `base64`)
- Extracting files from archives with the `extract` resolver
- Chaining resolvers for complex transformations
- Security implications of HTTP resolvers

## Next Steps

- **[AWS Resolvers](resolvers-aws.md)** - Integrate with AWS SSM, CloudFormation, and S3
- **[Custom Resolvers](resolvers-custom.md)** - Write your own resolvers for custom data sources

## See Also

- [ADR-002 Resolver Architecture](../adr/ADR-002-resolver-architecture.md) - Technical design
- [FEAT-002 Core Resolvers](../specs/features/FEAT-002-core-resolvers.md) - Full specification

# Custom Resolvers

HoloConf's built-in resolvers cover common use cases, but sometimes you need to integrate with custom data sources: a company-internal secrets manager, a proprietary configuration service, or a specialized API. Custom resolvers let you extend HoloConf to work with any data source.

## Why Custom Resolvers?

Let's say your organization uses HashiCorp Vault for secrets management. You want to write:

```yaml
database:
  password: ${vault:secret/data/database/password}
```

HoloConf doesn't have a built-in Vault resolver, but you can write one yourself in just a few lines of code.

## Your First Custom Resolver

Let's start with the simplest possible resolver - one that looks up values in a dictionary. This helps you understand the basics before tackling more complex integrations.

Create a resolver function:

=== "Python"

    ```python
    import holoconf

    # Our "database" of values
    LOOKUP_TABLE = {
        "db_host": "prod-db.example.com",
        "db_port": "5432",
        "api_url": "https://api.example.com"
    }

    # Resolver function - just looks up the key
    def lookup_resolver(key: str) -> str:
        if key not in LOOKUP_TABLE:
            raise ValueError(f"Key not found: {key}")
        return LOOKUP_TABLE[key]

    # Register the resolver
    holoconf.register("lookup", lookup_resolver)
    ```

=== "Rust"

    ```rust
    use holoconf_core::{Config, Resolver, ResolverResult};
    use std::collections::HashMap;

    struct LookupResolver {
        data: HashMap<String, String>,
    }

    impl Resolver for LookupResolver {
        fn resolve(&self, key: &str) -> ResolverResult<String> {
            self.data.get(key)
                .cloned()
                .ok_or_else(|| format!("Key not found: {}", key).into())
        }
    }

    // Register the resolver
    let mut data = HashMap::new();
    data.insert("db_host".to_string(), "prod-db.example.com".to_string());
    data.insert("db_port".to_string(), "5432".to_string());

    let resolver = LookupResolver { data };
    holoconf_core::register_resolver("lookup", Box::new(resolver));
    ```

Now you can use it in your configuration:

```yaml
database:
  host: ${lookup:db_host}
  port: ${lookup:db_port}

api:
  url: ${lookup:api_url}
```

Let's try it:

=== "Python"

    ```python
    import holoconf

    # Register resolver (from above)
    holoconf.register("lookup", lookup_resolver)

    # Load config that uses the resolver
    config = holoconf.Config.loads("""
    database:
      host: ${lookup:db_host}
      port: ${lookup:db_port}
    """)

    host = config.database.host
    print(f"Host: {host}")
    # Host: prod-db.example.com

    port = config.database.port
    print(f"Port: {port}")
    # Port: 5432
    ```

=== "Rust"

    ```rust
    // After registering the resolver (from above)

    let yaml = r#"
    database:
      host: ${lookup:db_host}
      port: ${lookup:db_port}
    "#;

    let config = Config::from_yaml_str(yaml)?;
    let host: String = config.get("database.host")?;
    println!("Host: {}", host);
    // Host: prod-db.example.com
    ```

That's it! You've created your first custom resolver.

## Async Resolvers

Many data sources require async I/O - HTTP APIs, database queries, cloud services. HoloConf supports async resolvers seamlessly.

Let's create a resolver that fetches secrets from an HTTP API:

=== "Python"

    ```python
    import holoconf
    import httpx

    # Async resolver function
    async def secret_resolver(key: str) -> str:
        async with httpx.AsyncClient() as client:
            response = await client.get(f"https://secrets.internal/{key}")
            response.raise_for_status()
            return response.text()

    # Register the async resolver
    holoconf.register("secret", secret_resolver)
    ```

Use it just like any other resolver:

=== "Python"

    ```python
    config = holoconf.Config.loads("""
    api:
      key: ${secret:api-key}
      token: ${secret:api-token}
    """)

    # Accessing the value automatically waits for the async call
    api_key = config.api.key
    print(f"API key: {api_key}")
    # API key: super-secret-key-12345
    ```

HoloConf handles the async execution automatically. You don't need to use `await` or manage event loops - just access the value normally.

!!! note "Parallel Resolution"
    When you access multiple async resolver values, HoloConf resolves them in parallel for better performance:

    ```python
    # Both fetches happen in parallel
    api_key = config.api.key
    api_token = config.api.token
    ```

## Returning Sensitive Values

Some resolvers fetch secrets that should never appear in logs or dumps. Mark these values as sensitive:

=== "Python"

    ```python
    import holoconf
    from holoconf import ResolvedValue

    # Use a class-based resolver to return metadata
    class VaultResolver:
        def __init__(self, vault_client):
            self.client = vault_client

        def resolve(self, key: str) -> ResolvedValue:
            # Fetch secret from Vault
            secret = self.client.read(f"secret/data/{key}")

            # Return value with sensitive=True
            return ResolvedValue(
                value=secret["data"]["value"],
                sensitive=True  # All Vault values are secrets
            )

    # Register the resolver
    vault_client = get_vault_client()  # Your Vault client
    holoconf.register("vault", VaultResolver(vault_client))
    ```

=== "Rust"

    ```rust
    use holoconf_core::{ResolvedValue, Resolver, ResolverResult};

    struct VaultResolver {
        client: VaultClient,
    }

    impl Resolver for VaultResolver {
        fn resolve(&self, key: &str) -> ResolverResult<ResolvedValue> {
            let secret = self.client.read(&format!("secret/data/{}", key))?;

            Ok(ResolvedValue {
                value: secret.data.value,
                sensitive: true,  // Mark as sensitive
            })
        }
    }
    ```

Now when you use the resolver, values are automatically redacted:

```yaml
api:
  key: ${vault:api-key}
  secret: ${vault:api-secret}
```

=== "Python"

    ```python
    config = holoconf.Config.load("config.yaml")

    # Sensitive values are redacted in dumps
    print(config.to_yaml(redact=True))
    # api:
    #   key: '[REDACTED]'
    #   secret: '[REDACTED]'

    # But you can still access them
    api_key = config.api.key
    print(f"Key length: {len(api_key)}")
    # Key length: 32
    ```

### Simple Functions vs Classes

There are two ways to write resolvers:

**Simple function** (values not marked sensitive):

=== "Python"

    ```python
    def my_resolver(key: str) -> str:
        return lookup_value(key)

    holoconf.register("myresolver", my_resolver)
    ```

**Class-based** (can return sensitivity metadata):

=== "Python"

    ```python
    from holoconf import ResolvedValue

    class MyResolver:
        def resolve(self, key: str) -> ResolvedValue:
            value = lookup_value(key)
            is_secret = key.startswith("secret/")

            return ResolvedValue(
                value=value,
                sensitive=is_secret
            )

    holoconf.register("myresolver", MyResolver())
    ```

Use the simple function form for non-sensitive data. Use the class form when you need to mark values as sensitive.

## Error Handling

What happens when your resolver encounters an error? Let's see different scenarios:

### Missing Values

If a value doesn't exist, raise an exception:

=== "Python"

    ```python
    def my_resolver(key: str) -> str:
        if key not in MY_DATA:
            raise KeyError(f"Key not found: {key}")
        return MY_DATA[key]

    holoconf.register("myresolver", my_resolver)
    ```

When accessed:

=== "Python"

    ```python
    from holoconf import ResolverError

    config = holoconf.Config.loads("value: ${myresolver:missing-key}")

    try:
        value = config.value
    except ResolverError as e:
        print(f"Error: {e}")
        # Error: Resolver 'myresolver' failed: Key not found: missing-key
    ```

The error message includes the resolver name and your error message, making debugging easier.

### Network Errors

For network-based resolvers, handle transient failures gracefully:

=== "Python"

    ```python
    import httpx
    import holoconf
    from holoconf import ResolvedValue

    class APIResolver:
        def __init__(self, base_url: str, timeout: int = 10):
            self.base_url = base_url
            self.timeout = timeout

        async def resolve(self, key: str) -> ResolvedValue:
            try:
                async with httpx.AsyncClient() as client:
                    response = await client.get(
                        f"{self.base_url}/{key}",
                        timeout=self.timeout
                    )
                    response.raise_for_status()
                    return ResolvedValue(value=response.json())

            except httpx.TimeoutException:
                raise TimeoutError(f"API request timed out after {self.timeout}s")

            except httpx.HTTPStatusError as e:
                if e.response.status_code == 404:
                    raise KeyError(f"Value not found: {key}")
                else:
                    raise RuntimeError(f"API error: {e.response.status_code}")

    holoconf.register("api", APIResolver("https://api.internal"))
    ```

Specific error messages help users understand what went wrong.

### Fallback to Defaults

Remember that users can provide defaults:

```yaml
# If the resolver fails, use the default
value: ${myresolver:some-key,default=fallback-value}
```

You don't need to handle this in your resolver - HoloConf does it automatically.

## Returning Complex Types

Resolvers aren't limited to strings. You can return structured data:

=== "Python"

    ```python
    import holoconf

    def database_resolver(env: str) -> dict:
        # Return entire database config as a dict
        configs = {
            "prod": {
                "host": "prod-db.example.com",
                "port": 5432,
                "pool_size": 50
            },
            "dev": {
                "host": "localhost",
                "port": 5432,
                "pool_size": 10
            }
        }

        if env not in configs:
            raise ValueError(f"Unknown environment: {env}")

        return configs[env]

    holoconf.register("dbconfig", database_resolver)
    ```

Use it in your config:

```yaml
database: ${dbconfig:prod}
```

Access nested values:

=== "Python"

    ```python
    config = holoconf.Config.loads("database: ${dbconfig:prod}")

    # Returns the dict
    db_config = config.database
    print(f"Database config: {db_config}")
    # Database config: {'host': 'prod-db.example.com', 'port': 5432, 'pool_size': 50}

    # Access nested values with dot notation
    host = config.database.host
    port = config.database.port
    print(f"Connect to {host}:{port}")
    # Connect to prod-db.example.com:5432
    ```

You can return:

- Strings, integers, floats, booleans
- Dictionaries (become nested Config objects)
- Lists

## Resolver Options

Resolvers can accept options just like built-in resolvers:

```yaml
value: ${myresolver:key,option1=value1,option2=value2}
```

Access options in your resolver:

=== "Python"

    ```python
    from typing import Optional

    class ConfigurableResolver:
        def resolve(self, key: str, region: Optional[str] = None, timeout: int = 30) -> str:
            # Use the options
            print(f"Fetching {key} from region {region} with timeout {timeout}s")

            # Your logic here
            return fetch_value(key, region=region, timeout=timeout)

    holoconf.register("myresolver", ConfigurableResolver())
    ```

Use it:

```yaml
# Uses default region and timeout
value1: ${myresolver:key1}

# Overrides region
value2: ${myresolver:key2,region=us-west-2}

# Overrides both
value3: ${myresolver:key3,region=eu-west-1,timeout=60}
```

The framework-level options (`default`, `sensitive`) are handled automatically - you don't need to implement them.

## Real-World Example: HashiCorp Vault

Let's create a complete Vault resolver:

=== "Python"

    ```python
    import holoconf
    from holoconf import ResolvedValue
    import hvac  # pip install hvac

    class VaultResolver:
        def __init__(self, url: str, token: str):
            self.client = hvac.Client(url=url, token=token)

        def resolve(self, path: str, key: Optional[str] = None) -> ResolvedValue:
            """
            Fetch secret from Vault.

            Args:
                path: Vault path (e.g., 'secret/data/myapp')
                key: Optional key within the secret (e.g., 'password')
            """
            try:
                # Read secret from Vault
                response = self.client.secrets.kv.v2.read_secret_version(path=path)
                data = response["data"]["data"]

                # If key specified, extract it
                if key:
                    if key not in data:
                        raise KeyError(f"Key '{key}' not found in secret at {path}")
                    value = data[key]
                else:
                    value = data

                # All Vault values are sensitive
                return ResolvedValue(value=value, sensitive=True)

            except hvac.exceptions.InvalidPath:
                raise KeyError(f"Vault path not found: {path}")

            except Exception as e:
                raise RuntimeError(f"Vault error: {e}")

    # Register with your Vault instance
    import os
    vault_resolver = VaultResolver(
        url=os.environ.get("VAULT_ADDR", "http://localhost:8200"),
        token=os.environ["VAULT_TOKEN"]
    )
    holoconf.register("vault", vault_resolver)
    ```

Now use it in your config:

```yaml
database:
  # Fetch entire secret
  credentials: ${vault:secret/data/database}

  # Fetch specific key from secret
  password: ${vault:secret/data/database,key=password}
  username: ${vault:secret/data/database,key=username}

api:
  token: ${vault:secret/data/api,key=token}
```

=== "Python"

    ```python
    config = holoconf.Config.load("config.yaml")

    # Access values normally
    password = config.database.password
    username = config.database.username

    # Sensitive values are automatically redacted
    print(config.to_yaml(redact=True))
    # database:
    #   credentials: '[REDACTED]'
    #   password: '[REDACTED]'
    #   username: '[REDACTED]'
    ```

## Testing Custom Resolvers

When testing code that uses custom resolvers, you can register mock resolvers:

=== "Python"

    ```python
    import holoconf
    import pytest

    @pytest.fixture
    def mock_vault():
        """Register a mock Vault resolver for testing"""
        mock_data = {
            "secret/data/database": {
                "username": "test_user",
                "password": "test_password"
            },
            "secret/data/api": {
                "token": "test_token"
            }
        }

        def mock_vault_resolver(path: str, key: str = None):
            if path not in mock_data:
                raise KeyError(f"Mock secret not found: {path}")

            data = mock_data[path]
            if key:
                return data[key]
            return data

        holoconf.register("vault", mock_vault_resolver)
        yield
        # Cleanup after test
        holoconf.unregister("vault")

    def test_database_config(mock_vault):
        config = holoconf.Config.loads("""
        database:
          username: ${vault:secret/data/database,key=username}
          password: ${vault:secret/data/database,key=password}
        """)

        assert config.database.username == "test_user"
        assert config.database.password == "test_password"
    ```

This makes your tests fast and reliable without needing actual Vault infrastructure.

## Quick Reference

### Function Resolver

Simple resolvers that return plain values:

=== "Python"

    ```python
    def my_resolver(key: str) -> str:
        return fetch_value(key)

    holoconf.register("myresolver", my_resolver)
    ```

### Async Resolver

For async I/O:

=== "Python"

    ```python
    async def my_async_resolver(key: str) -> str:
        return await fetch_async(key)

    holoconf.register("myresolver", my_async_resolver)
    ```

### Class Resolver with Sensitivity

For marking values as sensitive:

=== "Python"

    ```python
    from holoconf import ResolvedValue

    class MyResolver:
        def resolve(self, key: str) -> ResolvedValue:
            return ResolvedValue(
                value=fetch_value(key),
                sensitive=is_secret(key)
            )

    holoconf.register("myresolver", MyResolver())
    ```

### Resolver with Options

For configurable resolvers:

=== "Python"

    ```python
    class MyResolver:
        def resolve(self, key: str, region: str = "us-east-1", timeout: int = 30) -> str:
            return fetch_value(key, region=region, timeout=timeout)

    holoconf.register("myresolver", MyResolver())
    ```

Usage:

```yaml
value: ${myresolver:key,region=us-west-2,timeout=60}
```

## Best Practices

1. **Raise clear exceptions** - Include the key/path in error messages
2. **Mark secrets as sensitive** - Use `ResolvedValue(sensitive=True)` for secrets
3. **Handle missing values** - Raise `KeyError` for missing values (users can provide `default=`)
4. **Support async when needed** - Use async resolvers for I/O-bound operations
5. **Validate options** - Check that required options are provided
6. **Cache when appropriate** - Cache expensive lookups for the lifetime of the resolver
7. **Test with mocks** - Write tests using mock resolvers

## What You've Learned

You now understand:

- Writing simple function-based resolvers
- Creating async resolvers for I/O-bound operations
- Returning sensitive values with metadata
- Handling errors gracefully
- Returning complex types (dicts, lists)
- Adding configurable options to resolvers
- Testing custom resolvers with mocks
- Real-world examples (Vault integration)

Custom resolvers make HoloConf infinitely extensible. You can integrate with any data source your organization uses.

## Next Steps

- **[Core Resolvers](resolvers-core.md)** - Learn about built-in resolvers
- **[AWS Resolvers](resolvers-aws.md)** - Integrate with AWS services

## See Also

- [ADR-002 Resolver Architecture](../adr/ADR-002-resolver-architecture.md) - Technical design
- [ADR-019 Resolver Extension Packages](../adr/ADR-019-resolver-extension-packages.md) - Extension architecture

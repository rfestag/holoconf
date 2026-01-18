# Schema Validation

Configuration errors in production are expensive. A typo in a port number. A missing required field. A value that's the wrong type. These bugs slip through because configuration often isn't validated until runtime - when it's too late.

Let's learn how to catch these errors early using JSON Schema validation.

## Why Validate Configuration?

Imagine deploying your application to production. Everything seems fine until:

```python
# Your code expects an integer
pool_size = config.database.pool_size
connection_pool.initialize(pool_size)
# TypeError: expected int, got str
```

Someone set `pool_size: "10"` (a string) instead of `pool_size: 10` (an integer). The application crashes.

Or worse:

```python
db_host = config.database.host
# AttributeError: 'dict' object has no attribute 'host'
```

Someone forgot to set the database host at all.

These errors should be caught before deployment, not discovered in production. That's where schema validation comes in.

## Loading with a Schema

Let's start with a simple configuration:

```yaml
# config.yaml
app:
  name: my-application

database:
  host: localhost
```

And a schema that describes what we expect:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["app", "database"],
  "properties": {
    "app": {
      "type": "object",
      "required": ["name"],
      "properties": {
        "name": { "type": "string" },
        "debug": { "type": "boolean", "default": false }
      }
    },
    "database": {
      "type": "object",
      "required": ["host"],
      "properties": {
        "host": { "type": "string" },
        "port": { "type": "integer", "minimum": 1, "maximum": 65535, "default": 5432 },
        "pool_size": { "type": "integer", "minimum": 1, "default": 10 }
      }
    }
  }
}
```

Now let's load the configuration with the schema attached:

=== "Python"

    ```python
    from holoconf import Config

    # Load config with schema attached
    config = Config.load("config.yaml", schema="schema.json")

    # The schema provides defaults for missing values
    debug = config.app.debug
    print(f"Debug mode: {debug}")
    # Debug mode: False (from schema default)

    port = config.database.port
    print(f"Port: {port}")
    # Port: 5432 (from schema default)

    pool_size = config.database.pool_size
    print(f"Pool size: {pool_size}")
    # Pool size: 10 (from schema default)
    ```

=== "CLI"

    ```bash
    # Get value with schema defaults
    $ holoconf get config.yaml database.port --schema schema.json
    5432

    # Dump config with schema defaults applied
    $ holoconf dump config.yaml --schema schema.json --resolve
    app:
      name: my-application
      debug: false
    database:
      host: localhost
      port: 5432
      pool_size: 10
    ```

Notice how the schema filled in missing values? You get sensible defaults without cluttering your configuration file.

## Validating Configuration

Here's something important: attaching a schema does **not** automatically validate. The schema only provides defaults. To actually check if your configuration is valid, you need to call `validate()`:

=== "Python"

    ```python
    from holoconf import Config, ValidationError

    config = Config.load("config.yaml", schema="schema.json")

    try:
        config.validate()  # Uses the attached schema
        print("Configuration is valid!")
    except ValidationError as e:
        print(f"Validation failed: {e}")
    ```

=== "CLI"

    ```bash
    $ holoconf validate config.yaml --schema schema.json
    Configuration is valid!
    ```

Let's see what happens when validation fails. Create a broken configuration:

```yaml
# broken.yaml
app:
  name: my-application

database:
  host: localhost
  port: "invalid"  # Should be an integer, not a string
```

Now try to validate it:

=== "Python"

    ```python
    from holoconf import Config, ValidationError

    config = Config.load("broken.yaml", schema="schema.json")

    try:
        config.validate()
    except ValidationError as e:
        print(f"Error at {e.path}: {e.message}")
        # Error at database.port: "invalid" is not of type 'integer'
    ```

=== "CLI"

    ```bash
    $ holoconf validate broken.yaml --schema schema.json
    Validation error at 'database.port': "invalid" is not of type 'integer'
    ```

The validation error tells you exactly what's wrong and where. This makes fixing configuration errors much easier!

## Schema Defaults vs Config Values

When you attach a schema, there's a precedence order for values:

1. **Config value** - If the path exists in your config file, that value is used
2. **Resolver default** - If using `${env:VAR,default=value}`, the resolver's default
3. **Schema default** - If the path is missing and the schema has a default

Let's see this in action:

```json
// schema.json
{
  "type": "object",
  "properties": {
    "database": {
      "type": "object",
      "properties": {
        "port": { "type": "integer", "default": 5432 },
        "pool_size": { "type": "integer", "default": 10 }
      }
    }
  }
}
```

```yaml
# config.yaml
database:
  port: ${env:DB_PORT,default=3306}
  # pool_size not specified - will use schema default
```

=== "Python"

    ```python
    import os
    from holoconf import Config

    config = Config.load("config.yaml", schema="schema.json")

    # Scenario 1: Environment variable is set
    os.environ["DB_PORT"] = "5433"
    port = config.database.port
    print(f"Port: {port}")
    # Port: 5433 (from environment variable)

    # Scenario 2: Environment variable not set
    del os.environ["DB_PORT"]
    config = Config.load("config.yaml", schema="schema.json")
    port = config.database.port
    print(f"Port: {port}")
    # Port: 3306 (from resolver default, not schema default)

    # pool_size not in config, uses schema default
    pool_size = config.database.pool_size
    print(f"Pool size: {pool_size}")
    # Pool size: 10 (from schema default)
    ```

This shows how the three levels work together: config values override everything, resolver defaults override schema defaults, and schema defaults fill in the rest.

## Type Coercion

Here's something powerful: when you validate, HoloConf can automatically coerce values to match the schema's types.

```yaml
# config.yaml
database:
  port: "5432"  # String in YAML
  pool_size: "10"  # String in YAML
```

Without validation, these are strings. But with a schema that says they should be integers:

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config.yaml", schema="schema.json")

    # Without validation - they're still strings
    port = config.database.port
    print(f"Port type: {type(port)}, value: {port}")
    # Port type: <class 'str'>, value: 5432

    # After validation - they're coerced to integers
    config.validate()
    port = config.database.port
    print(f"Port type: {type(port)}, value: {port}")
    # Port type: <class 'int'>, value: 5432
    ```

This is incredibly helpful when loading configuration from sources that don't preserve types (like environment variables, which are always strings).

!!! note "Type Coercion Details"
    For full details on how type coercion works, see [ADR-012 Type Coercion](../adr/ADR-012-type-coercion.md).

## Null Handling

What about `null` values? The schema determines how these are handled:

```json
// schema.json
{
  "type": "object",
  "properties": {
    "timeout": {
      "type": "integer",  // null not allowed
      "default": 30
    },
    "optional_value": {
      "type": ["integer", "null"],  // null explicitly allowed
      "default": 30
    }
  }
}
```

```yaml
# config.yaml
timeout: null
optional_value: null
```

=== "Python"

    ```python
    from holoconf import Config

    config = Config.load("config.yaml", schema="schema.json")

    # timeout is null but schema doesn't allow null - uses default
    timeout = config.timeout
    print(f"Timeout: {timeout}")
    # Timeout: 30 (schema default)

    # optional_value is null and schema allows null - preserved
    optional = config.optional_value
    print(f"Optional: {optional}")
    # Optional: None
    ```

This gives you fine-grained control: some fields can be `null`, others can't.

## Validation Errors: Catching Problems

Let's create a configuration with multiple problems and see what validation tells us:

```yaml
# broken-config.yaml
app:
  # Missing required 'name' field
  debug: "yes"  # Should be boolean

database:
  host: localhost
  port: -1  # Port must be >= 1
  pool_size: 0  # Pool size must be >= 1
```

Now validate:

=== "Python"

    ```python
    from holoconf import Config, ValidationError

    config = Config.load("broken-config.yaml", schema="schema.json")

    try:
        config.validate()
    except ValidationError as e:
        print(f"Path: {e.path}")
        print(f"Message: {e.message}")
        # Path: app.name
        # Message: 'name' is a required property
    ```

=== "CLI"

    ```bash
    $ holoconf validate broken-config.yaml --schema schema.json
    Validation error at 'app.name': 'name' is a required property
    ```

Validation stops at the first error, so fix that and run again:

```yaml
# broken-config.yaml (fixed app.name)
app:
  name: my-application
  debug: "yes"  # Still wrong

database:
  host: localhost
  port: -1  # Still wrong
  pool_size: 0  # Still wrong
```

=== "CLI"

    ```bash
    $ holoconf validate broken-config.yaml --schema schema.json
    Validation error at 'app.debug': "yes" is not of type 'boolean'
    ```

Fix each error one by one until your configuration is valid.

!!! tip "Validate in CI/CD"
    Add validation to your CI/CD pipeline:

    ```bash
    # In your build script
    holoconf validate config/production.yaml --schema config/schema.json
    ```

    This catches configuration errors before they reach production!

## When to Validate

You might wonder: should you always validate? Here are some guidelines:

**Always validate:**
- Production configurations before deployment
- Configuration changes in CI/CD pipelines
- User-provided configuration files

**Sometimes validate:**
- Development configurations (helps catch errors early)
- Configurations generated from templates

**Rarely validate:**
- Trusted internal configurations
- Configuration fragments being merged
- Performance-critical paths (validation has overhead)

The key is balancing safety and performance. For production deployments, safety wins every time.

## Complete Example

Let's put it all together with a realistic example:

```json
// schema.json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["app", "database"],
  "properties": {
    "app": {
      "type": "object",
      "required": ["name"],
      "properties": {
        "name": { "type": "string", "minLength": 1 },
        "debug": { "type": "boolean", "default": false },
        "port": { "type": "integer", "minimum": 1024, "maximum": 65535, "default": 8000 }
      }
    },
    "database": {
      "type": "object",
      "required": ["host"],
      "properties": {
        "host": { "type": "string", "minLength": 1 },
        "port": { "type": "integer", "minimum": 1, "maximum": 65535, "default": 5432 },
        "pool_size": { "type": "integer", "minimum": 1, "maximum": 100, "default": 10 },
        "timeout": { "type": "integer", "minimum": 1, "default": 30 }
      }
    },
    "logging": {
      "type": "object",
      "properties": {
        "level": {
          "type": "string",
          "enum": ["debug", "info", "warning", "error"],
          "default": "info"
        },
        "format": {
          "type": "string",
          "enum": ["json", "text"],
          "default": "json"
        }
      }
    }
  }
}
```

```yaml
# config.yaml
app:
  name: my-application
  debug: ${env:DEBUG,default=false}

database:
  host: ${env:DB_HOST,default=localhost}
  port: ${env:DB_PORT,default=5432}

logging:
  level: ${env:LOG_LEVEL,default=info}
```

=== "Python"

    ```python
    from holoconf import Config, ValidationError

    # Load with schema
    config = Config.load("config.yaml", schema="schema.json")

    # Validate before using
    try:
        config.validate()
        print("Configuration is valid!")
    except ValidationError as e:
        print(f"Configuration error at {e.path}: {e.message}")
        exit(1)

    # Now safe to use
    app_name = config.app.name
    db_host = config.database.host
    pool_size = config.database.pool_size  # From schema default

    print(f"Starting {app_name}")
    print(f"Connecting to {db_host}")
    print(f"Pool size: {pool_size}")
    ```

=== "CLI"

    ```bash
    # Validate first
    $ holoconf validate config.yaml --schema schema.json
    Configuration is valid!

    # Then use it
    $ holoconf dump config.yaml --schema schema.json --resolve
    app:
      name: my-application
      debug: false
      port: 8000
    database:
      host: localhost
      port: 5432
      pool_size: 10
      timeout: 30
    logging:
      level: info
      format: json
    ```

!!! tip "Try It Yourself"
    Create your own schema and configuration:

    1. Start with a simple schema with one or two fields
    2. Add default values
    3. Create a config file (intentionally with errors)
    4. Run validation and fix the errors
    5. Add more constraints (minimum, maximum, enum)
    6. See how validation catches violations

## What You've Learned

You now understand:

- Why validation is important (catch errors before production)
- How to load configuration with a schema attached
- The difference between attaching a schema and validating
- Value precedence: config values → resolver defaults → schema defaults
- Type coercion during validation
- How to handle null values
- Reading validation error messages
- When to validate (always in production!)

Validation gives you confidence that your configuration is correct before it reaches production. Combined with HoloConf's other features, you get configuration that's both flexible and safe.

## Next Steps

- **[ADR-007 Schema Validation](../adr/ADR-007-schema-validation.md)** - Design rationale for validation
- **[ADR-012 Type Coercion](../adr/ADR-012-type-coercion.md)** - How type coercion works
- Learn about [JSON Schema](https://json-schema.org/) to write more sophisticated schemas

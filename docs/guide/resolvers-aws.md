# AWS Resolvers

When running applications on AWS, you often need to fetch configuration from AWS services like SSM Parameter Store, CloudFormation stacks, or S3 buckets. HoloConf provides AWS-specific resolvers to make this seamless.

## Installation

AWS resolvers are distributed separately to keep the core library lean:

=== "Python"

    ```bash
    # Install both holoconf and holoconf-aws
    pip install holoconf holoconf-aws
    ```

    AWS resolvers are automatically discovered when you import holoconf:

    ```python
    import holoconf  # AWS resolvers auto-register if holoconf-aws is installed

    config = holoconf.Config.load("config.yaml")
    password = config.database.password  # Can use ${ssm:...} resolver
    ```

=== "Rust"

    Add both crates to your `Cargo.toml`:

    ```toml
    [dependencies]
    holoconf-core = "0.1"
    holoconf-aws = "0.1"
    ```

    Then register AWS resolvers explicitly:

    ```rust
    use holoconf_core::Config;
    use holoconf_aws;

    // Register all AWS resolvers
    holoconf_aws::register_all();

    let config = Config::load("config.yaml")?;
    ```

## SSM Parameter Store

AWS Systems Manager Parameter Store is perfect for storing configuration and secrets. Let's see how to use it.

First, let's try to reference an SSM parameter:

```yaml
database:
  host: ${ssm:/myapp/prod/db-host}
  password: ${ssm:/myapp/prod/db-password}
```

=== "Python"

    ```python
    import holoconf

    config = holoconf.Config.load("config.yaml")

    # Fetches from SSM Parameter Store
    host = config.database.host
    print(f"Database host: {host}")
    # Database host: prod-db.example.com

    password = config.database.password
    print(f"Password: {password}")
    # Password: super-secret-password
    ```

=== "Rust"

    ```rust
    use holoconf_core::Config;
    use holoconf_aws;

    holoconf_aws::register_all();

    let config = Config::load("config.yaml")?;
    let host: String = config.get("database.host")?;
    println!("Database host: {}", host);
    // Database host: prod-db.example.com
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml database.host
    prod-db.example.com

    $ holoconf get config.yaml database.password
    super-secret-password
    ```

### Automatic Decryption

SSM parameters are automatically decrypted if they use AWS KMS encryption. You don't need to do anything special:

```yaml
# This parameter is encrypted with KMS
api_key: ${ssm:/myapp/prod/api-key}
```

HoloConf automatically calls SSM with `WithDecryption=true`, so you get the decrypted value.

### Automatic Sensitivity Detection

SSM SecureString parameters are automatically marked as sensitive and redacted in dumps:

```yaml
password: ${ssm:/myapp/prod/db-password}  # SecureString parameter
```

=== "Python"

    ```python
    import holoconf

    config = holoconf.Config.load("config.yaml")

    # Sensitive values are automatically redacted
    print(config.to_yaml(redact=True))
    # password: '[REDACTED]'

    # But you can still access the actual value
    password = config.password
    print(f"Password length: {len(password)}")
    # Password length: 20
    ```

=== "CLI"

    ```bash
    $ holoconf dump config.yaml --resolve
    password: '[REDACTED]'
    ```

If you want to override sensitivity detection, you can do so explicitly:

```yaml
# Force sensitivity even for String parameters
debug_token: ${ssm:/myapp/dev/token,sensitive=true}

# Disable sensitivity for SecureString (not recommended!)
public_value: ${ssm:/myapp/public-key,sensitive=false}
```

### Handling Missing Parameters

What happens if a parameter doesn't exist?

=== "Python"

    ```python
    import holoconf
    from holoconf import ResolverError

    # config.yaml contains: timeout: ${ssm:/myapp/timeout}
    config = holoconf.Config.load("config.yaml")

    try:
        timeout = config.timeout
    except ResolverError as e:
        print(f"Error: {e}")
        # Error: SSM parameter not found: /myapp/timeout
    ```

=== "Rust"

    ```rust
    use holoconf_core::{Config, Error};

    match config.get::<String>("timeout") {
        Err(Error::ResolverError { message, .. }) => {
            println!("Error: {}", message);
        }
        _ => {}
    }
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml timeout
    Error: SSM parameter not found: /myapp/timeout
    ```

Provide a default for optional parameters:

```yaml
timeout: ${ssm:/myapp/timeout,default=30}
```

Now if the parameter doesn't exist, it uses `30` instead of erroring.

### Cross-Region Parameters

By default, SSM parameters are fetched from your configured AWS region. To fetch from a different region:

```yaml
# Fetch from us-west-2, even if default region is us-east-1
west_config: ${ssm:/shared/config,region=us-west-2}
```

=== "Python"

    ```python
    import holoconf

    config = holoconf.Config.load("config.yaml")
    west_config = config.west_config
    # Fetched from us-west-2
    ```

=== "Rust"

    ```rust
    let config = Config::load("config.yaml")?;
    let west_config: String = config.get("west_config")?;
    // Fetched from us-west-2
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml west_config
    # Fetched from us-west-2
    ```

### AWS Secrets Manager Integration

SSM provides a special path prefix to access Secrets Manager:

```yaml
# Access Secrets Manager secret via SSM
db_creds: ${ssm:/aws/reference/secretsmanager/myapp/db-credentials}
```

This is convenient because you can use the same resolver for both SSM Parameter Store and Secrets Manager.

### Authentication and Credentials

SSM resolvers use the standard AWS credential chain:

1. Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
2. AWS profile from `~/.aws/credentials`
3. IAM instance profile (when running on EC2)
4. ECS task role (when running in ECS)

To use a specific profile:

```yaml
shared_config: ${ssm:/shared/config,profile=shared-account}
```

Or set the environment variable:

```bash
export AWS_PROFILE=my-profile
holoconf get config.yaml database.host
```

## CloudFormation Outputs

When you deploy infrastructure with CloudFormation, you often need to reference stack outputs in your application configuration. The `cfn` resolver makes this easy.

Let's say you have a CloudFormation stack called `myapp-infrastructure` with these outputs:

- `DatabaseEndpoint` - The database host
- `CacheEndpoint` - The Redis cache host
- `ApiUrl` - The API endpoint

Reference them in your config:

```yaml
database:
  host: ${cfn:myapp-infrastructure.DatabaseEndpoint}

cache:
  host: ${cfn:myapp-infrastructure.CacheEndpoint}

api:
  url: ${cfn:myapp-infrastructure.ApiUrl}
```

=== "Python"

    ```python
    import holoconf

    config = holoconf.Config.load("config.yaml")

    # Fetches stack outputs from CloudFormation
    db_host = config.database.host
    print(f"Database: {db_host}")
    # Database: prod-db.us-east-1.rds.amazonaws.com

    api_url = config.api.url
    print(f"API: {api_url}")
    # API: https://api.example.com
    ```

=== "Rust"

    ```rust
    use holoconf_core::Config;
    use holoconf_aws;

    holoconf_aws::register_all();

    let config = Config::load("config.yaml")?;
    let db_host: String = config.get("database.host")?;
    println!("Database: {}", db_host);
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml database.host
    prod-db.us-east-1.rds.amazonaws.com
    ```

### Syntax

The CloudFormation resolver uses this syntax:

```
${cfn:StackName.OutputKey}
```

For example:

```yaml
endpoint: ${cfn:myapp-prod.ApiEndpoint}
```

This fetches the `ApiEndpoint` output from the `myapp-prod` stack.

### Handling Missing Stacks or Outputs

What if the stack doesn't exist?

=== "Python"

    ```python
    import holoconf
    from holoconf import ResolverError

    # config.yaml contains: host: ${cfn:missing-stack.Output}
    config = holoconf.Config.load("config.yaml")

    try:
        host = config.host
    except ResolverError as e:
        print(f"Error: {e}")
        # Error: CloudFormation stack not found: missing-stack
    ```

=== "Rust"

    ```rust
    match config.get::<String>("host") {
        Err(Error::ResolverError { message, .. }) => {
            println!("Error: {}", message);
        }
        _ => {}
    }
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml host
    Error: CloudFormation stack not found: missing-stack
    ```

Or if the output key doesn't exist:

```yaml
host: ${cfn:myapp-stack.NonExistentOutput}
```

```
Error: CloudFormation output not found: NonExistentOutput in stack myapp-stack
```

Provide a default for optional outputs:

```yaml
optional_endpoint: ${cfn:myapp-stack.OptionalOutput,default=http://localhost:8000}
```

### Cross-Region Stacks

To reference a stack in a different region:

```yaml
west_endpoint: ${cfn:myapp-stack.Endpoint,region=us-west-2}
```

=== "Python"

    ```python
    config = holoconf.Config.load("config.yaml")
    endpoint = config.west_endpoint
    # Fetches from CloudFormation in us-west-2
    ```

## S3 Objects

For larger configuration files or shared team configurations, you can store them in S3 and reference them with the `s3` resolver.

Let's say you have a shared configuration file in S3:

```
s3://my-config-bucket/shared/database.json
```

Reference it in your config:

```yaml
database: ${s3:my-config-bucket/shared/database.json}
```

=== "Python"

    ```python
    import holoconf

    config = holoconf.Config.load("config.yaml")

    # Fetches and parses the JSON from S3
    db_config = config.database
    print(f"Host: {db_config['host']}")
    # Host: prod-db.example.com
    ```

=== "Rust"

    ```rust
    use holoconf_core::Config;
    use holoconf_aws;

    holoconf_aws::register_all();

    let config = Config::load("config.yaml")?;
    let db_config: serde_json::Value = config.get("database")?;
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml database
    host: prod-db.example.com
    port: 5432
    ```

### Automatic Format Detection

S3 objects are automatically parsed based on file extension:

- `.json` - Parsed as JSON
- `.yaml`, `.yml` - Parsed as YAML
- `.txt`, `.pem`, or no extension - Returned as plain text

```yaml
# Parses as JSON
api_config: ${s3:my-bucket/config/api.json}

# Parses as YAML
db_config: ${s3:my-bucket/config/database.yaml}

# Returns as plain text
certificate: ${s3:my-bucket/certs/server.pem}
```

### S3 URI Syntax

You can use either format:

```yaml
# Without s3:// prefix (recommended)
config: ${s3:my-bucket/path/to/file.json}

# With s3:// prefix (also works)
config: ${s3:s3://my-bucket/path/to/file.json}
```

Both work identically.

### Handling Missing Objects

What if the S3 object doesn't exist?

=== "Python"

    ```python
    import holoconf
    from holoconf import ResolverError

    # config.yaml contains: data: ${s3:my-bucket/missing.json}
    config = holoconf.Config.load("config.yaml")

    try:
        data = config.data
    except ResolverError as e:
        print(f"Error: {e}")
        # Error: S3 object not found: s3://my-bucket/missing.json
    ```

=== "Rust"

    ```rust
    match config.get::<String>("data") {
        Err(Error::ResolverError { message, .. }) => {
            println!("Error: {}", message);
        }
        _ => {}
    }
    ```

=== "CLI"

    ```bash
    $ holoconf get config.yaml data
    Error: S3 object not found: s3://my-bucket/missing.json
    ```

Provide a default:

```yaml
data: ${s3:my-bucket/optional.json,default={}}
```

### Versioned Objects

To fetch a specific version of an S3 object:

```yaml
config: ${s3:my-bucket/config.json,version_id=abc123}
```

This is useful for:

- Rolling back to a previous configuration
- Ensuring consistent config across deployments
- Auditing configuration changes

### Authentication and Permissions

S3 resolvers use the same AWS credential chain as SSM resolvers. Your credentials need these permissions:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "s3:GetObject"
      ],
      "Resource": "arn:aws:s3:::my-config-bucket/*"
    }
  ]
}
```

## Configuration API

When you need to set defaults for AWS resolvers across your application, the configuration API provides a two-tier system: global configuration that applies to all AWS services, and service-specific configuration for fine-grained control.

### Why Use the Configuration API?

Before diving into the API, let's understand when and why you'd use it:

- **Testing with moto or LocalStack** - Point AWS services to local endpoints for integration tests
- **Multi-region applications** - Set a default region without adding `region=` to every resolver call
- **Environment-based profiles** - Use different AWS profiles for dev, staging, and production
- **Test isolation** - Clean up configuration between test runs

### Global Configuration

Set defaults that apply to all AWS resolvers:

=== "Python"

    ```python
    import holoconf_aws

    # Set defaults for all AWS services (S3, SSM, CloudFormation)
    holoconf_aws.configure(
        region="us-east-1",   # Default region
        profile="prod"        # Default AWS profile
    )
    ```

=== "Rust"

    ```rust
    use holoconf_aws;

    // Set defaults for all AWS services
    holoconf_aws::configure(
        Some("us-east-1".to_string()),
        Some("prod".to_string())
    );
    ```

Now all AWS resolvers will use `us-east-1` and the `prod` profile by default:

```yaml
# All three use us-east-1 and prod profile
database:
  password: ${ssm:/myapp/db-password}
  endpoint: ${cfn:my-stack.DatabaseEndpoint}
  schema: ${s3:my-bucket/schema.sql}
```

### Service-Specific Configuration

Override global defaults for individual services. This is particularly useful for setting custom endpoints when testing with moto or LocalStack:

=== "Python"

    ```python
    import holoconf_aws

    # Configure S3 to use LocalStack
    holoconf_aws.s3(
        endpoint="http://localhost:4566",
        region="us-west-2"    # Can override global region
    )

    # Configure SSM separately
    holoconf_aws.ssm(
        endpoint="http://localhost:4566",
        profile="testing"     # Can override global profile
    )

    # Configure CloudFormation
    holoconf_aws.cfn(
        endpoint="http://localhost:4566"
    )
    ```

=== "Rust"

    ```rust
    use holoconf_aws;

    // Configure S3 for LocalStack
    holoconf_aws::configure_s3(
        Some("http://localhost:4566".to_string()),
        Some("us-west-2".to_string()),
        None
    );

    // Configure SSM separately
    holoconf_aws::configure_ssm(
        Some("http://localhost:4566".to_string()),
        None,
        Some("testing".to_string())
    );

    // Configure CloudFormation
    holoconf_aws::configure_cfn(
        Some("http://localhost:4566".to_string()),
        None,
        None
    );
    ```

### Configuration Precedence

Configuration follows a four-level precedence chain from highest to lowest priority:

1. **Resolver kwargs** - Explicit overrides in your config file
2. **Service configuration** - Set via `holoconf_aws.s3()`, `ssm()`, or `cfn()`
3. **Global configuration** - Set via `holoconf_aws.configure()`
4. **AWS SDK defaults** - Environment variables, credentials file, IAM roles

Here's how precedence works in practice:

=== "Python"

    ```python
    import holoconf_aws

    # 3. Set global default
    holoconf_aws.configure(region="us-east-1")

    # 2. Override S3 specifically
    holoconf_aws.s3(region="us-west-2")
    ```

```yaml
# Uses us-west-2 (service config overrides global)
config: ${s3:bucket/config.yaml}

# Uses eu-west-1 (resolver kwargs override everything)
europe: ${s3:bucket/eu-config.yaml,region=eu-west-1}

# Uses us-east-1 (global config, no SSM-specific override)
password: ${ssm:/myapp/password}

# 4. AWS SDK default (no configuration set)
# Falls back to AWS_REGION environment variable or ~/.aws/config
```

### Additive Configuration

Configuration calls are **additive** - passing `None` leaves existing values unchanged:

=== "Python"

    ```python
    import holoconf_aws

    # Set both region and profile
    holoconf_aws.configure(region="us-east-1", profile="prod")

    # Later, update only the region - profile remains "prod"
    holoconf_aws.configure(region="us-west-2")

    # Or update only the profile - region remains "us-west-2"
    holoconf_aws.configure(profile="staging")
    ```

=== "Rust"

    ```rust
    use holoconf_aws;

    // Set both region and profile
    holoconf_aws::configure(
        Some("us-east-1".to_string()),
        Some("prod".to_string())
    );

    // Update only region - profile remains "prod"
    holoconf_aws::configure(
        Some("us-west-2".to_string()),
        None
    );

    // Update only profile - region remains "us-west-2"
    holoconf_aws::configure(
        None,
        Some("staging".to_string())
    );
    ```

### Resetting Configuration

To clear all configuration and start fresh, use `reset()`:

=== "Python"

    ```python
    import holoconf_aws

    # Configure for testing
    holoconf_aws.s3(endpoint="http://localhost:4566")
    holoconf_aws.configure(region="us-east-1")

    # ... run tests ...

    # Clean up for next test
    holoconf_aws.reset()

    # All configuration is cleared
    # Client cache is also cleared
    ```

=== "Rust"

    ```rust
    use holoconf_aws;

    // Configure for testing
    holoconf_aws::configure_s3(
        Some("http://localhost:4566".to_string()),
        None,
        None
    );

    // ... run tests ...

    // Clean up for next test
    holoconf_aws::reset();
    ```

The `reset()` function is particularly useful for test isolation - it clears both configuration and the internal AWS client cache.

### Real-World Example: Testing with moto

Let's see how to use the configuration API for testing with moto, the AWS service mocking library:

=== "Python"

    ```python
    import pytest
    import holoconf
    import holoconf_aws
    from moto import mock_aws

    @pytest.fixture
    def aws_config():
        """Configure AWS resolvers for testing."""
        # Point all AWS services to moto's mock endpoints
        holoconf_aws.configure(region="us-east-1")

        # Start moto mock
        with mock_aws():
            # Set up test data
            import boto3
            ssm = boto3.client("ssm", region_name="us-east-1")
            ssm.put_parameter(
                Name="/myapp/db-password",
                Value="test-password",
                Type="SecureString"
            )

            yield

        # Clean up after test
        holoconf_aws.reset()

    def test_config_with_ssm(aws_config):
        """Test that SSM parameters are resolved correctly."""
        config = holoconf.Config.loads("""
        database:
          password: ${ssm:/myapp/db-password}
        """)

        assert config.database.password == "test-password"
    ```

=== "Rust"

    ```rust
    use holoconf_core::Config;
    use holoconf_aws;

    #[test]
    fn test_config_with_localstack() {
        // Configure for LocalStack
        holoconf_aws::configure(
            Some("us-east-1".to_string()),
            None
        );
        holoconf_aws::configure_ssm(
            Some("http://localhost:4566".to_string()),
            None,
            None
        );

        // Register resolvers
        holoconf_aws::register_all();

        // Load config that uses SSM
        let config = Config::from_yaml_str(r#"
            database:
              password: ${ssm:/myapp/db-password}
        "#).unwrap();

        // ... test assertions ...

        // Clean up
        holoconf_aws::reset();
    }
    ```

### Real-World Example: Multi-Region Application

Here's how to handle an application deployed in multiple AWS regions:

=== "Python"

    ```python
    import os
    import holoconf
    import holoconf_aws

    # Set default region from environment
    region = os.environ.get("AWS_REGION", "us-east-1")
    holoconf_aws.configure(region=region)

    # Load config - all AWS resolvers use the configured region
    config = holoconf.Config.load("config.yaml")
    ```

```yaml
# config.yaml - no need to specify region on every resolver
database:
  host: ${ssm:/myapp/db-host}
  password: ${ssm:/myapp/db-password}

cache:
  endpoint: ${cfn:myapp-infra.CacheEndpoint}

feature_flags: ${s3:myapp-config/features.yaml}
```

When you deploy to `us-west-2`, just set `AWS_REGION=us-west-2` and all resolvers automatically use the correct region.

### Real-World Example: Environment-Based Profiles

Use different AWS profiles for different environments:

=== "Python"

    ```python
    import os
    import holoconf
    import holoconf_aws

    # Configure based on environment
    env = os.environ.get("ENV", "dev")

    if env == "dev":
        holoconf_aws.configure(profile="dev", region="us-east-1")
    elif env == "staging":
        holoconf_aws.configure(profile="staging", region="us-east-1")
    elif env == "prod":
        holoconf_aws.configure(profile="prod", region="us-east-1")

    # Load config - uses the appropriate profile
    config = holoconf.Config.load("config.yaml")
    ```

=== "Rust"

    ```rust
    use holoconf_core::Config;
    use holoconf_aws;
    use std::env;

    // Register AWS resolvers
    holoconf_aws::register_all();

    // Configure based on environment
    let environment = env::var("ENV").unwrap_or_else(|_| "dev".to_string());

    match environment.as_str() {
        "dev" => holoconf_aws::configure(
            Some("us-east-1".to_string()),
            Some("dev".to_string())
        ),
        "staging" => holoconf_aws::configure(
            Some("us-east-1".to_string()),
            Some("staging".to_string())
        ),
        "prod" => holoconf_aws::configure(
            Some("us-east-1".to_string()),
            Some("prod".to_string())
        ),
        _ => {}
    }

    // Load config - uses the appropriate profile
    let config = Config::load("config.yaml")?;
    ```

## AWS Authentication Summary

All AWS resolvers (`ssm`, `cfn`, `s3`) use the standard AWS credential chain:

1. **Environment variables**:
   ```bash
   export AWS_ACCESS_KEY_ID=your_key
   export AWS_SECRET_ACCESS_KEY=your_secret
   export AWS_REGION=us-east-1
   ```

2. **AWS profile** from `~/.aws/credentials`:
   ```bash
   export AWS_PROFILE=my-profile
   ```

3. **IAM instance profile** (when running on EC2)

4. **ECS task role** (when running in ECS/Fargate)

5. **IRSA (IAM Roles for Service Accounts)** (when running in EKS)

You can also specify region and profile per-resolver:

```yaml
# Different regions for different parameters
east_db: ${ssm:/myapp/db-host,region=us-east-1}
west_db: ${ssm:/myapp/db-host,region=us-west-2}

# Different profiles for different accounts
prod_config: ${ssm:/prod/config,profile=prod-account}
shared_config: ${ssm:/shared/config,profile=shared-account}
```

## Performance Considerations

### Caching

AWS resolvers cache values for the lifetime of the Config object to avoid repeated API calls:

=== "Python"

    ```python
    config = holoconf.Config.load("config.yaml")

    # First access - fetches from SSM
    password1 = config.database.password

    # Second access - uses cached value
    password2 = config.database.password  # No API call!
    ```

To get fresh values, reload the config:

=== "Python"

    ```python
    # Reload to fetch fresh values
    config = holoconf.Config.load("config.yaml")
    password = config.database.password  # Fetches from SSM again
    ```

### Lazy Resolution

Like all resolvers, AWS resolvers are lazy - they only execute when you access the value:

```yaml
database:
  host: ${ssm:/myapp/db-host}
  backup_host: ${ssm:/myapp/backup-host}
```

=== "Python"

    ```python
    config = holoconf.Config.load("config.yaml")
    # No AWS API calls yet!

    host = config.database.host
    # Now SSM is called for /myapp/db-host

    # backup_host is never accessed, so /myapp/backup-host is never fetched
    ```

This means you only pay for the API calls you actually need.

### Batch Optimization

For SSM parameters, consider using parameter hierarchies to reduce API calls:

```yaml
# Instead of many individual parameters:
db_host: ${ssm:/myapp/prod/db/host}
db_port: ${ssm:/myapp/prod/db/port}
db_name: ${ssm:/myapp/prod/db/name}

# Store as structured data in one parameter:
database: ${ssm:/myapp/prod/database}
```

Then store a JSON value in SSM:

```bash
aws ssm put-parameter \
  --name /myapp/prod/database \
  --type SecureString \
  --value '{"host":"prod-db.example.com","port":5432,"name":"myapp"}'
```

One API call instead of three!

## Quick Reference

| Resolver | Syntax | Description | Example |
|----------|--------|-------------|---------|
| `ssm` | `${ssm:/path}` | SSM Parameter Store | `${ssm:/myapp/prod/db-password}` |
| `cfn` | `${cfn:Stack.Output}` | CloudFormation output | `${cfn:myapp-stack.DatabaseEndpoint}` |
| `s3` | `${s3:bucket/key}` | S3 object content | `${s3:my-bucket/config.json}` |

All AWS resolvers support:

- `default=value` - Fallback if not found
- `sensitive=true/false` - Override sensitivity detection
- `region=name` - Override AWS region

SSM additionally supports:

- `profile=name` - AWS profile for credentials
- Automatic access to Secrets Manager via `/aws/reference/secretsmanager/` prefix

S3 additionally supports:

- `version_id=id` - Fetch specific object version

## What You've Learned

You now understand:

- Installing and registering AWS resolvers
- Fetching parameters from SSM Parameter Store with `${ssm:/path}`
- Automatic decryption and sensitivity detection for SSM
- Referencing CloudFormation stack outputs with `${cfn:Stack.Output}`
- Including S3 object content with `${s3:bucket/key}`
- Cross-region and cross-account access
- AWS authentication and credential chain
- **Configuration API** for setting global and service-specific defaults
- **Precedence chain** for configuration (kwargs > service > global > SDK)
- **Test isolation** with `reset()` for cleaning up between tests
- Caching and performance optimization

## Next Steps

- **[Custom Resolvers](resolvers-custom.md)** - Write your own resolvers for custom data sources
- **[Core Resolvers](resolvers-core.md)** - Environment variables, file includes, HTTP fetching

## See Also

- [ADR-002 Resolver Architecture](../adr/ADR-002-resolver-architecture.md) - Technical design
- [ADR-019 Resolver Extension Packages](../adr/ADR-019-resolver-extension-packages.md) - Extension architecture
- [FEAT-007 AWS Resolvers](../specs/features/FEAT-007-aws-resolvers.md) - Full specification

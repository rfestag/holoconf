# holoconf-aws

AWS resolvers for [holoconf](https://github.com/rfestag/holoconf) configuration library.

## Installation

```bash
pip install holoconf-aws
```

## SSM Parameter Store Resolver

The `ssm` resolver fetches values from AWS Systems Manager Parameter Store.

### Quick Start

```python
import holoconf_aws  # Auto-registers the SSM resolver

from holoconf import Config

config = Config.loads("""
database:
    host: ${ssm:/app/prod/db-host}
    password: ${ssm:/app/prod/db-password}
""")

print(config.get("database.host"))
```

### Usage in YAML

```yaml
database:
    host: ${ssm:/app/prod/db-host}
    password: ${ssm:/app/prod/db-password}

# With options
settings:
    api_key: ${ssm:/app/api-key,region=us-west-2}
    secret: ${ssm:/app/secret,profile=production}
    timeout: ${ssm:/app/timeout,default=30}
```

### Parameter Types

| Type | Behavior |
|------|----------|
| String | Returned as-is |
| SecureString | Automatically marked as sensitive for redaction |
| StringList | Returned as a Python list (split by comma) |

### Kwargs

| Kwarg | Description |
|-------|-------------|
| `region` | Override the AWS region for this parameter |
| `profile` | Use a specific AWS profile |
| `default` | Value to use if parameter not found (framework-handled) |
| `sensitive` | Override automatic sensitivity detection (framework-handled) |

### AWS Authentication

The SSM resolver uses boto3's default credential chain:

1. Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
2. Shared credential file (`~/.aws/credentials`)
3. AWS config file (`~/.aws/config`)
4. IAM role (when running on EC2/ECS/Lambda)

### Manual Registration

The resolver is automatically registered when you import `holoconf_aws`.
For manual control:

```python
from holoconf_aws import register_ssm

# Register with default settings
register_ssm()

# Force re-registration (overwrites existing)
register_ssm(force=True)
```

## License

MIT OR Apache-2.0

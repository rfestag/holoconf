# holoconf-aws

AWS resolvers for [holoconf](https://github.com/rfestag/holoconf) configuration library.

## Installation

```bash
pip install holoconf-aws
```

## Quick Start

```python
import holoconf  # holoconf-aws is auto-discovered if installed

config = holoconf.Config.loads("""
database:
    host: ${ssm:/app/prod/db-host}
    endpoint: ${cfn:my-stack/DatabaseEndpoint}
    settings: ${s3:my-bucket/configs/db.yaml}
""")
```

## SSM Parameter Store Resolver

The `ssm` resolver fetches values from AWS Systems Manager Parameter Store.

### Usage

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

## CloudFormation Resolver

The `cfn` resolver fetches outputs from CloudFormation stacks.

### Usage

```yaml
infrastructure:
    endpoint: ${cfn:my-database-stack/DatabaseEndpoint}
    bucket: ${cfn:my-storage-stack/BucketName}

# With options
    vpc_id: ${cfn:shared-infra/VpcId,region=us-west-2}
    subnet: ${cfn:network-stack/SubnetId,profile=network-account}
    url: ${cfn:optional-stack/ApiUrl,default=http://localhost:8080}
```

### Kwargs

| Kwarg | Description |
|-------|-------------|
| `region` | Override the AWS region for this lookup |
| `profile` | Use a specific AWS profile |
| `default` | Value to use if output not found (framework-handled) |
| `sensitive` | Mark value as sensitive (default: false) |

## S3 Resolver

The `s3` resolver fetches objects from Amazon S3.

### Usage

```yaml
# Auto-parsed based on file extension
shared_config: ${s3:my-bucket/configs/shared.yaml}
feature_flags: ${s3:my-bucket/settings/flags.json}

# Raw text content
readme: ${s3:my-bucket/docs/README.md,parse=text}

# With options
config: ${s3:bucket/config.yaml,region=eu-west-1}
shared: ${s3:shared-bucket/common.yaml,profile=shared-account}
optional: ${s3:bucket/optional.yaml,default={}}
```

### Parse Modes

| Mode | Description |
|------|-------------|
| `auto` (default) | Detect by file extension (.yaml/.yml, .json) or Content-Type |
| `yaml` | Parse as YAML |
| `json` | Parse as JSON |
| `text` | Return raw text content |
| `binary` | Return raw bytes (useful for certificates, images, etc.) |

### Kwargs

| Kwarg | Description |
|-------|-------------|
| `parse` | How to interpret content (auto, yaml, json, text, binary) |
| `region` | Override the AWS region for this lookup |
| `profile` | Use a specific AWS profile |
| `default` | Value to use if object not found (framework-handled) |
| `sensitive` | Mark value as sensitive (default: false) |

## AWS Authentication

All resolvers use the standard AWS credential chain (via the AWS SDK for Rust):

1. Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
2. Shared credential file (`~/.aws/credentials`)
3. AWS config file (`~/.aws/config`)
4. IAM role (when running on EC2/ECS/Lambda)

## Manual Registration

Resolvers are automatically registered when you import `holoconf` (via entry point discovery). For manual control:

```python
from holoconf_aws import register_ssm, register_cfn, register_s3, register_all

# Register individual resolvers
register_ssm()
register_cfn()
register_s3()

# Or register all at once
register_all()

# Force re-registration (overwrites existing)
register_all(force=True)
```

## License

MIT OR Apache-2.0

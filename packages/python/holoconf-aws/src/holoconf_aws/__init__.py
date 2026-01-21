"""AWS resolvers for holoconf.

This package provides AWS-specific resolvers for holoconf configuration.

## SSM Parameter Store Resolver

The `ssm` resolver fetches values from AWS Systems Manager Parameter Store.

```yaml
database:
    password: ${ssm:/app/prod/db-password}
```

## CloudFormation Resolver

The `cfn` resolver fetches outputs from CloudFormation stacks.

```yaml
database:
    endpoint: ${cfn:my-database-stack/DatabaseEndpoint}
```

## S3 Resolver

The `s3` resolver fetches objects from Amazon S3.

```yaml
config: ${s3:my-bucket/configs/app.yaml}
```

## Configuration API

Configure AWS client defaults for testing with moto/LocalStack and advanced use cases:

```python
import holoconf_aws

# Global configuration (applies to all AWS services)
holoconf_aws.configure(region="us-east-1", profile="prod")

# Service-specific configuration (overrides global defaults)
holoconf_aws.s3(endpoint="http://localhost:5000", region="us-west-2")
holoconf_aws.ssm(endpoint="http://localhost:5001")
holoconf_aws.cfn(profile="testing")

# Reset all configuration and clear client cache
holoconf_aws.reset()
```

Configuration precedence (highest to lowest):
1. Resolver kwargs in config file (e.g., `${s3:bucket/file,region=us-east-1}`)
2. Service-specific configuration (`holoconf_aws.s3()`, `ssm()`, `cfn()`)
3. Global configuration (`holoconf_aws.configure()`)
4. AWS SDK defaults (environment variables, credentials file)

Calling configuration functions with `None` leaves existing values unchanged.
Use `reset()` to clear all configuration for test isolation.

## Setup

AWS resolvers are automatically registered when holoconf is imported, via
entry point discovery. Just install holoconf-aws and the resolvers become
available:

```python
import holoconf  # Auto-discovers and registers holoconf-aws

config = holoconf.Config.loads("secret: ${ssm:/app/password}")
```

You can also register manually if needed:

```python
from holoconf_aws import register_ssm, register_cfn, register_s3
register_ssm()
register_cfn()
register_s3()
```
"""

from holoconf_aws._holoconf_aws import (
    cfn,
    configure,
    register_all,
    register_cfn,
    register_s3,
    register_ssm,
    reset,
    s3,
    ssm,
)

__all__ = [
    "cfn",
    "configure",
    "register_all",
    "register_cfn",
    "register_s3",
    "register_ssm",
    "reset",
    "s3",
    "ssm",
]

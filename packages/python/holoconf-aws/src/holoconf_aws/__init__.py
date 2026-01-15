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

### Setup

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

from holoconf_aws._holoconf_aws import register_all, register_cfn, register_s3, register_ssm

__all__ = ["register_all", "register_cfn", "register_s3", "register_ssm"]

"""AWS resolvers for holoconf.

This package provides AWS-specific resolvers for holoconf configuration.

## SSM Parameter Store Resolver

The `ssm` resolver fetches values from AWS Systems Manager Parameter Store.

```yaml
database:
    password: ${ssm:/app/prod/db-password}
```

### Setup

AWS resolvers are automatically registered when holoconf is imported, via
entry point discovery. Just install holoconf-aws and the `ssm` resolver
becomes available:

```python
import holoconf  # Auto-discovers and registers holoconf-aws

config = holoconf.Config.loads("secret: ${ssm:/app/password}")
```

You can also register manually if needed:

```python
from holoconf_aws import register_ssm
register_ssm()
```
"""

from holoconf_aws._holoconf_aws import register_all, register_ssm

__all__ = ["register_all", "register_ssm"]

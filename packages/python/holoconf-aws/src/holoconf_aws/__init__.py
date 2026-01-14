"""AWS resolvers for holoconf.

This package provides AWS-specific resolvers for holoconf configuration.

## SSM Parameter Store Resolver

The `ssm` resolver fetches values from AWS Systems Manager Parameter Store.

```yaml
database:
    password: ${ssm:/app/prod/db-password}
```

### Setup

Import this package to automatically register AWS resolvers:

```python
import holoconf_aws  # Registers SSM resolver automatically
```

Or register manually:

```python
from holoconf_aws import register_ssm
register_ssm()
```
"""

from holoconf_aws.ssm import SsmResolver, register_ssm

__all__ = ["SsmResolver", "register_ssm"]

# Auto-register on import for convenience
register_ssm()

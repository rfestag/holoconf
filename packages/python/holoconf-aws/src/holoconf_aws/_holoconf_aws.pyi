"""Type stubs for holoconf-aws Rust bindings."""

def register_ssm(force: bool = False) -> None:
    """Register the SSM resolver in the global registry.

    This makes the SSM resolver available to all Config instances.

    Args:
        force: If True, overwrite any existing 'ssm' resolver.
               If False (default), only register if not already registered.
    """
    ...

def register_cfn(force: bool = False) -> None:
    """Register the CloudFormation resolver in the global registry.

    This makes the `cfn` resolver available to all Config instances.

    Args:
        force: If True, overwrite any existing 'cfn' resolver.
               If False (default), only register if not already registered.
    """
    ...

def register_s3(force: bool = False) -> None:
    """Register the S3 resolver in the global registry.

    This makes the `s3` resolver available to all Config instances.

    Args:
        force: If True, overwrite any existing 's3' resolver.
               If False (default), only register if not already registered.
    """
    ...

def register_all(force: bool = False) -> None:
    """Register all AWS resolvers in the global registry.

    Registers:
    - ssm: AWS Systems Manager Parameter Store resolver
    - cfn: AWS CloudFormation outputs resolver
    - s3: AWS S3 object resolver

    Args:
        force: If True, overwrite any existing resolvers.
               If False (default), only register if not already registered.
    """
    ...

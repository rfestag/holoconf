"""Type stubs for holoconf-aws Rust bindings."""

def register_ssm(force: bool = False) -> None:
    """Register the SSM resolver in the global registry.

    This makes the SSM resolver available to all Config instances.

    Args:
        force: If True, overwrite any existing 'ssm' resolver.
               If False (default), only register if not already registered.
    """
    ...

def register_all(force: bool = False) -> None:
    """Register all AWS resolvers in the global registry.

    Currently registers:
    - ssm: AWS Systems Manager Parameter Store resolver

    Args:
        force: If True, overwrite any existing resolvers.
               If False (default), only register if not already registered.
    """
    ...

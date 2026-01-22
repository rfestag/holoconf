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

def configure(region: str | None = None, profile: str | None = None) -> None:
    """Configure global defaults for all AWS services.

    Sets default region and profile that apply to all AWS services (S3, SSM, CloudFormation).
    These can be overridden by service-specific configuration or per-resolver kwargs.

    Configuration precedence (highest to lowest):
    1. Resolver kwargs in config file (e.g., ${s3:bucket/file,region=us-east-1})
    2. Service-specific configuration (holoconf_aws.s3(), ssm(), cfn())
    3. Global configuration (holoconf_aws.configure())
    4. AWS SDK defaults (environment variables, credentials file)

    Args:
        region: Default AWS region (e.g., "us-east-1"). Pass None to leave unchanged.
        profile: Default AWS profile name. Pass None to leave unchanged.

    Example:
        >>> import holoconf_aws
        >>> holoconf_aws.configure(region="us-east-1", profile="prod")
    """
    ...

def s3(
    endpoint: str | None = None,
    region: str | None = None,
    profile: str | None = None,
) -> None:
    """Configure S3-specific defaults.

    Overrides global configuration for the S3 resolver.

    Args:
        endpoint: S3 endpoint URL (for moto/LocalStack, e.g., "http://localhost:5000").
                  Pass None to leave unchanged.
        region: AWS region (overrides global region). Pass None to leave unchanged.
        profile: AWS profile name (overrides global profile). Pass None to leave unchanged.

    Example:
        >>> import holoconf_aws
        >>> holoconf_aws.s3(endpoint="http://localhost:5000", region="us-west-2")
    """
    ...

def ssm(
    endpoint: str | None = None,
    region: str | None = None,
    profile: str | None = None,
) -> None:
    """Configure SSM-specific defaults.

    Overrides global configuration for the SSM resolver.

    Args:
        endpoint: SSM endpoint URL (for moto/LocalStack, e.g., "http://localhost:5001").
                  Pass None to leave unchanged.
        region: AWS region (overrides global region). Pass None to leave unchanged.
        profile: AWS profile name (overrides global profile). Pass None to leave unchanged.

    Example:
        >>> import holoconf_aws
        >>> holoconf_aws.ssm(endpoint="http://localhost:5001")
    """
    ...

def cfn(
    endpoint: str | None = None,
    region: str | None = None,
    profile: str | None = None,
) -> None:
    """Configure CloudFormation-specific defaults.

    Overrides global configuration for the CloudFormation resolver.

    Args:
        endpoint: CloudFormation endpoint URL (for moto/LocalStack, e.g., "http://localhost:5002").
                  Pass None to leave unchanged.
        region: AWS region (overrides global region). Pass None to leave unchanged.
        profile: AWS profile name (overrides global profile). Pass None to leave unchanged.

    Example:
        >>> import holoconf_aws
        >>> holoconf_aws.cfn(profile="testing")
    """
    ...

def reset() -> None:
    """Reset all configuration and clear the client cache.

    Clears all global and service-specific configuration, and removes all cached AWS clients.
    Useful for test isolation.

    Example:
        >>> import holoconf_aws
        >>> holoconf_aws.s3(endpoint="http://localhost:5000")
        >>> # ... run tests ...
        >>> holoconf_aws.reset()  # Clean up for next test
    """
    ...

"""SSM Parameter Store resolver for holoconf.

Provides the `ssm` resolver for fetching values from AWS Systems Manager Parameter Store.

## Usage

```yaml
database:
    host: ${ssm:/app/prod/db-host}
    password: ${ssm:/app/prod/db-password}
```

## Parameter Types

- **String**: Returned as-is
- **SecureString**: Automatically marked as sensitive for redaction
- **StringList**: Returned as an array (split by comma)

## Kwargs

- `region`: Override the AWS region for this parameter
- `profile`: Use a specific AWS profile
- `default`: Value to use if parameter not found (framework-handled)
- `sensitive`: Override automatic sensitivity detection (framework-handled)
"""

from typing import Any

import boto3
from botocore.exceptions import ClientError

import holoconf

_registered = False


class SsmResolver:
    """SSM Parameter Store resolver.

    Fetches values from AWS Systems Manager Parameter Store.

    Attributes:
        name: The resolver name ("ssm")
    """

    name = "ssm"

    def __init__(self, session: boto3.Session | None = None):
        """Create a new SSM resolver.

        Args:
            session: Optional boto3 session to use. If not provided, a default
                    session will be created.
        """
        self._session = session

    def __call__(self, path: str, **kwargs: Any) -> Any:
        """Resolve an SSM parameter.

        Args:
            path: The SSM parameter path (must start with /)
            **kwargs: Optional resolver kwargs:
                - region: AWS region override
                - profile: AWS profile to use

        Returns:
            The parameter value:
            - String: returned as-is
            - SecureString: returned as ResolvedValue with sensitive=True
            - StringList: returned as a list

        Raises:
            KeyError: If the parameter is not found (triggers default handling)
            ValueError: If the path doesn't start with /
        """
        # Validate path format
        if not path.startswith("/"):
            raise ValueError(f"SSM parameter path must start with /: {path}")

        # Build session with optional overrides
        session = self._get_session(kwargs.get("region"), kwargs.get("profile"))
        client = session.client("ssm")

        try:
            response = client.get_parameter(Name=path, WithDecryption=True)
        except ClientError as e:
            error_code = e.response.get("Error", {}).get("Code", "")
            if error_code == "ParameterNotFound":
                # Raise KeyError to trigger default handling at framework level
                raise KeyError(f"SSM parameter not found: {path}") from e
            raise

        parameter = response.get("Parameter", {})
        value = parameter.get("Value", "")
        param_type = parameter.get("Type", "String")

        # Handle different parameter types
        if param_type == "SecureString":
            # SecureString is automatically sensitive
            return holoconf.ResolvedValue(value, sensitive=True)
        elif param_type == "StringList":
            # StringList is comma-separated, convert to array
            return value.split(",")
        else:
            # Regular string
            return value

    def _get_session(
        self, region: str | None = None, profile: str | None = None
    ) -> boto3.Session:
        """Get a boto3 session with optional overrides."""
        if self._session is not None and region is None and profile is None:
            return self._session

        session_kwargs = {}
        if region:
            session_kwargs["region_name"] = region
        if profile:
            session_kwargs["profile_name"] = profile

        return boto3.Session(**session_kwargs) if session_kwargs else boto3.Session()


def register_ssm(force: bool = False) -> None:
    """Register the SSM resolver with holoconf.

    Args:
        force: If True, overwrite any existing 'ssm' resolver.
              If False (default), only register if not already registered.

    This function is idempotent - calling it multiple times with force=False
    will only register the resolver once.

    Note: The 'ssm' resolver is automatically registered when you import
    the holoconf_aws package, so you typically don't need to call this directly.
    """
    global _registered

    if _registered and not force:
        return

    resolver = SsmResolver()
    holoconf.register_resolver("ssm", resolver, force=force)
    _registered = True

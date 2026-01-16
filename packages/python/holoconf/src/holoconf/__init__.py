"""
holoconf - Cross-language configuration library with resolver support

This module provides a configuration library that supports:
- YAML and JSON configuration files
- Environment variable interpolation: ${env:VAR}
- Self-references: ${path.to.value}
- File includes: ${file:./other.yaml}
- JSON Schema validation
- Type coercion with schema support
- Plugin-based resolver extensions via entry points

Exception Hierarchy:
    HoloconfError (base)
    ├── ParseError - YAML/JSON syntax errors
    ├── ValidationError - Schema validation failures
    ├── ResolverError - Resolution failures (missing env vars, etc.)
    ├── PathNotFoundError - Requested config path doesn't exist
    ├── CircularReferenceError - Circular reference in config
    └── TypeCoercionError - Type conversion failures

Plugin Discovery:
    Resolver plugins are automatically discovered and registered when holoconf
    is imported. Plugins use the "holoconf.resolvers" entry point group:

        [project.entry-points."holoconf.resolvers"]
        ssm = "holoconf_aws:register_ssm"

    The discover_plugins() function can be called manually to re-discover plugins.
"""

import logging
import sys

from holoconf._holoconf import (
    CircularReferenceError,
    # Classes
    Config,
    # Exceptions
    HoloconfError,
    ParseError,
    PathNotFoundError,
    ResolvedValue,
    ResolverError,
    Schema,
    TypeCoercionError,
    ValidationError,
    # Functions
    register_resolver,
)

_logger = logging.getLogger(__name__)


def discover_plugins() -> list[str]:
    """Discover and load resolver plugins via entry points.

    This function discovers all installed packages that provide resolver plugins
    via the "holoconf.resolvers" entry point group, and calls their registration
    functions.

    This is called automatically when holoconf is imported. You can call it
    manually to re-discover plugins if new ones are installed at runtime.

    Returns:
        A list of successfully loaded plugin names.

    Example:
        >>> import holoconf
        >>> loaded = holoconf.discover_plugins()
        >>> print(f"Loaded plugins: {loaded}")
        Loaded plugins: ['ssm']

    Note:
        Plugins should define entry points in their pyproject.toml:

        [project.entry-points."holoconf.resolvers"]
        ssm = "holoconf_aws:register_ssm"
    """
    loaded = []

    # Use importlib.metadata for Python 3.9+
    if sys.version_info >= (3, 10):
        from importlib.metadata import entry_points

        eps = entry_points(group="holoconf.resolvers")
    else:
        from importlib.metadata import entry_points

        all_eps = entry_points()
        eps = all_eps.get("holoconf.resolvers", [])

    for ep in eps:
        try:
            register_func = ep.load()
            register_func()
            loaded.append(ep.name)
        except Exception as e:
            _logger.warning(
                "Failed to load holoconf plugin '%s' from '%s': %s",
                ep.name,
                ep.value,
                e,
            )

    return loaded


# Auto-discover plugins on import
discover_plugins()

__version__ = "0.1.0"
__all__ = [
    "CircularReferenceError",
    "Config",
    "HoloconfError",
    "ParseError",
    "PathNotFoundError",
    "ResolvedValue",
    "ResolverError",
    "Schema",
    "TypeCoercionError",
    "ValidationError",
    "__version__",
    "discover_plugins",
    "register_resolver",
]

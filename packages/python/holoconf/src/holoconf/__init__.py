"""
holoconf - Cross-language configuration library with resolver support

This module provides a configuration library that supports:
- YAML and JSON configuration files
- Environment variable interpolation: ${env:VAR}
- Self-references: ${path.to.value}
- File includes: ${file:./other.yaml}
- JSON Schema validation
- Type coercion with schema support

Exception Hierarchy:
    HoloconfError (base)
    ├── ParseError - YAML/JSON syntax errors
    ├── ValidationError - Schema validation failures
    ├── ResolverError - Resolution failures (missing env vars, etc.)
    ├── PathNotFoundError - Requested config path doesn't exist
    ├── CircularReferenceError - Circular reference in config
    └── TypeCoercionError - Type conversion failures
"""

from holoconf._holoconf import (
    CircularReferenceError,
    # Classes
    Config,
    # Exceptions
    HoloconfError,
    ParseError,
    PathNotFoundError,
    ResolverError,
    Schema,
    TypeCoercionError,
    ValidationError,
)

__version__ = "0.1.0"
__all__ = [
    "CircularReferenceError",
    # Classes
    "Config",
    # Exceptions
    "HoloconfError",
    "ParseError",
    "PathNotFoundError",
    "ResolverError",
    "Schema",
    "TypeCoercionError",
    "ValidationError",
    # Metadata
    "__version__",
]

"""
holoconf - Cross-language configuration library with resolver support

This module provides a configuration library that supports:
- YAML and JSON configuration files
- Environment variable interpolation: ${env:VAR}
- Self-references: ${path.to.value}
- File includes: ${file:./other.yaml}
- Type coercion with schema support
"""

from holoconf._holoconf import Config

__version__ = "0.1.0"
__all__ = ["Config", "__version__"]

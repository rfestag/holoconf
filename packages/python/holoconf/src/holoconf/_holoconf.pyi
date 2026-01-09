"""Python type stubs for the holoconf native extension module.

This module provides type hints for the Rust PyO3 bindings.
"""

from typing import Any

class Config:
    """Configuration object for loading and accessing configuration values.

    The Config class is the main entry point for holoconf. It provides methods
    for loading configuration from files or strings, accessing values with
    automatic interpolation resolution, and exporting configuration in various
    formats.

    Example:
        >>> config = Config.load("config.yaml")
        >>> host = config.get("database.host")
        >>> port = config.get_int("database.port")
    """

    @staticmethod
    def load(path: str, allow_http: bool = False) -> Config:
        """Load configuration from a YAML file.

        Args:
            path: Path to the YAML file
            allow_http: Enable HTTP resolver (disabled by default for security)

        Returns:
            A new Config object

        Raises:
            ParseError: If the file cannot be parsed
            HoloconfError: If the file cannot be read
        """
        ...

    @staticmethod
    def loads(yaml: str, base_path: str | None = None, allow_http: bool = False) -> Config:
        """Load configuration from a YAML string.

        Args:
            yaml: YAML content as a string
            base_path: Optional base path for resolving relative file references
            allow_http: Enable HTTP resolver (disabled by default for security)

        Returns:
            A new Config object

        Raises:
            ParseError: If the YAML is invalid
        """
        ...

    @staticmethod
    def from_json(json: str) -> Config:
        """Load configuration from a JSON string.

        Args:
            json: JSON content as a string

        Returns:
            A new Config object

        Raises:
            ParseError: If the JSON is invalid
        """
        ...

    @staticmethod
    def load_merged(paths: list[str]) -> Config:
        """Load and merge multiple YAML files.

        Files are merged in order, with later files overriding earlier ones.

        Args:
            paths: List of paths to YAML files

        Returns:
            A new Config object with merged configuration

        Raises:
            ParseError: If any file cannot be parsed
            HoloconfError: If any file cannot be read
        """
        ...

    def get(self, path: str) -> Any:
        """Get a resolved value by path (e.g., "database.host").

        Interpolations like ${env:VAR} are resolved before returning.

        Args:
            path: Dot-separated path to the value

        Returns:
            The resolved value (str, int, float, bool, list, dict, or None)

        Raises:
            PathNotFoundError: If the path doesn't exist
            ResolverError: If resolution fails (e.g., missing env var)
        """
        ...

    def get_raw(self, path: str) -> Any:
        """Get the raw (unresolved) value by path.

        Returns the value without resolving interpolations.
        Useful for debugging or inspecting the raw configuration.

        Args:
            path: Dot-separated path to the value

        Returns:
            The raw value (may contain ${...} interpolation syntax)

        Raises:
            PathNotFoundError: If the path doesn't exist
        """
        ...

    def get_string(self, path: str) -> str:
        """Get a string value, with type coercion if needed.

        Args:
            path: Dot-separated path to the value

        Returns:
            The value as a string

        Raises:
            TypeCoercionError: If the value cannot be converted to string
        """
        ...

    def get_int(self, path: str) -> int:
        """Get an integer value, with type coercion if needed.

        String values like "42" will be parsed as integers.

        Args:
            path: Dot-separated path to the value

        Returns:
            The value as an integer

        Raises:
            TypeCoercionError: If the value cannot be converted to integer
        """
        ...

    def get_float(self, path: str) -> float:
        """Get a float value, with type coercion if needed.

        String values like "3.14" will be parsed as floats.

        Args:
            path: Dot-separated path to the value

        Returns:
            The value as a float

        Raises:
            TypeCoercionError: If the value cannot be converted to float
        """
        ...

    def get_bool(self, path: str) -> bool:
        """Get a boolean value, with strict coercion.

        Only "true" and "false" (case-insensitive) are accepted for string coercion.

        Args:
            path: Dot-separated path to the value

        Returns:
            The value as a boolean

        Raises:
            TypeCoercionError: If the value cannot be converted to boolean
        """
        ...

    def to_dict(self, resolve: bool = True, redact: bool = False) -> dict[str, Any]:
        """Export the configuration as a Python dict.

        Args:
            resolve: If True (default), resolve all interpolations. If False, return raw values.
            redact: If True (default False), redact sensitive values with "[REDACTED]"

        Returns:
            The configuration as a Python dictionary
        """
        ...

    def to_yaml(self, resolve: bool = True, redact: bool = False) -> str:
        """Export the configuration as YAML.

        Args:
            resolve: If True (default), resolve all interpolations. If False, return raw values.
            redact: If True (default False), redact sensitive values with "[REDACTED]"

        Returns:
            The configuration as a YAML string
        """
        ...

    def to_json(self, resolve: bool = True, redact: bool = False) -> str:
        """Export the configuration as JSON.

        Args:
            resolve: If True (default), resolve all interpolations. If False, return raw values.
            redact: If True (default False), redact sensitive values with "[REDACTED]"

        Returns:
            The configuration as a JSON string
        """
        ...

    def merge(self, other: Config) -> None:
        """Merge another config into this one.

        The other config's values override this config's values.

        Args:
            other: Another Config to merge into this one
        """
        ...

    def resolve_all(self) -> None:
        """Resolve all values eagerly.

        By default, values are resolved lazily when accessed. This method
        forces resolution of all values upfront, which can be useful for
        detecting errors early or for performance when all values are needed.

        Raises:
            ResolverError: If any value fails to resolve
        """
        ...

    def clear_cache(self) -> None:
        """Clear the resolution cache.

        Resolved values are cached for performance. Call this method to clear
        the cache, for example after environment variables have changed.
        """
        ...

    def validate(self, schema: Schema) -> None:
        """Validate the resolved configuration against a schema.

        This resolves all values first, then validates the resolved values
        against the schema, checking types, constraints, and patterns.

        Args:
            schema: A Schema object to validate against

        Raises:
            ValidationError: If validation fails
            ResolverError: If resolution fails
        """
        ...

    def validate_raw(self, schema: Schema) -> None:
        """Validate the raw (unresolved) configuration against a schema.

        This performs structural validation before resolution, checking that
        required keys exist and the configuration structure matches the schema.
        Interpolation placeholders (${...}) are allowed as valid values.

        Args:
            schema: A Schema object to validate against

        Raises:
            ValidationError: If validation fails
        """
        ...

    def validate_collect(self, schema: Schema) -> list[str]:
        """Validate and collect all errors (instead of failing on first).

        Args:
            schema: A Schema object to validate against

        Returns:
            A list of error message strings (empty if valid)
        """
        ...

    def __getitem__(self, key: str) -> Any:
        """Dict-like access: config["key"]."""
        ...

    def __getattr__(self, name: str) -> Any:
        """Attribute access: config.key."""
        ...

class Schema:
    """Schema for validating configuration against JSON Schema.

    The Schema class loads JSON Schema definitions from files or strings,
    and is used with Config.validate() to validate configuration values.

    Example:
        >>> schema = Schema.load("schema.json")
        >>> config.validate(schema)
    """

    @staticmethod
    def load(path: str) -> Schema:
        """Load a schema from a file (JSON or YAML based on extension).

        Args:
            path: Path to the schema file (.yaml, .yml, or .json)

        Returns:
            A new Schema object

        Raises:
            ParseError: If the file cannot be parsed
            HoloconfError: If the file cannot be read
        """
        ...

    @staticmethod
    def from_yaml(yaml: str) -> Schema:
        """Load a schema from a YAML string.

        Args:
            yaml: JSON Schema as a YAML string

        Returns:
            A new Schema object

        Raises:
            ParseError: If the YAML is invalid or not a valid JSON Schema
        """
        ...

    @staticmethod
    def from_json(json: str) -> Schema:
        """Load a schema from a JSON string.

        Args:
            json: JSON Schema as a JSON string

        Returns:
            A new Schema object

        Raises:
            ParseError: If the JSON is invalid or not a valid JSON Schema
        """
        ...

class HoloconfError(Exception):
    """Base exception for all holoconf errors.

    Catch this exception to handle any holoconf-related error.

    Example:
        >>> try:
        ...     config = Config.load("config.yaml")
        ... except HoloconfError as e:
        ...     print(f"Configuration error: {e}")
    """

    ...

class ParseError(HoloconfError):
    """Error parsing configuration (YAML/JSON syntax).

    Raised when YAML or JSON content cannot be parsed due to syntax errors,
    malformed content, or encoding issues.
    """

    ...

class ValidationError(HoloconfError):
    """Schema validation error.

    Raised when configuration fails to validate against a JSON Schema.
    Common causes include missing required fields, type mismatches,
    and constraint violations.
    """

    ...

class ResolverError(HoloconfError):
    """Error during value resolution.

    Raised when a resolver fails during value resolution. Common causes
    include missing environment variables (without defaults), file not found
    for file resolver, HTTP request failures, or invalid resolver syntax.
    """

    ...

class PathNotFoundError(HoloconfError):
    """Requested path does not exist in configuration.

    Raised when attempting to access a configuration path that doesn't exist.
    Check for typos in the path name, missing configuration sections,
    or incorrect path separators (use '.' not '/').
    """

    ...

class CircularReferenceError(HoloconfError):
    """Circular reference detected in configuration.

    Raised when a circular reference is detected during value resolution.
    This occurs when interpolations form a cycle, such as a value
    referencing itself or indirect circular dependencies.
    """

    ...

class TypeCoercionError(HoloconfError):
    """Failed to coerce value to requested type.

    Raised when a value cannot be converted to the requested type.
    For example, calling get_int() on a non-numeric string,
    or get_bool() on a string other than "true"/"false".
    """

    ...

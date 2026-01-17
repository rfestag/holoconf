"""Python type stubs for the holoconf native extension module.

This module provides type hints for the Rust PyO3 bindings.
"""

from typing import Any, Callable

def register_resolver(name: str, func: Callable[..., Any], force: bool = False) -> None:
    """Register a resolver in the global registry.

    This makes the resolver available to all Config instances created after registration.
    Use this for extension packages that provide additional resolvers.

    Args:
        name: The resolver name (used as ${name:...} in config)
        func: A callable that takes (*args, **kwargs) and returns a value
        force: If True, overwrite any existing resolver with the same name.
               If False (default), raise an error if the name is already registered.

    Example:
        >>> import holoconf
        >>>
        >>> def ssm_resolver(path, region=None, profile=None):
        ...     # Implementation here
        ...     return value
        >>>
        >>> holoconf.register_resolver("ssm", ssm_resolver)
        >>> # Now any Config can use ${ssm:/my/param}
    """
    ...

class ResolvedValue:
    """A resolved value with optional sensitivity metadata.

    Use this to return sensitive values from custom resolvers that
    should be redacted when using config.to_yaml(redact=True).

    Example:
        >>> def secret_resolver(key):
        ...     value = fetch_secret(key)
        ...     return ResolvedValue(value, sensitive=True)
    """

    def __init__(self, value: Any, sensitive: bool = False) -> None:
        """Create a resolved value.

        Args:
            value: The resolved value
            sensitive: Whether the value should be redacted in output (default False)
        """
        ...

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
    def load(
        path: str,
        schema: str | None = None,
        allow_http: bool = False,
        http_allowlist: list[str] | None = None,
        http_proxy: str | None = None,
        http_proxy_from_env: bool = False,
        http_ca_bundle: str | None = None,
        http_extra_ca_bundle: str | None = None,
        http_client_cert: str | None = None,
        http_client_key: str | None = None,
        http_client_key_password: str | None = None,
        http_insecure: bool = False,
    ) -> Config:
        """Load configuration from a YAML file (required - errors if missing).

        This is the primary way to load configuration. Use `Config.optional()`
        for files that may not exist.

        Args:
            path: Path to the YAML file
            schema: Optional path to a JSON Schema file. If provided, schema defaults
                   will be used when accessing missing paths.
            allow_http: Enable HTTP resolver (disabled by default for security)
            http_allowlist: List of URL patterns to allow (glob-style)
            http_proxy: Proxy URL (e.g., "http://proxy:8080" or "socks5://proxy:1080")
            http_proxy_from_env: Auto-detect proxy from HTTP_PROXY/HTTPS_PROXY env vars
            http_ca_bundle: Path to CA bundle PEM file (replaces default roots)
            http_extra_ca_bundle: Path to extra CA bundle PEM file (adds to default roots)
            http_client_cert: Path to client certificate (PEM or P12/PFX) for mTLS
            http_client_key: Path to client private key PEM (not needed for P12/PFX)
            http_client_key_password: Password for encrypted key or P12/PFX file
            http_insecure: DANGEROUS - Skip TLS certificate verification

        Returns:
            A new Config object

        Raises:
            HoloconfError: If the file cannot be read or doesn't exist
            ParseError: If the file cannot be parsed

        Example:
            >>> config = Config.load("config.yaml", schema="schema.yaml")
            >>> config.pool_size  # Returns schema default if not in config
        """
        ...

    @staticmethod
    def required(
        path: str,
        schema: str | None = None,
        allow_http: bool = False,
        http_allowlist: list[str] | None = None,
        http_proxy: str | None = None,
        http_proxy_from_env: bool = False,
        http_ca_bundle: str | None = None,
        http_extra_ca_bundle: str | None = None,
        http_client_cert: str | None = None,
        http_client_key: str | None = None,
        http_client_key_password: str | None = None,
        http_insecure: bool = False,
    ) -> Config:
        """Alias for `load()` - load a required config file.

        Provided for symmetry with `Config.optional()`.

        Args:
            path: Path to the YAML file
            schema: Optional path to a JSON Schema file
            allow_http: Enable HTTP resolver (disabled by default for security)
            http_allowlist: List of URL patterns to allow (glob-style)
            http_proxy: Proxy URL (e.g., "http://proxy:8080" or "socks5://proxy:1080")
            http_proxy_from_env: Auto-detect proxy from HTTP_PROXY/HTTPS_PROXY env vars
            http_ca_bundle: Path to CA bundle PEM file (replaces default roots)
            http_extra_ca_bundle: Path to extra CA bundle PEM file (adds to default roots)
            http_client_cert: Path to client certificate (PEM or P12/PFX) for mTLS
            http_client_key: Path to client private key PEM (not needed for P12/PFX)
            http_client_key_password: Password for encrypted key or P12/PFX file
            http_insecure: DANGEROUS - Skip TLS certificate verification

        Returns:
            A new Config object

        Raises:
            HoloconfError: If the file cannot be read or doesn't exist
            ParseError: If the file cannot be parsed
        """
        ...

    @staticmethod
    def optional(path: str) -> Config:
        """Load an optional configuration file.

        Returns an empty Config if the file doesn't exist.
        Use this for configuration files that may or may not be present,
        such as local overrides.

        Args:
            path: Path to the config file

        Returns:
            A Config object (empty if file doesn't exist)

        Example:
            >>> base = Config.load("base.yaml")
            >>> local = Config.optional("local.yaml")
            >>> base.merge(local)
        """
        ...

    @staticmethod
    def loads(
        yaml: str,
        base_path: str | None = None,
        allow_http: bool = False,
        http_allowlist: list[str] | None = None,
        http_proxy: str | None = None,
        http_proxy_from_env: bool = False,
        http_ca_bundle: str | None = None,
        http_extra_ca_bundle: str | None = None,
        http_client_cert: str | None = None,
        http_client_key: str | None = None,
        http_client_key_password: str | None = None,
        http_insecure: bool = False,
    ) -> Config:
        """Load configuration from a YAML string.

        Args:
            yaml: YAML content as a string
            base_path: Optional base path for resolving relative file references
            allow_http: Enable HTTP resolver (disabled by default for security)
            http_allowlist: List of URL patterns to allow (glob-style)
            http_proxy: Proxy URL (e.g., "http://proxy:8080" or "socks5://proxy:1080")
            http_proxy_from_env: Auto-detect proxy from HTTP_PROXY/HTTPS_PROXY env vars
            http_ca_bundle: Path to CA bundle PEM file (replaces default roots)
            http_extra_ca_bundle: Path to extra CA bundle PEM file (adds to default roots)
            http_client_cert: Path to client certificate (PEM or P12/PFX) for mTLS
            http_client_key: Path to client private key PEM (not needed for P12/PFX)
            http_client_key_password: Password for encrypted key or P12/PFX file
            http_insecure: DANGEROUS - Skip TLS certificate verification

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

        Binary data (from file resolver with encoding=binary) is returned as Python bytes objects.

        Args:
            resolve: If True (default), resolve all interpolations. If False, return raw values.
            redact: If True (default False), redact sensitive values with "[REDACTED]"

        Returns:
            The configuration as a Python dictionary. Values may include bytes objects.
        """
        ...

    def to_yaml(self, resolve: bool = True, redact: bool = False) -> str:
        """Export the configuration as YAML.

        Binary data (from file resolver with encoding=binary) is serialized as base64 strings.

        Args:
            resolve: If True (default), resolve all interpolations. If False, return raw values.
            redact: If True (default False), redact sensitive values with "[REDACTED]"

        Returns:
            The configuration as a YAML string
        """
        ...

    def to_json(self, resolve: bool = True, redact: bool = False) -> str:
        """Export the configuration as JSON.

        Binary data (from file resolver with encoding=binary) is serialized as base64 strings.

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

    def get_source(self, path: str) -> str | None:
        """Get the source file for a config path.

        Returns the filename of the config file that provided this value.
        For merged configs, this returns the file that "won" for this path.

        Args:
            path: The config path (e.g., "database.host")

        Returns:
            The filename or None if source tracking is not available
        """
        ...

    def dump_sources(self) -> dict[str, str]:
        """Get all source mappings.

        Returns a dict mapping config paths to their source filenames.
        Useful for debugging which file each value came from.

        Returns:
            A dict of {path: filename} entries
        """
        ...

    def set_schema(self, schema: Schema) -> None:
        """Attach a schema to this config for default value lookup.

        When a schema is attached, accessing a missing path will return the
        schema's default value (if defined) instead of raising PathNotFoundError.

        Args:
            schema: A Schema object to attach

        Example:
            >>> config = Config.load("config.yaml")
            >>> schema = Schema.load("schema.yaml")
            >>> config.set_schema(schema)
            >>> config.pool_size  # Returns schema default if not in config
        """
        ...

    def get_schema(self) -> Schema | None:
        """Get the attached schema, if any.

        Returns:
            The attached Schema object, or None if no schema is attached
        """
        ...

    def validate(self, schema: Schema | None = None) -> None:
        """Validate the resolved configuration against a schema.

        This resolves all values first, then validates the resolved values
        against the schema, checking types, constraints, and patterns.

        Args:
            schema: Optional Schema object to validate against. If not provided,
                   uses the attached schema (set via `set_schema()` or `load(schema=...)`).

        Raises:
            ValidationError: If validation fails
            ResolverError: If resolution fails
            HoloconfError: If no schema is provided and none is attached
        """
        ...

    def validate_raw(self, schema: Schema | None = None) -> None:
        """Validate the raw (unresolved) configuration against a schema.

        This performs structural validation before resolution, checking that
        required keys exist and the configuration structure matches the schema.
        Interpolation placeholders (${...}) are allowed as valid values.

        Args:
            schema: Optional Schema object to validate against. If not provided,
                   uses the attached schema (set via `set_schema()` or `load(schema=...)`).

        Raises:
            ValidationError: If validation fails
            HoloconfError: If no schema is provided and none is attached
        """
        ...

    def validate_collect(self, schema: Schema | None = None) -> list[str]:
        """Validate and collect all errors (instead of failing on first).

        Args:
            schema: Optional Schema object to validate against. If not provided,
                   uses the attached schema (set via `set_schema()` or `load(schema=...)`).

        Returns:
            A list of error message strings (empty if valid)

        Raises:
            HoloconfError: If no schema is provided and none is attached
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

//! Python bindings for holoconf
//!
//! This crate provides Python bindings for holoconf-core using PyO3.
//!
//! Exception hierarchy per ADR-008:
//! - HoloconfError (base)
//!   - ParseError
//!   - ValidationError
//!   - ResolverError
//!   - PathNotFoundError
//!   - CircularReferenceError
//!   - TypeCoercionError

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyTuple};

use std::collections::HashMap;
use std::sync::Arc;

use holoconf_core::{
    error::ErrorKind,
    resolver::{ResolvedValue as CoreResolvedValue, Resolver, ResolverContext},
    Config as CoreConfig, ConfigOptions, Error as CoreError, Schema as CoreSchema,
    Value as CoreValue,
};

// Define exception hierarchy per ADR-008
create_exception!(
    holoconf,
    HoloconfError,
    PyException,
    "Base exception for all holoconf errors"
);
create_exception!(
    holoconf,
    ParseError,
    HoloconfError,
    "Error parsing configuration (YAML/JSON syntax)"
);
create_exception!(
    holoconf,
    ValidationError,
    HoloconfError,
    "Schema validation error"
);
create_exception!(
    holoconf,
    ResolverError,
    HoloconfError,
    "Error during value resolution"
);
create_exception!(
    holoconf,
    PathNotFoundError,
    HoloconfError,
    "Requested path does not exist in configuration"
);
create_exception!(
    holoconf,
    CircularReferenceError,
    HoloconfError,
    "Circular reference detected in configuration"
);
create_exception!(
    holoconf,
    TypeCoercionError,
    HoloconfError,
    "Failed to coerce value to requested type"
);

/// Convert a holoconf error to the appropriate Python exception
fn to_py_err(err: CoreError) -> PyErr {
    let message = format!("{}", err);

    match &err.kind {
        ErrorKind::Parse => ParseError::new_err(message),
        ErrorKind::Validation => ValidationError::new_err(message),
        ErrorKind::PathNotFound => PathNotFoundError::new_err(message),
        ErrorKind::CircularReference => CircularReferenceError::new_err(message),
        ErrorKind::TypeCoercion => TypeCoercionError::new_err(message),
        ErrorKind::Resolver(_) => ResolverError::new_err(message),
        ErrorKind::Io => HoloconfError::new_err(message),
        ErrorKind::Internal => HoloconfError::new_err(message),
    }
}

/// Convert a CoreValue to a Python object
fn value_to_py(py: Python<'_>, value: &CoreValue) -> PyResult<PyObject> {
    match value {
        CoreValue::Null => Ok(py.None()),
        CoreValue::Bool(b) => Ok(b.into_pyobject(py)?.to_owned().unbind().into_any()),
        CoreValue::Integer(i) => Ok(i.into_pyobject(py)?.to_owned().unbind().into_any()),
        CoreValue::Float(f) => Ok(f.into_pyobject(py)?.to_owned().unbind().into_any()),
        CoreValue::String(s) => Ok(s.into_pyobject(py)?.to_owned().unbind().into_any()),
        CoreValue::Bytes(bytes) => Ok(PyBytes::new(py, bytes).unbind().into_any()),
        CoreValue::Sequence(seq) => {
            let list = PyList::empty(py);
            for item in seq {
                list.append(value_to_py(py, item)?)?;
            }
            Ok(list.unbind().into_any())
        }
        CoreValue::Mapping(map) => {
            let dict = PyDict::new(py);
            for (key, val) in map {
                dict.set_item(key, value_to_py(py, val)?)?;
            }
            Ok(dict.unbind().into_any())
        }
    }
}

/// Convert a Python object to a CoreValue
#[allow(clippy::only_used_in_recursion)]
fn py_to_value(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<CoreValue> {
    if obj.is_none() {
        return Ok(CoreValue::Null);
    }
    if let Ok(b) = obj.extract::<bool>() {
        return Ok(CoreValue::Bool(b));
    }
    if let Ok(i) = obj.extract::<i64>() {
        return Ok(CoreValue::Integer(i));
    }
    if let Ok(f) = obj.extract::<f64>() {
        return Ok(CoreValue::Float(f));
    }
    if let Ok(s) = obj.extract::<String>() {
        return Ok(CoreValue::String(s));
    }
    if let Ok(bytes) = obj.extract::<Vec<u8>>() {
        return Ok(CoreValue::Bytes(bytes));
    }
    if let Ok(list) = obj.downcast::<PyList>() {
        let mut seq = Vec::new();
        for item in list.iter() {
            seq.push(py_to_value(py, &item)?);
        }
        return Ok(CoreValue::Sequence(seq));
    }
    if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = indexmap::IndexMap::new();
        for (key, val) in dict.iter() {
            let key_str: String = key.extract()?;
            map.insert(key_str, py_to_value(py, &val)?);
        }
        return Ok(CoreValue::Mapping(map));
    }
    // Default to string representation
    let repr = obj.str()?.to_string();
    Ok(CoreValue::String(repr))
}

/// Python wrapper for ResolvedValue
///
/// Use this to return sensitive values from custom resolvers.
#[pyclass(name = "ResolvedValue")]
struct PyResolvedValue {
    value: PyObject,
    sensitive: bool,
}

#[pymethods]
impl PyResolvedValue {
    /// Create a resolved value
    ///
    /// Args:
    ///     value: The resolved value
    ///     sensitive: Whether the value should be redacted in output (default False)
    #[new]
    #[pyo3(signature = (value, sensitive=false))]
    fn new(value: PyObject, sensitive: bool) -> Self {
        Self { value, sensitive }
    }
}

/// A Python callable wrapped as a Rust Resolver
struct PyResolver {
    name: String,
    callable: PyObject,
}

impl PyResolver {
    fn new(name: String, callable: PyObject) -> Self {
        Self { name, callable }
    }
}

// Safety: PyResolver is Send + Sync because we acquire the GIL before calling Python
unsafe impl Send for PyResolver {}
unsafe impl Sync for PyResolver {}

impl Resolver for PyResolver {
    fn resolve(
        &self,
        args: &[String],
        kwargs: &HashMap<String, String>,
        _ctx: &ResolverContext,
    ) -> holoconf_core::error::Result<CoreResolvedValue> {
        Python::with_gil(|py| {
            // Convert args to Python tuple
            let py_args = PyTuple::new(py, args).map_err(|e| {
                CoreError::resolver_custom(&self.name, format!("Failed to convert args: {}", e))
            })?;

            // Convert kwargs to Python dict
            let py_kwargs = PyDict::new(py);
            for (k, v) in kwargs {
                py_kwargs.set_item(k, v).map_err(|e| {
                    CoreError::resolver_custom(&self.name, format!("Failed to set kwarg: {}", e))
                })?;
            }

            // Call the Python function
            let result = self
                .callable
                .call(py, py_args, Some(&py_kwargs))
                .map_err(|e| {
                    // Check if this is a KeyError (indicates "not found" condition)
                    // This enables framework-level default handling
                    if e.is_instance_of::<pyo3::exceptions::PyKeyError>(py) {
                        // Extract the resource name from the first arg if available
                        let resource = args
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "unknown".to_string());
                        CoreError::not_found(resource, None)
                    } else {
                        CoreError::resolver_custom(&self.name, format!("Resolver error: {}", e))
                    }
                })?;

            // Check if result is a coroutine (async function) and await it
            let inspect = py.import("inspect").map_err(|e| {
                CoreError::resolver_custom(&self.name, format!("Failed to import inspect: {}", e))
            })?;
            let is_coroutine = inspect
                .call_method1("iscoroutine", (result.bind(py),))
                .map_err(|e| {
                    CoreError::resolver_custom(
                        &self.name,
                        format!("Failed to check coroutine: {}", e),
                    )
                })?
                .extract::<bool>()
                .unwrap_or(false);

            let result = if is_coroutine {
                // Run the coroutine using asyncio.run()
                let asyncio = py.import("asyncio").map_err(|e| {
                    CoreError::resolver_custom(
                        &self.name,
                        format!("Failed to import asyncio: {}", e),
                    )
                })?;
                asyncio
                    .call_method1("run", (result.bind(py),))
                    .map_err(|e| {
                        // Check if this is a KeyError from the async function
                        if e.is_instance_of::<pyo3::exceptions::PyKeyError>(py) {
                            let resource = args
                                .first()
                                .cloned()
                                .unwrap_or_else(|| "unknown".to_string());
                            CoreError::not_found(resource, None)
                        } else {
                            CoreError::resolver_custom(
                                &self.name,
                                format!("Async resolver error: {}", e),
                            )
                        }
                    })?
                    .unbind()
            } else {
                result
            };

            // Convert result to CoreResolvedValue
            let result_bound = result.bind(py);

            // Check if result is a PyResolvedValue by downcasting
            if let Ok(resolved_cell) = result_bound.downcast::<PyResolvedValue>() {
                let resolved = resolved_cell.borrow();
                let value = py_to_value(py, resolved.value.bind(py)).map_err(|e| {
                    CoreError::resolver_custom(
                        &self.name,
                        format!("Failed to convert value: {}", e),
                    )
                })?;
                if resolved.sensitive {
                    Ok(CoreResolvedValue::sensitive(value))
                } else {
                    Ok(CoreResolvedValue::new(value))
                }
            } else {
                // Plain return value - convert directly
                let value = py_to_value(py, result_bound).map_err(|e| {
                    CoreError::resolver_custom(
                        &self.name,
                        format!("Failed to convert value: {}", e),
                    )
                })?;
                Ok(CoreResolvedValue::new(value))
            }
        })
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Configuration object for Python
#[pyclass(name = "Config")]
struct PyConfig {
    inner: CoreConfig,
}

#[pymethods]
impl PyConfig {
    /// Load configuration from a YAML string
    ///
    /// Args:
    ///     yaml: YAML content as a string
    ///     base_path: Optional base path for resolving relative file references
    ///     allow_http: Enable HTTP resolver (disabled by default for security)
    ///     http_allowlist: List of URL patterns to allow (glob-style)
    ///     http_proxy: Proxy URL (e.g., "http://proxy:8080" or "socks5://proxy:1080")
    ///     http_proxy_from_env: Auto-detect proxy from HTTP_PROXY/HTTPS_PROXY env vars
    ///     http_ca_bundle: Path to CA bundle PEM file (replaces default roots)
    ///     http_extra_ca_bundle: Path to extra CA bundle PEM file (adds to default roots)
    ///     http_client_cert: Path to client certificate (PEM or P12/PFX) for mTLS
    ///     http_client_key: Path to client private key PEM (not needed for P12/PFX)
    ///     http_client_key_password: Password for encrypted key or P12/PFX file
    ///     http_insecure: DANGEROUS - Skip TLS certificate verification
    #[staticmethod]
    #[pyo3(signature = (
        yaml,
        base_path=None,
        allow_http=false,
        http_allowlist=None,
        http_proxy=None,
        http_proxy_from_env=false,
        http_ca_bundle=None,
        http_extra_ca_bundle=None,
        http_client_cert=None,
        http_client_key=None,
        http_client_key_password=None,
        http_insecure=false
    ))]
    #[allow(clippy::too_many_arguments)]
    fn loads(
        yaml: &str,
        base_path: Option<&str>,
        allow_http: bool,
        http_allowlist: Option<Vec<String>>,
        http_proxy: Option<&str>,
        http_proxy_from_env: bool,
        http_ca_bundle: Option<&str>,
        http_extra_ca_bundle: Option<&str>,
        http_client_cert: Option<&str>,
        http_client_key: Option<&str>,
        http_client_key_password: Option<&str>,
        http_insecure: bool,
    ) -> PyResult<Self> {
        let mut options = ConfigOptions::default();
        if let Some(bp) = base_path {
            options.base_path = Some(std::path::PathBuf::from(bp));
        }
        options.allow_http = allow_http;
        options.http_allowlist = http_allowlist.unwrap_or_default();
        options.http_proxy = http_proxy.map(String::from);
        options.http_proxy_from_env = http_proxy_from_env;
        options.http_ca_bundle = http_ca_bundle.map(std::path::PathBuf::from);
        options.http_extra_ca_bundle = http_extra_ca_bundle.map(std::path::PathBuf::from);
        options.http_client_cert = http_client_cert.map(std::path::PathBuf::from);
        options.http_client_key = http_client_key.map(std::path::PathBuf::from);
        options.http_client_key_password = http_client_key_password.map(String::from);
        options.http_insecure = http_insecure;
        let inner = CoreConfig::from_yaml_with_options(yaml, options).map_err(to_py_err)?;
        Ok(Self { inner })
    }

    /// Load configuration from a YAML file (required - errors if missing)
    ///
    /// This is the primary way to load configuration. Use `Config.optional()`
    /// for files that may not exist.
    ///
    /// Args:
    ///     path: Path to the YAML file
    ///     schema: Optional path to a JSON Schema file. If provided, schema defaults
    ///            will be used when accessing missing paths.
    ///     allow_http: Enable HTTP resolver (disabled by default for security)
    ///     http_allowlist: List of URL patterns to allow (glob-style)
    ///     http_proxy: Proxy URL (e.g., "http://proxy:8080" or "socks5://proxy:1080")
    ///     http_proxy_from_env: Auto-detect proxy from HTTP_PROXY/HTTPS_PROXY env vars
    ///     http_ca_bundle: Path to CA bundle PEM file (replaces default roots)
    ///     http_extra_ca_bundle: Path to extra CA bundle PEM file (adds to default roots)
    ///     http_client_cert: Path to client certificate (PEM or P12/PFX) for mTLS
    ///     http_client_key: Path to client private key PEM (not needed for P12/PFX)
    ///     http_client_key_password: Password for encrypted key or P12/PFX file
    ///     http_insecure: DANGEROUS - Skip TLS certificate verification
    ///
    /// Returns:
    ///     A new Config object
    ///
    /// Raises:
    ///     HoloconfError: If the file cannot be read or doesn't exist
    ///     ParseError: If the file cannot be parsed
    #[staticmethod]
    #[pyo3(signature = (
        path,
        schema=None,
        allow_http=false,
        http_allowlist=None,
        http_proxy=None,
        http_proxy_from_env=false,
        http_ca_bundle=None,
        http_extra_ca_bundle=None,
        http_client_cert=None,
        http_client_key=None,
        http_client_key_password=None,
        http_insecure=false
    ))]
    #[allow(clippy::too_many_arguments)]
    fn load(
        path: &str,
        schema: Option<&str>,
        allow_http: bool,
        http_allowlist: Option<Vec<String>>,
        http_proxy: Option<&str>,
        http_proxy_from_env: bool,
        http_ca_bundle: Option<&str>,
        http_extra_ca_bundle: Option<&str>,
        http_client_cert: Option<&str>,
        http_client_key: Option<&str>,
        http_client_key_password: Option<&str>,
        http_insecure: bool,
    ) -> PyResult<Self> {
        let mut options = ConfigOptions::default();
        options.allow_http = allow_http;
        options.http_allowlist = http_allowlist.unwrap_or_default();
        options.http_proxy = http_proxy.map(String::from);
        options.http_proxy_from_env = http_proxy_from_env;
        options.http_ca_bundle = http_ca_bundle.map(std::path::PathBuf::from);
        options.http_extra_ca_bundle = http_extra_ca_bundle.map(std::path::PathBuf::from);
        options.http_client_cert = http_client_cert.map(std::path::PathBuf::from);
        options.http_client_key = http_client_key.map(std::path::PathBuf::from);
        options.http_client_key_password = http_client_key_password.map(String::from);
        options.http_insecure = http_insecure;

        // Use load_with_options which supports glob patterns
        let mut inner = CoreConfig::load_with_options(path, options).map_err(to_py_err)?;

        // If schema path provided, load and attach it
        if let Some(schema_path) = schema {
            let schema_obj = CoreSchema::from_file(schema_path).map_err(to_py_err)?;
            inner.set_schema(schema_obj);
        }

        Ok(Self { inner })
    }

    /// Alias for `load()` - load a required config file
    ///
    /// Provided for symmetry with `Config.optional()`.
    ///
    /// Args:
    ///     path: Path to the YAML file
    ///     schema: Optional path to a JSON Schema file
    ///     allow_http: Enable HTTP resolver (disabled by default for security)
    ///     http_allowlist: List of URL patterns to allow (glob-style)
    ///     http_proxy: Proxy URL (e.g., "http://proxy:8080" or "socks5://proxy:1080")
    ///     http_proxy_from_env: Auto-detect proxy from HTTP_PROXY/HTTPS_PROXY env vars
    ///     http_ca_bundle: Path to CA bundle PEM file (replaces default roots)
    ///     http_extra_ca_bundle: Path to extra CA bundle PEM file (adds to default roots)
    ///     http_client_cert: Path to client certificate (PEM or P12/PFX) for mTLS
    ///     http_client_key: Path to client private key PEM (not needed for P12/PFX)
    ///     http_client_key_password: Password for encrypted key or P12/PFX file
    ///     http_insecure: DANGEROUS - Skip TLS certificate verification
    #[staticmethod]
    #[pyo3(signature = (
        path,
        schema=None,
        allow_http=false,
        http_allowlist=None,
        http_proxy=None,
        http_proxy_from_env=false,
        http_ca_bundle=None,
        http_extra_ca_bundle=None,
        http_client_cert=None,
        http_client_key=None,
        http_client_key_password=None,
        http_insecure=false
    ))]
    #[allow(clippy::too_many_arguments)]
    fn required(
        path: &str,
        schema: Option<&str>,
        allow_http: bool,
        http_allowlist: Option<Vec<String>>,
        http_proxy: Option<&str>,
        http_proxy_from_env: bool,
        http_ca_bundle: Option<&str>,
        http_extra_ca_bundle: Option<&str>,
        http_client_cert: Option<&str>,
        http_client_key: Option<&str>,
        http_client_key_password: Option<&str>,
        http_insecure: bool,
    ) -> PyResult<Self> {
        Self::load(
            path,
            schema,
            allow_http,
            http_allowlist,
            http_proxy,
            http_proxy_from_env,
            http_ca_bundle,
            http_extra_ca_bundle,
            http_client_cert,
            http_client_key,
            http_client_key_password,
            http_insecure,
        )
    }

    /// Load an optional configuration file
    ///
    /// Returns an empty Config if the file doesn't exist.
    /// Use this for configuration files that may or may not be present,
    /// such as local overrides.
    ///
    /// Args:
    ///     path: Path to the config file
    ///
    /// Returns:
    ///     A Config object (empty if file doesn't exist)
    ///
    /// Example:
    ///     >>> base = Config.load("base.yaml")
    ///     >>> local = Config.optional("local.yaml")
    ///     >>> base.merge(local)
    #[staticmethod]
    fn optional(path: &str) -> PyResult<Self> {
        let inner = CoreConfig::optional(path).map_err(to_py_err)?;
        Ok(Self { inner })
    }

    /// Merge another config into this one
    ///
    /// The other config's values override this config's values.
    ///
    /// Args:
    ///     other: Another Config to merge into this one
    fn merge(&mut self, other: &PyConfig) {
        self.inner.merge(other.inner.clone());
    }

    /// Load configuration from a JSON string
    ///
    /// Args:
    ///     json: JSON content as a string
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner = CoreConfig::from_json(json).map_err(to_py_err)?;
        Ok(Self { inner })
    }

    /// Get a resolved value by path (e.g., "database.host")
    ///
    /// Interpolations like ${env:VAR} are resolved before returning.
    ///
    /// Args:
    ///     path: Dot-separated path to the value
    ///
    /// Returns:
    ///     The resolved value (str, int, float, bool, list, dict, or None)
    ///
    /// Raises:
    ///     PathNotFoundError: If the path doesn't exist
    ///     ResolverError: If resolution fails (e.g., missing env var)
    fn get(&self, py: Python<'_>, path: &str) -> PyResult<PyObject> {
        let value = self.inner.get(path).map_err(to_py_err)?;
        value_to_py(py, &value)
    }

    /// Get the raw (unresolved) value by path
    ///
    /// Returns the value without resolving interpolations.
    /// Useful for debugging or inspecting the raw configuration.
    ///
    /// Args:
    ///     path: Dot-separated path to the value
    ///
    /// Returns:
    ///     The raw value (may contain ${...} interpolation syntax)
    ///
    /// Raises:
    ///     PathNotFoundError: If the path doesn't exist
    fn get_raw(&self, py: Python<'_>, path: &str) -> PyResult<PyObject> {
        let value = self.inner.get_raw(path).map_err(to_py_err)?;
        value_to_py(py, value)
    }

    /// Get a string value, with type coercion if needed
    ///
    /// Args:
    ///     path: Dot-separated path to the value
    ///
    /// Returns:
    ///     The value as a string
    ///
    /// Raises:
    ///     TypeCoercionError: If the value cannot be converted to string
    fn get_string(&self, path: &str) -> PyResult<String> {
        self.inner.get_string(path).map_err(to_py_err)
    }

    /// Get an integer value, with type coercion if needed
    ///
    /// String values like "42" will be parsed as integers.
    ///
    /// Args:
    ///     path: Dot-separated path to the value
    ///
    /// Returns:
    ///     The value as an integer
    ///
    /// Raises:
    ///     TypeCoercionError: If the value cannot be converted to integer
    fn get_int(&self, path: &str) -> PyResult<i64> {
        self.inner.get_i64(path).map_err(to_py_err)
    }

    /// Get a float value, with type coercion if needed
    ///
    /// String values like "3.14" will be parsed as floats.
    ///
    /// Args:
    ///     path: Dot-separated path to the value
    ///
    /// Returns:
    ///     The value as a float
    ///
    /// Raises:
    ///     TypeCoercionError: If the value cannot be converted to float
    fn get_float(&self, path: &str) -> PyResult<f64> {
        self.inner.get_f64(path).map_err(to_py_err)
    }

    /// Get a boolean value, with strict coercion
    ///
    /// Only "true" and "false" (case-insensitive) are accepted for string coercion.
    ///
    /// Args:
    ///     path: Dot-separated path to the value
    ///
    /// Returns:
    ///     The value as a boolean
    ///
    /// Raises:
    ///     TypeCoercionError: If the value cannot be converted to boolean
    fn get_bool(&self, path: &str) -> PyResult<bool> {
        self.inner.get_bool(path).map_err(to_py_err)
    }

    /// Resolve all values eagerly
    ///
    /// By default, values are resolved lazily when accessed. This method
    /// forces resolution of all values upfront, which can be useful for
    /// detecting errors early or for performance when all values are needed.
    ///
    /// Raises:
    ///     ResolverError: If any value fails to resolve
    fn resolve_all(&self) -> PyResult<()> {
        self.inner.resolve_all().map_err(to_py_err)
    }

    /// Export the configuration as a Python dict
    ///
    /// Args:
    ///     resolve: If True (default), resolve all interpolations. If False, return raw values.
    ///     redact: If True (default False), redact sensitive values with "[REDACTED]"
    ///
    /// Returns:
    ///     The configuration as a Python dictionary
    #[pyo3(signature = (resolve=true, redact=false))]
    fn to_dict(&self, py: Python<'_>, resolve: bool, redact: bool) -> PyResult<PyObject> {
        let value = self.inner.to_value(resolve, redact).map_err(to_py_err)?;
        value_to_py(py, &value)
    }

    /// Export the configuration as YAML
    ///
    /// Args:
    ///     resolve: If True (default), resolve all interpolations. If False, return raw values.
    ///     redact: If True (default False), redact sensitive values with "[REDACTED]"
    ///
    /// Returns:
    ///     The configuration as a YAML string
    #[pyo3(signature = (resolve=true, redact=false))]
    fn to_yaml(&self, resolve: bool, redact: bool) -> PyResult<String> {
        self.inner.to_yaml(resolve, redact).map_err(to_py_err)
    }

    /// Export the configuration as JSON
    ///
    /// Args:
    ///     resolve: If True (default), resolve all interpolations. If False, return raw values.
    ///     redact: If True (default False), redact sensitive values with "[REDACTED]"
    ///
    /// Returns:
    ///     The configuration as a JSON string
    #[pyo3(signature = (resolve=true, redact=false))]
    fn to_json(&self, resolve: bool, redact: bool) -> PyResult<String> {
        self.inner.to_json(resolve, redact).map_err(to_py_err)
    }

    /// Clear the resolution cache
    ///
    /// Resolved values are cached for performance. Call this method to clear
    /// the cache, for example after environment variables have changed.
    fn clear_cache(&self) {
        self.inner.clear_cache();
    }

    /// Register a custom resolver
    ///
    /// The resolver function is called with positional arguments from the interpolation
    /// and keyword arguments. It should return a value (string, int, float, bool, list, dict)
    /// or a ResolvedValue for sensitive data.
    ///
    /// Args:
    ///     name: The resolver name (used as ${name:...} in config)
    ///     func: A callable that takes (*args, **kwargs) and returns a value
    ///
    /// Example:
    ///     >>> def my_resolver(key, default=None):
    ///     ...     return lookup(key) or default
    ///     >>> config.register_resolver("myresolver", my_resolver)
    ///     >>> # Now use: ${myresolver:some_key,fallback_value}
    fn register_resolver(&mut self, name: String, func: PyObject) -> PyResult<()> {
        let resolver = PyResolver::new(name, func);
        self.inner.register_resolver(Arc::new(resolver));
        Ok(())
    }

    /// Get the source file for a config path
    ///
    /// Returns the filename of the config file that provided this value.
    /// For merged configs, this returns the file that "won" for this path.
    ///
    /// Args:
    ///     path: The config path (e.g., "database.host")
    ///
    /// Returns:
    ///     The filename or None if source tracking is not available
    fn get_source(&self, path: &str) -> Option<String> {
        self.inner.get_source(path).map(|s| s.to_string())
    }

    /// Get all source mappings
    ///
    /// Returns a dict mapping config paths to their source filenames.
    /// Useful for debugging which file each value came from.
    ///
    /// Returns:
    ///     A dict of {path: filename} entries
    fn dump_sources(&self) -> HashMap<String, String> {
        self.inner.dump_sources().clone()
    }

    /// Attach a schema to this config for default value lookup
    ///
    /// When a schema is attached, accessing a missing path will return the
    /// schema's default value (if defined) instead of raising PathNotFoundError.
    ///
    /// Args:
    ///     schema: A Schema object to attach
    ///
    /// Example:
    ///     >>> config = Config.load("config.yaml")
    ///     >>> schema = Schema.load("schema.yaml")
    ///     >>> config.set_schema(schema)
    ///     >>> config.pool_size  # Returns schema default if not in config
    fn set_schema(&mut self, schema: &PySchema) {
        self.inner.set_schema(schema.inner.clone());
    }

    /// Get the attached schema, if any
    ///
    /// Returns:
    ///     The attached Schema object, or None if no schema is attached
    fn get_schema(&self) -> Option<PySchema> {
        self.inner
            .get_schema()
            .map(|s| PySchema { inner: s.clone() })
    }

    /// Validate the raw (unresolved) configuration against a schema
    ///
    /// This performs structural validation before resolution, checking that
    /// required keys exist and the configuration structure matches the schema.
    /// Interpolation placeholders (${...}) are allowed as valid values.
    ///
    /// Args:
    ///     schema: Optional Schema object to validate against. If not provided,
    ///            uses the attached schema (set via `set_schema()` or `load(schema=...)`).
    ///
    /// Raises:
    ///     ValidationError: If validation fails
    ///     HoloconfError: If no schema is provided and none is attached
    #[pyo3(signature = (schema=None))]
    fn validate_raw(&self, schema: Option<&PySchema>) -> PyResult<()> {
        self.inner
            .validate_raw(schema.map(|s| &s.inner))
            .map_err(to_py_err)
    }

    /// Validate the resolved configuration against a schema
    ///
    /// This resolves all values first, then validates the resolved values
    /// against the schema, checking types, constraints, and patterns.
    ///
    /// Args:
    ///     schema: Optional Schema object to validate against. If not provided,
    ///            uses the attached schema (set via `set_schema()` or `load(schema=...)`).
    ///
    /// Raises:
    ///     ValidationError: If validation fails
    ///     ResolverError: If resolution fails
    ///     HoloconfError: If no schema is provided and none is attached
    #[pyo3(signature = (schema=None))]
    fn validate(&self, schema: Option<&PySchema>) -> PyResult<()> {
        self.inner
            .validate(schema.map(|s| &s.inner))
            .map_err(to_py_err)
    }

    /// Validate and collect all errors (instead of failing on first)
    ///
    /// Args:
    ///     schema: Optional Schema object to validate against. If not provided,
    ///            uses the attached schema (set via `set_schema()` or `load(schema=...)`).
    ///
    /// Returns:
    ///     A list of error message strings (empty if valid)
    ///
    /// Raises:
    ///     HoloconfError: If no schema is provided and none is attached
    #[pyo3(signature = (schema=None))]
    fn validate_collect(&self, schema: Option<&PySchema>) -> PyResult<Vec<String>> {
        // Need to handle the case where no schema is provided and none attached
        let schema_ref = schema.map(|s| &s.inner);

        // Check if we have a schema to use
        if schema_ref.is_none() && self.inner.get_schema().is_none() {
            return Err(to_py_err(holoconf_core::Error::validation(
                "",
                "No schema provided and no schema attached to config",
            )));
        }

        Ok(self
            .inner
            .validate_collect(schema_ref)
            .into_iter()
            .map(|e| e.to_string())
            .collect())
    }

    /// Python dict-like access: config["key"]
    fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<PyObject> {
        self.get(py, key)
    }

    /// Python attribute access: config.key
    fn __getattr__(&self, py: Python<'_>, name: &str) -> PyResult<PyObject> {
        self.get(py, name)
    }

    fn __repr__(&self) -> String {
        "Config(<...>)".to_string()
    }
}

/// Schema for validating configuration
#[pyclass(name = "Schema")]
struct PySchema {
    inner: CoreSchema,
}

#[pymethods]
impl PySchema {
    /// Load a schema from a YAML string
    ///
    /// Args:
    ///     yaml: JSON Schema as a YAML string
    ///
    /// Returns:
    ///     A Schema object
    ///
    /// Raises:
    ///     ParseError: If the YAML is invalid or not a valid JSON Schema
    #[staticmethod]
    fn from_yaml(yaml: &str) -> PyResult<Self> {
        let inner = CoreSchema::from_yaml(yaml).map_err(to_py_err)?;
        Ok(Self { inner })
    }

    /// Load a schema from a JSON string
    ///
    /// Args:
    ///     json: JSON Schema as a JSON string
    ///
    /// Returns:
    ///     A Schema object
    ///
    /// Raises:
    ///     ParseError: If the JSON is invalid or not a valid JSON Schema
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner = CoreSchema::from_json(json).map_err(to_py_err)?;
        Ok(Self { inner })
    }

    /// Load a schema from a file (JSON or YAML based on extension)
    ///
    /// Args:
    ///     path: Path to the schema file (.yaml, .yml, or .json)
    ///
    /// Returns:
    ///     A Schema object
    ///
    /// Raises:
    ///     ParseError: If the file cannot be parsed
    ///     HoloconfError: If the file cannot be read
    #[staticmethod]
    fn load(path: &str) -> PyResult<Self> {
        let inner = CoreSchema::from_file(path).map_err(to_py_err)?;
        Ok(Self { inner })
    }

    /// Output the schema as YAML
    ///
    /// Returns:
    ///     The schema serialized as a YAML string
    fn to_yaml(&self) -> PyResult<String> {
        self.inner.to_yaml().map_err(to_py_err)
    }

    /// Output the schema as JSON
    ///
    /// Returns:
    ///     The schema serialized as a JSON string
    fn to_json(&self) -> PyResult<String> {
        self.inner.to_json().map_err(to_py_err)
    }

    /// Generate markdown documentation from the schema
    ///
    /// Returns:
    ///     Human-readable markdown documentation
    fn to_markdown(&self) -> String {
        self.inner.to_markdown()
    }

    fn __repr__(&self) -> String {
        "Schema(<...>)".to_string()
    }
}

/// Register a resolver in the global registry
///
/// This makes the resolver available to all Config instances created after registration.
/// Use this for extension packages that provide additional resolvers.
///
/// Args:
///     name: The resolver name (used as ${name:...} in config)
///     func: A callable that takes (*args, **kwargs) and returns a value
///     force: If True, overwrite any existing resolver with the same name.
///            If False (default), raise an error if the name is already registered.
///
/// Example:
///     >>> import holoconf
///     >>>
///     >>> def ssm_resolver(path, region=None, profile=None):
///     ...     # Implementation here
///     ...     return value
///     >>>
///     >>> holoconf.register_resolver("ssm", ssm_resolver)
///     >>> # Now any Config can use ${ssm:/my/param}
#[pyfunction]
#[pyo3(signature = (name, func, force=false))]
fn register_resolver(name: String, func: PyObject, force: bool) -> PyResult<()> {
    let resolver = Arc::new(PyResolver::new(name, func));
    holoconf_core::resolver::register_global(resolver, force).map_err(to_py_err)
}

/// The holoconf Python module
#[pymodule]
#[pyo3(name = "_holoconf")]
fn holoconf(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add classes
    m.add_class::<PyConfig>()?;
    m.add_class::<PySchema>()?;
    m.add_class::<PyResolvedValue>()?;

    // Add module-level functions
    m.add_function(wrap_pyfunction!(register_resolver, m)?)?;

    // Add exception hierarchy
    m.add("HoloconfError", m.py().get_type::<HoloconfError>())?;
    m.add("ParseError", m.py().get_type::<ParseError>())?;
    m.add("ValidationError", m.py().get_type::<ValidationError>())?;
    m.add("ResolverError", m.py().get_type::<ResolverError>())?;
    m.add("PathNotFoundError", m.py().get_type::<PathNotFoundError>())?;
    m.add(
        "CircularReferenceError",
        m.py().get_type::<CircularReferenceError>(),
    )?;
    m.add("TypeCoercionError", m.py().get_type::<TypeCoercionError>())?;

    Ok(())
}

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
use pyo3::types::{PyDict, PyList};

use holoconf_core::{
    error::ErrorKind, Config as CoreConfig, ConfigOptions, Error as CoreError,
    Schema as CoreSchema, Value as CoreValue,
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
    #[staticmethod]
    #[pyo3(signature = (yaml, base_path=None, allow_http=false))]
    fn loads(yaml: &str, base_path: Option<&str>, allow_http: bool) -> PyResult<Self> {
        let mut options = ConfigOptions::default();
        if let Some(bp) = base_path {
            options.base_path = Some(std::path::PathBuf::from(bp));
        }
        options.allow_http = allow_http;
        let inner = CoreConfig::from_yaml_with_options(yaml, options).map_err(to_py_err)?;
        Ok(Self { inner })
    }

    /// Load configuration from a YAML file
    ///
    /// Args:
    ///     path: Path to the YAML file
    ///     allow_http: Enable HTTP resolver (disabled by default for security)
    #[staticmethod]
    #[pyo3(signature = (path, allow_http=false))]
    fn load(path: &str, allow_http: bool) -> PyResult<Self> {
        let path_ref = std::path::Path::new(path);
        let content = std::fs::read_to_string(path_ref).map_err(|e| {
            to_py_err(holoconf_core::Error::parse(format!(
                "Failed to read file '{}': {}",
                path, e
            )))
        })?;

        let value: holoconf_core::Value = serde_yaml::from_str(&content)
            .map_err(|e| to_py_err(holoconf_core::Error::parse(e.to_string())))?;

        let mut options = ConfigOptions::default();
        options.base_path = path_ref.parent().map(|p| p.to_path_buf());
        options.allow_http = allow_http;

        let inner = CoreConfig::with_options(value, options);
        Ok(Self { inner })
    }

    /// Load and merge multiple YAML files
    ///
    /// Files are merged in order, with later files overriding earlier ones.
    ///
    /// Args:
    ///     paths: List of paths to YAML files
    #[staticmethod]
    fn load_merged(paths: Vec<String>) -> PyResult<Self> {
        let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
        let inner = CoreConfig::load_merged(&path_refs).map_err(to_py_err)?;
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
        let value = if resolve {
            self.inner.to_value_redacted(redact).map_err(to_py_err)?
        } else {
            self.inner.to_value_raw()
        };
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
        if resolve {
            self.inner.to_yaml_redacted(redact).map_err(to_py_err)
        } else {
            self.inner.to_yaml_raw().map_err(to_py_err)
        }
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
        if resolve {
            self.inner.to_json_redacted(redact).map_err(to_py_err)
        } else {
            self.inner.to_json_raw().map_err(to_py_err)
        }
    }

    /// Clear the resolution cache
    ///
    /// Resolved values are cached for performance. Call this method to clear
    /// the cache, for example after environment variables have changed.
    fn clear_cache(&self) {
        self.inner.clear_cache();
    }

    /// Validate the raw (unresolved) configuration against a schema
    ///
    /// This performs structural validation before resolution, checking that
    /// required keys exist and the configuration structure matches the schema.
    /// Interpolation placeholders (${...}) are allowed as valid values.
    ///
    /// Args:
    ///     schema: A Schema object to validate against
    ///
    /// Raises:
    ///     ValidationError: If validation fails
    fn validate_raw(&self, schema: &PySchema) -> PyResult<()> {
        self.inner.validate_raw(&schema.inner).map_err(to_py_err)
    }

    /// Validate the resolved configuration against a schema
    ///
    /// This resolves all values first, then validates the resolved values
    /// against the schema, checking types, constraints, and patterns.
    ///
    /// Args:
    ///     schema: A Schema object to validate against
    ///
    /// Raises:
    ///     ValidationError: If validation fails
    ///     ResolverError: If resolution fails
    fn validate(&self, schema: &PySchema) -> PyResult<()> {
        self.inner.validate(&schema.inner).map_err(to_py_err)
    }

    /// Validate and collect all errors (instead of failing on first)
    ///
    /// Args:
    ///     schema: A Schema object to validate against
    ///
    /// Returns:
    ///     A list of error message strings (empty if valid)
    fn validate_collect(&self, schema: &PySchema) -> Vec<String> {
        self.inner
            .validate_collect(&schema.inner)
            .into_iter()
            .map(|e| e.to_string())
            .collect()
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

    fn __repr__(&self) -> String {
        "Schema(<...>)".to_string()
    }
}

/// The holoconf Python module
#[pymodule]
#[pyo3(name = "_holoconf")]
fn holoconf(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add classes
    m.add_class::<PyConfig>()?;
    m.add_class::<PySchema>()?;

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

//! Python bindings for holoconf
//!
//! This crate provides Python bindings for holoconf-core using PyO3.

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use pyo3::types::{PyList, PyDict};

use holoconf_core::{Config as CoreConfig, ConfigOptions, Value as CoreValue, Error as CoreError};

/// Convert a holoconf error to a Python exception
fn to_py_err(err: CoreError) -> PyErr {
    PyValueError::new_err(format!("{}", err))
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
    #[staticmethod]
    #[pyo3(signature = (yaml, base_path=None))]
    fn loads(yaml: &str, base_path: Option<&str>) -> PyResult<Self> {
        let inner = if let Some(bp) = base_path {
            let mut options = ConfigOptions::default();
            options.base_path = Some(std::path::PathBuf::from(bp));
            CoreConfig::from_yaml_with_options(yaml, options).map_err(to_py_err)?
        } else {
            CoreConfig::from_yaml(yaml).map_err(to_py_err)?
        };
        Ok(Self { inner })
    }

    /// Load configuration from a YAML file
    #[staticmethod]
    fn load(path: &str) -> PyResult<Self> {
        let inner = CoreConfig::from_yaml_file(path).map_err(to_py_err)?;
        Ok(Self { inner })
    }

    /// Load and merge multiple YAML files
    ///
    /// Files are merged in order, with later files overriding earlier ones.
    #[staticmethod]
    fn load_merged(paths: Vec<String>) -> PyResult<Self> {
        let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
        let inner = CoreConfig::load_merged(&path_refs).map_err(to_py_err)?;
        Ok(Self { inner })
    }

    /// Merge another config into this one
    ///
    /// The other config's values override this config's values.
    fn merge(&mut self, other: &PyConfig) {
        self.inner.merge(other.inner.clone());
    }

    /// Load configuration from a JSON string
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner = CoreConfig::from_json(json).map_err(to_py_err)?;
        Ok(Self { inner })
    }

    /// Get a value by path (e.g., "database.host")
    fn get(&self, py: Python<'_>, path: &str) -> PyResult<PyObject> {
        let value = self.inner.get(path).map_err(to_py_err)?;
        value_to_py(py, &value)
    }

    /// Get a string value, with type coercion if needed
    fn get_string(&self, path: &str) -> PyResult<String> {
        self.inner.get_string(path).map_err(to_py_err)
    }

    /// Get an integer value, with type coercion if needed
    fn get_int(&self, path: &str) -> PyResult<i64> {
        self.inner.get_i64(path).map_err(to_py_err)
    }

    /// Get a float value, with type coercion if needed
    fn get_float(&self, path: &str) -> PyResult<f64> {
        self.inner.get_f64(path).map_err(to_py_err)
    }

    /// Get a boolean value, with strict coercion (only "true"/"false")
    fn get_bool(&self, path: &str) -> PyResult<bool> {
        self.inner.get_bool(path).map_err(to_py_err)
    }

    /// Resolve all values eagerly
    fn resolve_all(&self) -> PyResult<()> {
        self.inner.resolve_all().map_err(to_py_err)
    }

    /// Export the resolved configuration as a Python dict
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let value = self.inner.to_value().map_err(to_py_err)?;
        value_to_py(py, &value)
    }

    /// Export the resolved configuration as YAML
    fn to_yaml(&self) -> PyResult<String> {
        self.inner.to_yaml().map_err(to_py_err)
    }

    /// Export the resolved configuration as JSON
    fn to_json(&self) -> PyResult<String> {
        self.inner.to_json().map_err(to_py_err)
    }

    /// Clear the resolution cache
    fn clear_cache(&self) {
        self.inner.clear_cache();
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
        format!("Config(<...>)")
    }
}

/// The holoconf Python module
#[pymodule]
#[pyo3(name = "_holoconf")]
fn holoconf(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyConfig>()?;
    Ok(())
}

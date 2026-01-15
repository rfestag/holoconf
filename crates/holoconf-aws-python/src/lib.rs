//! Python bindings for holoconf AWS resolvers
//!
//! This crate exposes the Rust AWS resolvers to Python via PyO3.

use pyo3::prelude::*;
use std::sync::Arc;

use holoconf_aws::SsmResolver;
use holoconf_core::resolver::{global_registry, register_global};

/// Register the SSM resolver in the global registry.
///
/// This makes the SSM resolver available to all Config instances.
///
/// Args:
///     force: If True, overwrite any existing 'ssm' resolver.
///            If False (default), only register if not already registered.
#[pyfunction]
#[pyo3(signature = (force=false))]
fn register_ssm(force: bool) -> PyResult<()> {
    // Check if already registered (idempotent without force)
    if !force {
        let registry = global_registry().read().unwrap();
        if registry.contains("ssm") {
            return Ok(());
        }
    }

    let resolver = Arc::new(SsmResolver::new());
    register_global(resolver, force).map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to register SSM resolver: {}", e))
    })
}

/// Register all AWS resolvers in the global registry.
///
/// Currently registers:
/// - ssm: AWS Systems Manager Parameter Store resolver
#[pyfunction]
#[pyo3(signature = (force=false))]
fn register_all(force: bool) -> PyResult<()> {
    register_ssm(force)
}

/// Python module for holoconf AWS resolvers
#[pymodule]
fn _holoconf_aws(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(register_ssm, m)?)?;
    m.add_function(wrap_pyfunction!(register_all, m)?)?;
    Ok(())
}

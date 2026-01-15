//! Python bindings for holoconf AWS resolvers
//!
//! This crate exposes the Rust AWS resolvers to Python via PyO3.

use pyo3::prelude::*;
use std::sync::Arc;

use holoconf_aws::{CfnResolver, S3Resolver, SsmResolver};
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

/// Register the CloudFormation resolver in the global registry.
///
/// This makes the `cfn` resolver available to all Config instances.
///
/// Args:
///     force: If True, overwrite any existing 'cfn' resolver.
///            If False (default), only register if not already registered.
#[pyfunction]
#[pyo3(signature = (force=false))]
fn register_cfn(force: bool) -> PyResult<()> {
    // Check if already registered (idempotent without force)
    if !force {
        let registry = global_registry().read().unwrap();
        if registry.contains("cfn") {
            return Ok(());
        }
    }

    let resolver = Arc::new(CfnResolver::new());
    register_global(resolver, force).map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!(
            "Failed to register CloudFormation resolver: {}",
            e
        ))
    })
}

/// Register the S3 resolver in the global registry.
///
/// This makes the `s3` resolver available to all Config instances.
///
/// Args:
///     force: If True, overwrite any existing 's3' resolver.
///            If False (default), only register if not already registered.
#[pyfunction]
#[pyo3(signature = (force=false))]
fn register_s3(force: bool) -> PyResult<()> {
    // Check if already registered (idempotent without force)
    if !force {
        let registry = global_registry().read().unwrap();
        if registry.contains("s3") {
            return Ok(());
        }
    }

    let resolver = Arc::new(S3Resolver::new());
    register_global(resolver, force).map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to register S3 resolver: {}", e))
    })
}

/// Register all AWS resolvers in the global registry.
///
/// Registers:
/// - ssm: AWS Systems Manager Parameter Store resolver
/// - cfn: AWS CloudFormation outputs resolver
/// - s3: AWS S3 object resolver
#[pyfunction]
#[pyo3(signature = (force=false))]
fn register_all(force: bool) -> PyResult<()> {
    register_ssm(force)?;
    register_cfn(force)?;
    register_s3(force)?;
    Ok(())
}

/// Python module for holoconf AWS resolvers
#[pymodule]
fn _holoconf_aws(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(register_ssm, m)?)?;
    m.add_function(wrap_pyfunction!(register_cfn, m)?)?;
    m.add_function(wrap_pyfunction!(register_s3, m)?)?;
    m.add_function(wrap_pyfunction!(register_all, m)?)?;
    Ok(())
}

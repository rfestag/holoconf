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

/// Configure global defaults for all AWS services.
///
/// Sets default region and profile that apply to all AWS services (S3, SSM, CloudFormation).
/// These can be overridden by service-specific configuration or per-resolver kwargs.
///
/// Args:
///     region: Default AWS region (e.g., "us-east-1")
///     profile: Default AWS profile name
///
/// Example:
///     >>> import holoconf_aws
///     >>> holoconf_aws.configure(region="us-east-1", profile="prod")
#[pyfunction]
#[pyo3(signature = (region=None, profile=None))]
fn configure(region: Option<String>, profile: Option<String>) {
    holoconf_aws::configure(region, profile);
}

/// Configure S3-specific defaults.
///
/// Overrides global configuration for the S3 resolver.
///
/// Args:
///     endpoint: S3 endpoint URL (for moto/LocalStack, e.g., "http://localhost:5000")
///     region: AWS region (overrides global region)
///     profile: AWS profile name (overrides global profile)
///
/// Example:
///     >>> import holoconf_aws
///     >>> holoconf_aws.s3(endpoint="http://localhost:5000", region="us-west-2")
#[pyfunction]
#[pyo3(signature = (endpoint=None, region=None, profile=None))]
fn s3(endpoint: Option<String>, region: Option<String>, profile: Option<String>) {
    holoconf_aws::configure_s3(endpoint, region, profile);
}

/// Configure SSM-specific defaults.
///
/// Overrides global configuration for the SSM resolver.
///
/// Args:
///     endpoint: SSM endpoint URL (for moto/LocalStack)
///     region: AWS region (overrides global region)
///     profile: AWS profile name (overrides global profile)
///
/// Example:
///     >>> import holoconf_aws
///     >>> holoconf_aws.ssm(endpoint="http://localhost:5001")
#[pyfunction]
#[pyo3(signature = (endpoint=None, region=None, profile=None))]
fn ssm(endpoint: Option<String>, region: Option<String>, profile: Option<String>) {
    holoconf_aws::configure_ssm(endpoint, region, profile);
}

/// Configure CloudFormation-specific defaults.
///
/// Overrides global configuration for the CloudFormation resolver.
///
/// Args:
///     endpoint: CloudFormation endpoint URL (for moto/LocalStack)
///     region: AWS region (overrides global region)
///     profile: AWS profile name (overrides global profile)
///
/// Example:
///     >>> import holoconf_aws
///     >>> holoconf_aws.cfn(profile="testing")
#[pyfunction]
#[pyo3(signature = (endpoint=None, region=None, profile=None))]
fn cfn(endpoint: Option<String>, region: Option<String>, profile: Option<String>) {
    holoconf_aws::configure_cfn(endpoint, region, profile);
}

/// Reset all configuration and clear the client cache.
///
/// Clears all global and service-specific configuration, and removes all cached AWS clients.
/// Useful for test isolation.
///
/// Example:
///     >>> import holoconf_aws
///     >>> holoconf_aws.s3(endpoint="http://localhost:5000")
///     >>> # ... run tests ...
///     >>> holoconf_aws.reset()  # Clean up for next test
#[pyfunction]
fn reset() {
    holoconf_aws::reset();
}

/// Python module for holoconf AWS resolvers
#[pymodule]
fn _holoconf_aws(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(register_ssm, m)?)?;
    m.add_function(wrap_pyfunction!(register_cfn, m)?)?;
    m.add_function(wrap_pyfunction!(register_s3, m)?)?;
    m.add_function(wrap_pyfunction!(register_all, m)?)?;
    m.add_function(wrap_pyfunction!(configure, m)?)?;
    m.add_function(wrap_pyfunction!(s3, m)?)?;
    m.add_function(wrap_pyfunction!(ssm, m)?)?;
    m.add_function(wrap_pyfunction!(cfn, m)?)?;
    m.add_function(wrap_pyfunction!(reset, m)?)?;
    Ok(())
}

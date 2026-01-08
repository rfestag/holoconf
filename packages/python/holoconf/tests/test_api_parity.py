"""
API Parity Tests for Python Bindings

These tests are generated dynamically from tests/acceptance/api/parity.yaml
to ensure the Python bindings expose all methods and exceptions defined in the spec.

When adding new features to holoconf-core, add them to parity.yaml first -
these tests will automatically fail until the Python bindings are updated.

Run with: pytest tests/test_api_parity.py -v
"""

import inspect
from pathlib import Path

import pytest
import yaml

import holoconf

# Load the parity spec
SPEC_PATH = (
    Path(__file__).parent.parent.parent.parent.parent
    / "tests"
    / "acceptance"
    / "api"
    / "parity.yaml"
)
with open(SPEC_PATH) as f:
    PARITY_SPEC = yaml.safe_load(f)


# =============================================================================
# Exception Hierarchy Tests (generated from spec)
# =============================================================================


class TestExceptionHierarchy:
    """Verify exception hierarchy matches the spec."""

    @pytest.mark.parametrize(
        "exc_spec",
        PARITY_SPEC["exceptions"],
        ids=lambda x: x["name"],
    )
    def test_exception_exported(self, exc_spec):
        """Exception is exported from holoconf module."""
        name = exc_spec["name"]
        assert hasattr(holoconf, name), f"{name} not exported from holoconf module"

    @pytest.mark.parametrize(
        "exc_spec",
        PARITY_SPEC["exceptions"],
        ids=lambda x: x["name"],
    )
    def test_exception_inheritance(self, exc_spec):
        """Exception inherits from its specified parent."""
        name = exc_spec["name"]
        parent_name = exc_spec["parent"]

        exc_class = getattr(holoconf, name)

        if parent_name is None:
            # Base exception should inherit from Python's Exception
            assert issubclass(exc_class, Exception), f"{name} should inherit from Exception"
        else:
            parent_class = getattr(holoconf, parent_name)
            assert issubclass(exc_class, parent_class), f"{name} should inherit from {parent_name}"


# =============================================================================
# Class/Method Tests (generated from spec)
# =============================================================================


@pytest.mark.parametrize(
    "class_name",
    list(PARITY_SPEC["classes"].keys()),
)
def test_class_exported(class_name):
    """Class is exported from holoconf module."""
    assert hasattr(holoconf, class_name), f"{class_name} not exported"


# Generate static method test cases
_static_method_cases = [
    (class_name, method)
    for class_name, class_spec in PARITY_SPEC["classes"].items()
    for method in class_spec.get("static_methods", [])
]


@pytest.mark.parametrize(
    "class_name,method_spec",
    _static_method_cases,
    ids=[f"{c}.{m['name']}" for c, m in _static_method_cases],
)
def test_static_method_exists(class_name, method_spec):
    """Static method exists on the class."""
    cls = getattr(holoconf, class_name)
    method_name = method_spec["name"]
    assert hasattr(cls, method_name), f"{class_name}.{method_name} not found"
    assert callable(getattr(cls, method_name))


# Generate static method parameter test cases
_static_method_param_cases = [
    (class_name, method)
    for class_name, class_spec in PARITY_SPEC["classes"].items()
    for method in class_spec.get("static_methods", [])
    if method.get("parameters")
]


@pytest.mark.parametrize(
    "class_name,method_spec",
    _static_method_param_cases,
    ids=[f"{c}.{m['name']}_params" for c, m in _static_method_param_cases],
)
def test_static_method_parameters(class_name, method_spec):
    """Static method has the expected parameters."""
    cls = getattr(holoconf, class_name)
    method = getattr(cls, method_spec["name"])
    sig = inspect.signature(method)
    param_names = list(sig.parameters.keys())

    for param_spec in method_spec["parameters"]:
        assert param_spec["name"] in param_names, (
            f"{class_name}.{method_spec['name']} missing parameter '{param_spec['name']}'"
        )


# Generate instance method test cases
_instance_method_cases = [
    (class_name, method)
    for class_name, class_spec in PARITY_SPEC["classes"].items()
    for method in class_spec.get("instance_methods", [])
]


@pytest.fixture
def config_instance():
    """Create a Config instance for testing instance methods."""
    return holoconf.Config.loads("key: value")


@pytest.fixture
def schema_instance():
    """Create a Schema instance for testing instance methods."""
    return holoconf.Schema.from_yaml("type: object")


@pytest.mark.parametrize(
    "class_name,method_spec",
    _instance_method_cases,
    ids=[f"{c}.{m['name']}" for c, m in _instance_method_cases],
)
def test_instance_method_exists(class_name, method_spec, config_instance, schema_instance):
    """Instance method exists on the class."""
    # Get an instance of the appropriate class
    if class_name == "Config":
        instance = config_instance
    elif class_name == "Schema":
        instance = schema_instance
    else:
        pytest.skip(f"No fixture for {class_name}")

    method_name = method_spec["name"]
    assert hasattr(instance, method_name), f"{class_name}.{method_name} not found"
    assert callable(getattr(instance, method_name))


# Generate instance method parameter test cases
_instance_method_param_cases = [
    (class_name, method)
    for class_name, class_spec in PARITY_SPEC["classes"].items()
    for method in class_spec.get("instance_methods", [])
    if method.get("parameters")
]


@pytest.mark.parametrize(
    "class_name,method_spec",
    _instance_method_param_cases,
    ids=[f"{c}.{m['name']}_params" for c, m in _instance_method_param_cases],
)
def test_instance_method_parameters(class_name, method_spec, config_instance, schema_instance):
    """Instance method has the expected parameters."""
    # Get an instance of the appropriate class
    if class_name == "Config":
        instance = config_instance
    elif class_name == "Schema":
        instance = schema_instance
    else:
        pytest.skip(f"No fixture for {class_name}")

    method = getattr(instance, method_spec["name"])
    sig = inspect.signature(method)
    param_names = list(sig.parameters.keys())

    for param_spec in method_spec["parameters"]:
        assert param_spec["name"] in param_names, (
            f"{class_name}.{method_spec['name']} missing parameter '{param_spec['name']}'"
        )


# =============================================================================
# Note: Behavioral tests (the 'tests' section in parity.yaml) are executed
# by tools/test_runner.py, not here. This file only verifies API surface.
# =============================================================================

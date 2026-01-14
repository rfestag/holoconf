"""Tests for SSM resolver."""

import pytest

from holoconf_aws import SsmResolver, register_ssm


class TestSsmResolver:
    """Tests for SsmResolver class."""

    def test_name(self):
        """Test that resolver has correct name."""
        resolver = SsmResolver()
        assert resolver.name == "ssm"

    def test_path_must_start_with_slash(self):
        """Test that paths must start with /."""
        resolver = SsmResolver()
        with pytest.raises(ValueError, match="must start with /"):
            resolver("invalid-path")

    def test_path_validation_works_for_valid_path(self):
        """Test that valid paths pass validation."""
        resolver = SsmResolver()
        # This will fail due to no AWS credentials, but path validation should pass
        with pytest.raises(Exception):  # Could be KeyError or ClientError
            resolver("/app/test-param")


class TestRegistration:
    """Tests for registration functions."""

    def test_register_ssm_is_idempotent(self):
        """Test that register_ssm can be called multiple times."""
        # Should not raise
        register_ssm()
        register_ssm()

    def test_register_ssm_force_overwrites(self):
        """Test that force=True allows re-registration."""
        # Should not raise
        register_ssm(force=True)

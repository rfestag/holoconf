"""Tests for SSM resolver registration."""

from holoconf_aws import register_all, register_ssm


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

    def test_register_all_is_idempotent(self):
        """Test that register_all can be called multiple times."""
        # Should not raise
        register_all()
        register_all()

    def test_register_all_force_overwrites(self):
        """Test that force=True allows re-registration."""
        # Should not raise
        register_all(force=True)

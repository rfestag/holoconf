"""
Schema Validation Tests for Python Bindings

These tests ensure the schema validation functionality works correctly
through the Python bindings.
"""

import pytest

from holoconf import Config, HoloconfError, Schema


class TestSchemaValidation:
    """Test schema validation through Python bindings."""

    def test_schema_validation_success(self):
        """Test schema validation passes for valid config."""
        config = Config.loads("port: 8080")
        schema = Schema.from_yaml(
            """
type: object
properties:
  port:
    type: integer
"""
        )
        config.validate(schema)  # Should not raise

    def test_schema_validation_failure(self):
        """Test schema validation raises for invalid config."""
        config = Config.loads("port: not_a_number")
        schema = Schema.from_yaml(
            """
type: object
properties:
  port:
    type: integer
"""
        )
        with pytest.raises(HoloconfError):
            config.validate(schema)

    def test_schema_validation_required_field(self):
        """Test schema validation for required fields."""
        config = Config.loads("optional: value")
        schema = Schema.from_yaml(
            """
type: object
properties:
  required_field:
    type: string
required:
  - required_field
"""
        )
        with pytest.raises(HoloconfError):
            config.validate(schema)

    def test_schema_validation_nested_object(self):
        """Test schema validation for nested objects."""
        config = Config.loads(
            """
database:
  host: localhost
  port: 5432
"""
        )
        schema = Schema.from_yaml(
            """
type: object
properties:
  database:
    type: object
    properties:
      host:
        type: string
      port:
        type: integer
"""
        )
        config.validate(schema)  # Should not raise

    def test_schema_validate_raw(self):
        """Test validate_raw method (validates without resolving)."""
        config = Config.loads("value: ${env:SOME_VAR}")
        schema = Schema.from_yaml(
            """
type: object
properties:
  value:
    type: string
"""
        )
        # validate_raw should pass since raw value is a string
        config.validate_raw(schema)

    def test_schema_validate_collect(self):
        """Test validate_collect returns list of errors."""
        config = Config.loads(
            """
port: "not_a_number"
name: 123
"""
        )
        schema = Schema.from_yaml(
            """
type: object
properties:
  port:
    type: integer
  name:
    type: string
"""
        )
        errors = config.validate_collect(schema)
        # Should return a list of validation errors
        assert isinstance(errors, list)
        assert len(errors) > 0


class TestSchemaLoadMethods:
    """Test Schema loading methods."""

    def test_schema_from_yaml(self):
        """Test Schema.from_yaml class method."""
        schema = Schema.from_yaml("type: object")
        assert schema is not None

    def test_schema_from_file(self, tmp_path):
        """Test Schema.load class method from file."""
        schema_file = tmp_path / "schema.yaml"
        schema_file.write_text("type: object")
        schema = Schema.load(str(schema_file))
        assert schema is not None

    def test_schema_from_invalid_yaml(self):
        """Test Schema raises error for invalid YAML."""
        with pytest.raises(HoloconfError):
            Schema.from_yaml("invalid: [unclosed")


class TestErrorMessages:
    """Test that error messages are preserved through Python bindings."""

    def test_parse_error_message(self):
        """Test that parse errors include helpful context."""
        with pytest.raises(HoloconfError) as exc:
            Config.loads("invalid: [unclosed")
        error_msg = str(exc.value).lower()
        # Should contain some indication of the error
        assert "error" in error_msg or "parse" in error_msg or "invalid" in error_msg

    def test_validation_error_message(self):
        """Test that validation errors include field information."""
        config = Config.loads("port: invalid")
        schema = Schema.from_yaml(
            """
type: object
properties:
  port:
    type: integer
"""
        )
        with pytest.raises(HoloconfError) as exc:
            config.validate(schema)
        # Error message should reference the failing field
        error_msg = str(exc.value).lower()
        assert "port" in error_msg or "type" in error_msg or "integer" in error_msg

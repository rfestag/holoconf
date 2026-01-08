//! Schema validation for configuration (ADR-007, FEAT-004)
//!
//! Provides JSON Schema based validation with two-phase validation:
//! - Phase 1 (structural): Validates structure after merge, interpolations allowed
//! - Phase 2 (type/value): Validates resolved values against constraints

use std::path::Path;
use std::sync::Arc;

use crate::error::{Error, Result};
use crate::value::Value;

/// Schema for validating configuration
#[derive(Debug, Clone)]
pub struct Schema {
    /// The JSON Schema as a serde_json::Value
    schema: serde_json::Value,
    /// Compiled JSON Schema validator (wrapped in Arc for Clone)
    compiled: Arc<jsonschema::Validator>,
}

impl Schema {
    /// Load a schema from a JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        let schema: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| Error::parse(format!("Invalid JSON schema: {}", e)))?;
        Self::from_value(schema)
    }

    /// Load a schema from a YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let schema: serde_json::Value = serde_yaml::from_str(yaml)
            .map_err(|e| Error::parse(format!("Invalid YAML schema: {}", e)))?;
        Self::from_value(schema)
    }

    /// Load a schema from a file (JSON or YAML based on extension)
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|_| Error::file_not_found(path.display().to_string(), None))?;

        match path.extension().and_then(|e| e.to_str()) {
            Some("json") => Self::from_json(&content),
            Some("yaml") | Some("yml") => Self::from_yaml(&content),
            _ => Self::from_yaml(&content), // Default to YAML
        }
    }

    /// Create a schema from a serde_json::Value
    fn from_value(schema: serde_json::Value) -> Result<Self> {
        let compiled = jsonschema::validator_for(&schema)
            .map_err(|e| Error::parse(format!("Invalid JSON Schema: {}", e)))?;
        Ok(Self {
            schema,
            compiled: Arc::new(compiled),
        })
    }

    /// Validate a Value against this schema
    ///
    /// Returns Ok(()) if valid, or an error with details about the first validation failure.
    pub fn validate(&self, value: &Value) -> Result<()> {
        // Convert Value to serde_json::Value for validation
        let json_value = value_to_json(value);

        // Use iter_errors to get an iterator of validation errors
        let mut errors = self.compiled.iter_errors(&json_value);
        if let Some(error) = errors.next() {
            let path = error.instance_path.to_string();
            let message = error.to_string();
            return Err(Error::validation(
                if path.is_empty() { "<root>" } else { &path },
                &message,
            ));
        }
        Ok(())
    }

    /// Validate and collect all errors (instead of failing on first)
    pub fn validate_collect(&self, value: &Value) -> Vec<ValidationError> {
        let json_value = value_to_json(value);

        self.compiled
            .iter_errors(&json_value)
            .map(|e| ValidationError {
                path: e.instance_path.to_string(),
                message: e.to_string(),
            })
            .collect()
    }

    /// Get the raw schema value
    pub fn as_value(&self) -> &serde_json::Value {
        &self.schema
    }
}

/// A single validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Path to the invalid value (e.g., "/database/port")
    pub path: String,
    /// Error message
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{}: {}", self.path, self.message)
        }
    }
}

/// Convert a holoconf Value to serde_json::Value
fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Integer(i) => serde_json::Value::Number((*i).into()),
        Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Sequence(seq) => serde_json::Value::Array(seq.iter().map(value_to_json).collect()),
        Value::Mapping(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_from_yaml() {
        let schema_yaml = r#"
type: object
required:
  - name
properties:
  name:
    type: string
  port:
    type: integer
    minimum: 1
    maximum: 65535
"#;
        let schema = Schema::from_yaml(schema_yaml).unwrap();
        assert!(schema.as_value().is_object());
    }

    #[test]
    fn test_validate_valid_config() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  name:
    type: string
  port:
    type: integer
"#,
        )
        .unwrap();

        let mut map = indexmap::IndexMap::new();
        map.insert("name".into(), Value::String("myapp".into()));
        map.insert("port".into(), Value::Integer(8080));
        let config = Value::Mapping(map);

        assert!(schema.validate(&config).is_ok());
    }

    #[test]
    fn test_validate_missing_required() {
        let schema = Schema::from_yaml(
            r#"
type: object
required:
  - name
properties:
  name:
    type: string
"#,
        )
        .unwrap();

        let config = Value::Mapping(indexmap::IndexMap::new());
        let result = schema.validate(&config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("name"));
    }

    #[test]
    fn test_validate_wrong_type() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  port:
    type: integer
"#,
        )
        .unwrap();

        let mut map = indexmap::IndexMap::new();
        map.insert("port".into(), Value::String("not-a-number".into()));
        let config = Value::Mapping(map);

        let result = schema.validate(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_constraint_violation() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  port:
    type: integer
    minimum: 1
    maximum: 65535
"#,
        )
        .unwrap();

        let mut map = indexmap::IndexMap::new();
        map.insert("port".into(), Value::Integer(70000));
        let config = Value::Mapping(map);

        let result = schema.validate(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_enum() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  log_level:
    type: string
    enum: [debug, info, warn, error]
"#,
        )
        .unwrap();

        // Valid enum value
        let mut map = indexmap::IndexMap::new();
        map.insert("log_level".into(), Value::String("info".into()));
        let config = Value::Mapping(map);
        assert!(schema.validate(&config).is_ok());

        // Invalid enum value
        let mut map = indexmap::IndexMap::new();
        map.insert("log_level".into(), Value::String("verbose".into()));
        let config = Value::Mapping(map);
        assert!(schema.validate(&config).is_err());
    }

    #[test]
    fn test_validate_nested() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  database:
    type: object
    required: [host]
    properties:
      host:
        type: string
      port:
        type: integer
        default: 5432
"#,
        )
        .unwrap();

        // Valid nested config
        let mut db = indexmap::IndexMap::new();
        db.insert("host".into(), Value::String("localhost".into()));
        db.insert("port".into(), Value::Integer(5432));
        let mut map = indexmap::IndexMap::new();
        map.insert("database".into(), Value::Mapping(db));
        let config = Value::Mapping(map);
        assert!(schema.validate(&config).is_ok());

        // Missing required nested key
        let db = indexmap::IndexMap::new();
        let mut map = indexmap::IndexMap::new();
        map.insert("database".into(), Value::Mapping(db));
        let config = Value::Mapping(map);
        assert!(schema.validate(&config).is_err());
    }

    #[test]
    fn test_validate_collect_multiple_errors() {
        let schema = Schema::from_yaml(
            r#"
type: object
required:
  - name
  - port
properties:
  name:
    type: string
  port:
    type: integer
"#,
        )
        .unwrap();

        let config = Value::Mapping(indexmap::IndexMap::new());
        let errors = schema.validate_collect(&config);
        // Should have at least one error about missing required fields
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_validate_additional_properties_allowed() {
        // By default, additional properties are allowed
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  name:
    type: string
"#,
        )
        .unwrap();

        let mut map = indexmap::IndexMap::new();
        map.insert("name".into(), Value::String("myapp".into()));
        map.insert("extra".into(), Value::String("allowed".into()));
        let config = Value::Mapping(map);
        assert!(schema.validate(&config).is_ok());
    }

    #[test]
    fn test_validate_additional_properties_denied() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  name:
    type: string
additionalProperties: false
"#,
        )
        .unwrap();

        let mut map = indexmap::IndexMap::new();
        map.insert("name".into(), Value::String("myapp".into()));
        map.insert("extra".into(), Value::String("not allowed".into()));
        let config = Value::Mapping(map);
        assert!(schema.validate(&config).is_err());
    }

    #[test]
    fn test_validate_array() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  servers:
    type: array
    items:
      type: string
"#,
        )
        .unwrap();

        let mut map = indexmap::IndexMap::new();
        map.insert(
            "servers".into(),
            Value::Sequence(vec![
                Value::String("server1".into()),
                Value::String("server2".into()),
            ]),
        );
        let config = Value::Mapping(map);
        assert!(schema.validate(&config).is_ok());

        // Wrong item type
        let mut map = indexmap::IndexMap::new();
        map.insert(
            "servers".into(),
            Value::Sequence(vec![Value::String("server1".into()), Value::Integer(123)]),
        );
        let config = Value::Mapping(map);
        assert!(schema.validate(&config).is_err());
    }

    #[test]
    fn test_validate_pattern() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  version:
    type: string
    pattern: "^\\d+\\.\\d+\\.\\d+$"
"#,
        )
        .unwrap();

        // Valid semver
        let mut map = indexmap::IndexMap::new();
        map.insert("version".into(), Value::String("1.2.3".into()));
        let config = Value::Mapping(map);
        assert!(schema.validate(&config).is_ok());

        // Invalid format
        let mut map = indexmap::IndexMap::new();
        map.insert("version".into(), Value::String("v1.2".into()));
        let config = Value::Mapping(map);
        assert!(schema.validate(&config).is_err());
    }
}

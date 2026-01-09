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

    /// Output schema as YAML
    pub fn to_yaml(&self) -> Result<String> {
        serde_yaml::to_string(&self.schema)
            .map_err(|e| Error::internal(format!("Failed to serialize schema: {}", e)))
    }

    /// Output schema as JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(&self.schema)
            .map_err(|e| Error::internal(format!("Failed to serialize schema: {}", e)))
    }

    /// Generate markdown documentation from the schema
    pub fn to_markdown(&self) -> String {
        generate_markdown_doc(&self.schema)
    }

    /// Generate a YAML template from the schema
    ///
    /// Creates a configuration template with default values and comments
    /// indicating required fields and descriptions.
    pub fn to_template(&self) -> String {
        generate_template(&self.schema)
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

/// Generate markdown documentation from a JSON Schema
fn generate_markdown_doc(schema: &serde_json::Value) -> String {
    let mut output = String::new();

    // Get title or use default
    let title = schema
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Configuration Reference");
    output.push_str(&format!("# {}\n\n", title));

    // Get top-level description
    if let Some(desc) = schema.get("description").and_then(|v| v.as_str()) {
        output.push_str(&format!("{}\n\n", desc));
    }

    // Get required fields at root level
    let root_required: Vec<&str> = schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    // Process top-level properties
    if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
        for (name, prop_schema) in properties {
            generate_section(&mut output, name, prop_schema, root_required.contains(&name.as_str()), 2);
        }
    }

    output
}

/// Generate a section for a property (potentially recursive for nested objects)
fn generate_section(
    output: &mut String,
    name: &str,
    schema: &serde_json::Value,
    is_required: bool,
    heading_level: usize,
) {
    let heading = "#".repeat(heading_level);
    let required_marker = if is_required { " (required)" } else { "" };

    // Section heading
    output.push_str(&format!("{} {}{}\n\n", heading, name, required_marker));

    // Description
    if let Some(desc) = schema.get("description").and_then(|v| v.as_str()) {
        output.push_str(&format!("{}\n\n", desc));
    }

    // Check if this is an object with nested properties
    let is_object = schema
        .get("type")
        .and_then(|v| v.as_str())
        .map(|t| t == "object")
        .unwrap_or(false);

    if is_object {
        if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
            // Get required fields for this level
            let required: Vec<&str> = schema
                .get("required")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();

            // Generate table for immediate properties
            output.push_str("| Key | Type | Required | Default | Description |\n");
            output.push_str("|-----|------|----------|---------|-------------|\n");

            for (prop_name, prop_schema) in properties {
                let prop_type = get_type_string(prop_schema);
                let prop_required = if required.contains(&prop_name.as_str()) {
                    "Yes"
                } else {
                    "No"
                };
                let prop_default = schema_default_string(prop_schema);
                let prop_desc = prop_schema
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-");

                output.push_str(&format!(
                    "| {} | {} | {} | {} | {} |\n",
                    prop_name, prop_type, prop_required, prop_default, prop_desc
                ));
            }

            output.push('\n');

            // Recursively generate sections for nested objects
            for (prop_name, prop_schema) in properties {
                let prop_is_object = prop_schema
                    .get("type")
                    .and_then(|v| v.as_str())
                    .map(|t| t == "object")
                    .unwrap_or(false);

                if prop_is_object && prop_schema.get("properties").is_some() {
                    let nested_required = required.contains(&prop_name.as_str());
                    generate_section(output, prop_name, prop_schema, nested_required, heading_level + 1);
                }
            }
        }
    } else {
        // For non-object types at top level, just show a simple table
        output.push_str("| Key | Type | Required | Default | Description |\n");
        output.push_str("|-----|------|----------|---------|-------------|\n");

        let prop_type = get_type_string(schema);
        let prop_required = if is_required { "Yes" } else { "No" };
        let prop_default = schema_default_string(schema);
        let prop_desc = schema
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("-");

        output.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n\n",
            name, prop_type, prop_required, prop_default, prop_desc
        ));
    }
}

/// Get a human-readable type string from a schema
fn get_type_string(schema: &serde_json::Value) -> String {
    // Handle enum
    if let Some(enum_vals) = schema.get("enum").and_then(|v| v.as_array()) {
        let vals: Vec<String> = enum_vals
            .iter()
            .filter_map(|v| {
                if v.is_string() {
                    Some(format!("\"{}\"", v.as_str().unwrap()))
                } else {
                    Some(v.to_string())
                }
            })
            .collect();
        return format!("enum: {}", vals.join(", "));
    }

    // Handle type
    let base_type = schema
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("any");

    // Add constraints info
    let mut constraints = Vec::new();

    if let Some(min) = schema.get("minimum") {
        constraints.push(format!("min: {}", min));
    }
    if let Some(max) = schema.get("maximum") {
        constraints.push(format!("max: {}", max));
    }
    if let Some(pattern) = schema.get("pattern").and_then(|v| v.as_str()) {
        constraints.push(format!("pattern: {}", pattern));
    }
    if let Some(min_len) = schema.get("minLength") {
        constraints.push(format!("minLength: {}", min_len));
    }
    if let Some(max_len) = schema.get("maxLength") {
        constraints.push(format!("maxLength: {}", max_len));
    }

    if constraints.is_empty() {
        base_type.to_string()
    } else {
        format!("{} ({})", base_type, constraints.join(", "))
    }
}

/// Get the default value as a string, or "-" if none
fn schema_default_string(schema: &serde_json::Value) -> String {
    schema
        .get("default")
        .map(|v| {
            if v.is_string() {
                format!("\"{}\"", v.as_str().unwrap())
            } else {
                v.to_string()
            }
        })
        .unwrap_or_else(|| "-".to_string())
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
        Value::Bytes(bytes) => {
            // Serialize bytes as base64 string
            use base64::{Engine as _, engine::general_purpose::STANDARD};
            serde_json::Value::String(STANDARD.encode(bytes))
        }
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

/// Generate a YAML template from a JSON Schema
fn generate_template(schema: &serde_json::Value) -> String {
    let mut output = String::new();

    // Get title for header comment
    if let Some(title) = schema.get("title").and_then(|v| v.as_str()) {
        output.push_str(&format!("# Generated from: {}\n", title));
    } else {
        output.push_str("# Configuration template generated from schema\n");
    }
    output.push_str("# Required fields marked with # REQUIRED\n\n");

    // Get required fields at root level
    let root_required: Vec<&str> = schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    // Process top-level properties
    if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
        for (name, prop_schema) in properties {
            let is_required = root_required.contains(&name.as_str());
            generate_template_property(&mut output, name, prop_schema, is_required, 0);
        }
    }

    output
}

/// Generate template output for a single property
fn generate_template_property(
    output: &mut String,
    name: &str,
    schema: &serde_json::Value,
    is_required: bool,
    indent_level: usize,
) {
    let indent = "  ".repeat(indent_level);

    // Build the comment parts
    let mut comment_parts = Vec::new();
    if is_required {
        comment_parts.push("REQUIRED".to_string());
    }
    if let Some(desc) = schema.get("description").and_then(|v| v.as_str()) {
        comment_parts.push(desc.to_string());
    }

    // Get the type
    let prop_type = schema.get("type").and_then(|v| v.as_str()).unwrap_or("any");

    // Handle object type
    if prop_type == "object" {
        // Write comment if any
        if !comment_parts.is_empty() {
            output.push_str(&format!("{}# {}\n", indent, comment_parts.join(" - ")));
        }
        output.push_str(&format!("{}{}:\n", indent, name));

        // Get required fields for this object
        let required: Vec<&str> = schema
            .get("required")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();

        // Process nested properties
        if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
            for (prop_name, prop_schema) in properties {
                let prop_required = required.contains(&prop_name.as_str());
                generate_template_property(output, prop_name, prop_schema, prop_required, indent_level + 1);
            }
        }
    } else {
        // For scalar types, get the default value or a placeholder
        let value_str = get_template_value(schema, prop_type);

        // Build the line
        let mut line = format!("{}{}: {}", indent, name, value_str);

        // Add inline comment if there's a description or default info
        if let Some(default) = schema.get("default") {
            if !comment_parts.is_empty() {
                line.push_str(&format!("  # {} (default: {})", comment_parts.join(" - "), format_json_value(default)));
            } else {
                line.push_str(&format!("  # default: {}", format_json_value(default)));
            }
        } else if !comment_parts.is_empty() {
            line.push_str(&format!("  # {}", comment_parts.join(" - ")));
        }

        output.push_str(&line);
        output.push('\n');
    }
}

/// Get an appropriate template value for a property
fn get_template_value(schema: &serde_json::Value, prop_type: &str) -> String {
    // Use default value if available
    if let Some(default) = schema.get("default") {
        return format_json_value(default);
    }

    // Use first enum value if it's an enum
    if let Some(enum_vals) = schema.get("enum").and_then(|v| v.as_array()) {
        if let Some(first) = enum_vals.first() {
            return format_json_value(first);
        }
    }

    // Otherwise, provide a placeholder based on type
    match prop_type {
        "string" => "\"\"".to_string(),
        "integer" => "0".to_string(),
        "number" => "0.0".to_string(),
        "boolean" => "false".to_string(),
        "array" => "[]".to_string(),
        "null" => "null".to_string(),
        _ => "null".to_string(),
    }
}

/// Format a JSON value for YAML output
fn format_json_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            // Check if we need to quote the string
            if s.is_empty() || s.contains(':') || s.contains('#') || s.starts_with(' ') || s.ends_with(' ') {
                format!("\"{}\"", s.replace('"', "\\\""))
            } else {
                s.clone()
            }
        }
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                "[]".to_string()
            } else {
                // For non-empty arrays, format as YAML flow style
                let items: Vec<String> = arr.iter().map(format_json_value).collect();
                format!("[{}]", items.join(", "))
            }
        }
        serde_json::Value::Object(_) => "{}".to_string(),
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

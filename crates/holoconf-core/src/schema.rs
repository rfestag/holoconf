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

    /// Get the default value for a config path from the schema
    ///
    /// Navigates the schema's `properties` structure to find the default
    /// value for the given dot-separated path.
    ///
    /// # Example
    /// ```
    /// # use holoconf_core::Schema;
    /// let schema = Schema::from_yaml(r#"
    /// type: object
    /// properties:
    ///   database:
    ///     type: object
    ///     properties:
    ///       port:
    ///         type: integer
    ///         default: 5432
    /// "#).unwrap();
    ///
    /// assert_eq!(schema.get_default("database.port"), Some(holoconf_core::Value::Integer(5432)));
    /// assert_eq!(schema.get_default("missing"), None);
    /// ```
    pub fn get_default(&self, path: &str) -> Option<Value> {
        if path.is_empty() {
            return self.schema.get("default").map(json_to_value);
        }

        let segments: Vec<&str> = path.split('.').collect();
        self.get_default_at_path(&self.schema, &segments)
    }

    /// Internal helper to navigate schema and find default at path
    fn get_default_at_path(&self, schema: &serde_json::Value, segments: &[&str]) -> Option<Value> {
        if segments.is_empty() {
            // At target - check for default
            return schema.get("default").map(json_to_value);
        }

        let segment = segments[0];
        let remaining = &segments[1..];

        // Navigate into properties
        if let Some(properties) = schema.get("properties") {
            if let Some(prop_schema) = properties.get(segment) {
                return self.get_default_at_path(prop_schema, remaining);
            }
        }

        None
    }

    /// Check if null is allowed for a config path in the schema
    ///
    /// Returns true if:
    /// - The schema allows `type: "null"` or `type: ["string", "null"]`
    /// - The path doesn't exist in the schema (permissive by default)
    ///
    /// # Example
    /// ```
    /// # use holoconf_core::Schema;
    /// let schema = Schema::from_yaml(r#"
    /// type: object
    /// properties:
    ///   nullable_field:
    ///     type: ["string", "null"]
    ///   non_nullable:
    ///     type: string
    /// "#).unwrap();
    ///
    /// assert!(schema.allows_null("nullable_field"));
    /// assert!(!schema.allows_null("non_nullable"));
    /// assert!(schema.allows_null("missing")); // permissive for undefined paths
    /// ```
    pub fn allows_null(&self, path: &str) -> bool {
        if path.is_empty() {
            return self.type_allows_null(&self.schema);
        }

        let segments: Vec<&str> = path.split('.').collect();
        self.allows_null_at_path(&self.schema, &segments)
    }

    /// Internal helper to check if null is allowed at a path
    fn allows_null_at_path(&self, schema: &serde_json::Value, segments: &[&str]) -> bool {
        if segments.is_empty() {
            return self.type_allows_null(schema);
        }

        let segment = segments[0];
        let remaining = &segments[1..];

        // Navigate into properties
        if let Some(properties) = schema.get("properties") {
            if let Some(prop_schema) = properties.get(segment) {
                return self.allows_null_at_path(prop_schema, remaining);
            }
        }

        // Path not found in schema - be permissive
        true
    }

    /// Check if a schema type allows null
    fn type_allows_null(&self, schema: &serde_json::Value) -> bool {
        match schema.get("type") {
            Some(serde_json::Value::String(t)) => t == "null",
            Some(serde_json::Value::Array(types)) => {
                types.iter().any(|t| t.as_str() == Some("null"))
            }
            None => true, // No type constraint means anything is allowed
            _ => false,   // Invalid type value (number, bool, object, null) - treat as not nullable
        }
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
            generate_section(
                &mut output,
                name,
                prop_schema,
                root_required.contains(&name.as_str()),
                2,
            );
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
                    generate_section(
                        output,
                        prop_name,
                        prop_schema,
                        nested_required,
                        heading_level + 1,
                    );
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
            .map(|v| {
                if v.is_string() {
                    format!("\"{}\"", v.as_str().unwrap())
                } else {
                    v.to_string()
                }
            })
            .collect();
        return format!("enum: {}", vals.join(", "));
    }

    // Handle type
    let base_type = schema.get("type").and_then(|v| v.as_str()).unwrap_or("any");

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

/// Convert a serde_json::Value to holoconf Value
fn json_to_value(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                // Fallback for very large numbers
                Value::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(arr) => Value::Sequence(arr.iter().map(json_to_value).collect()),
        serde_json::Value::Object(obj) => {
            let map: indexmap::IndexMap<String, Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), json_to_value(v)))
                .collect();
            Value::Mapping(map)
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
        Value::Bytes(bytes) => {
            // Serialize bytes as base64 string
            use base64::{engine::general_purpose::STANDARD, Engine as _};
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
                generate_template_property(
                    output,
                    prop_name,
                    prop_schema,
                    prop_required,
                    indent_level + 1,
                );
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
                line.push_str(&format!(
                    "  # {} (default: {})",
                    comment_parts.join(" - "),
                    format_json_value(default)
                ));
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
            if s.is_empty()
                || s.contains(':')
                || s.contains('#')
                || s.starts_with(' ')
                || s.ends_with(' ')
            {
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

    #[test]
    fn test_schema_from_json() {
        let schema_json = r#"{
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": { "type": "string" },
                "port": { "type": "integer" }
            }
        }"#;
        let schema = Schema::from_json(schema_json).unwrap();
        assert!(schema.as_value().is_object());

        // Validate with it
        let mut map = indexmap::IndexMap::new();
        map.insert("name".into(), Value::String("test".into()));
        let config = Value::Mapping(map);
        assert!(schema.validate(&config).is_ok());
    }

    #[test]
    fn test_schema_from_json_invalid() {
        let invalid_json = "not valid json {{{";
        let result = Schema::from_json(invalid_json);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid JSON schema"));
    }

    #[test]
    fn test_schema_from_yaml_invalid() {
        let invalid_yaml = ":: invalid yaml :::";
        let result = Schema::from_yaml(invalid_yaml);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid YAML schema"));
    }

    #[test]
    fn test_schema_from_file_yaml() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_schema.yaml");

        let schema_content = r#"
type: object
properties:
  name:
    type: string
"#;
        std::fs::write(&path, schema_content).unwrap();

        let schema = Schema::from_file(&path).unwrap();
        assert!(schema.as_value().is_object());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_schema_from_file_json() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_schema.json");

        let schema_content = r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#;
        std::fs::write(&path, schema_content).unwrap();

        let schema = Schema::from_file(&path).unwrap();
        assert!(schema.as_value().is_object());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_schema_from_file_yml_extension() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_schema.yml");

        let schema_content = r#"
type: object
properties:
  name:
    type: string
"#;
        std::fs::write(&path, schema_content).unwrap();

        let schema = Schema::from_file(&path).unwrap();
        assert!(schema.as_value().is_object());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_schema_from_file_no_extension() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_schema_no_ext");

        // Default to YAML parsing
        let schema_content = r#"
type: object
properties:
  name:
    type: string
"#;
        std::fs::write(&path, schema_content).unwrap();

        let schema = Schema::from_file(&path).unwrap();
        assert!(schema.as_value().is_object());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_schema_from_file_not_found() {
        let result = Schema::from_file("/nonexistent/path/to/schema.yaml");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }

    #[test]
    fn test_schema_to_yaml() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  name:
    type: string
"#,
        )
        .unwrap();

        let yaml = schema.to_yaml().unwrap();
        assert!(yaml.contains("type"));
        assert!(yaml.contains("object"));
        assert!(yaml.contains("properties"));
    }

    #[test]
    fn test_schema_to_json() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  name:
    type: string
"#,
        )
        .unwrap();

        let json = schema.to_json().unwrap();
        assert!(json.contains("\"type\""));
        assert!(json.contains("\"object\""));
        assert!(json.contains("\"properties\""));
    }

    #[test]
    fn test_schema_to_markdown_basic() {
        let schema = Schema::from_yaml(
            r#"
title: Test Configuration
description: A test configuration schema
type: object
required:
  - name
properties:
  name:
    type: string
    description: The application name
  port:
    type: integer
    description: The server port
    default: 8080
    minimum: 1
    maximum: 65535
"#,
        )
        .unwrap();

        let markdown = schema.to_markdown();
        assert!(markdown.contains("# Test Configuration"));
        assert!(markdown.contains("A test configuration schema"));
        assert!(markdown.contains("name"));
        assert!(markdown.contains("port"));
        assert!(markdown.contains("(required)"));
    }

    #[test]
    fn test_schema_to_markdown_nested() {
        let schema = Schema::from_yaml(
            r#"
title: Nested Config
type: object
properties:
  database:
    type: object
    description: Database settings
    required:
      - host
    properties:
      host:
        type: string
        description: Database host
      port:
        type: integer
        default: 5432
"#,
        )
        .unwrap();

        let markdown = schema.to_markdown();
        assert!(markdown.contains("database"));
        assert!(markdown.contains("host"));
        assert!(markdown.contains("5432"));
    }

    #[test]
    fn test_schema_to_markdown_enum() {
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

        let markdown = schema.to_markdown();
        assert!(markdown.contains("enum:"));
    }

    #[test]
    fn test_schema_to_markdown_constraints() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  name:
    type: string
    minLength: 1
    maxLength: 100
    pattern: "^[a-z]+$"
"#,
        )
        .unwrap();

        let markdown = schema.to_markdown();
        assert!(markdown.contains("minLength"));
        assert!(markdown.contains("maxLength"));
        assert!(markdown.contains("pattern"));
    }

    #[test]
    fn test_schema_to_template_basic() {
        let schema = Schema::from_yaml(
            r#"
title: Test Config
type: object
required:
  - name
properties:
  name:
    type: string
    description: The application name
  port:
    type: integer
    default: 8080
"#,
        )
        .unwrap();

        let template = schema.to_template();
        assert!(template.contains("name:"));
        assert!(template.contains("port:"));
        assert!(template.contains("8080"));
        assert!(template.contains("REQUIRED"));
    }

    #[test]
    fn test_schema_to_template_nested() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  database:
    type: object
    required:
      - host
    properties:
      host:
        type: string
        description: Database host
      port:
        type: integer
        default: 5432
"#,
        )
        .unwrap();

        let template = schema.to_template();
        assert!(template.contains("database:"));
        assert!(template.contains("host:"));
        assert!(template.contains("port:"));
        assert!(template.contains("5432"));
    }

    #[test]
    fn test_schema_to_template_types() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  string_field:
    type: string
  int_field:
    type: integer
  number_field:
    type: number
  bool_field:
    type: boolean
  array_field:
    type: array
  null_field:
    type: "null"
"#,
        )
        .unwrap();

        let template = schema.to_template();
        assert!(template.contains("string_field: \"\""));
        assert!(template.contains("int_field: 0"));
        assert!(template.contains("number_field: 0.0"));
        assert!(template.contains("bool_field: false"));
        assert!(template.contains("array_field: []"));
        assert!(template.contains("null_field: null"));
    }

    #[test]
    fn test_schema_to_template_enum() {
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

        let template = schema.to_template();
        // Should use first enum value as default
        assert!(template.contains("log_level: debug") || template.contains("log_level: \"debug\""));
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError {
            path: "/database/port".to_string(),
            message: "expected integer".to_string(),
        };
        let display = format!("{}", err);
        assert_eq!(display, "/database/port: expected integer");
    }

    #[test]
    fn test_validation_error_display_empty_path() {
        let err = ValidationError {
            path: "".to_string(),
            message: "missing required field".to_string(),
        };
        let display = format!("{}", err);
        assert_eq!(display, "missing required field");
    }

    #[test]
    fn test_value_to_json_null() {
        let v = Value::Null;
        let json = value_to_json(&v);
        assert!(json.is_null());
    }

    #[test]
    fn test_value_to_json_bool() {
        let v = Value::Bool(true);
        let json = value_to_json(&v);
        assert_eq!(json, serde_json::Value::Bool(true));
    }

    #[test]
    fn test_value_to_json_integer() {
        let v = Value::Integer(42);
        let json = value_to_json(&v);
        assert_eq!(json, serde_json::json!(42));
    }

    #[test]
    fn test_value_to_json_float() {
        let v = Value::Float(2.71);
        let json = value_to_json(&v);
        assert!(json.is_number());
    }

    #[test]
    fn test_value_to_json_float_nan() {
        // NaN cannot be represented in JSON, should return null
        let v = Value::Float(f64::NAN);
        let json = value_to_json(&v);
        assert!(json.is_null());
    }

    #[test]
    fn test_value_to_json_string() {
        let v = Value::String("hello".into());
        let json = value_to_json(&v);
        assert_eq!(json, serde_json::json!("hello"));
    }

    #[test]
    fn test_value_to_json_bytes() {
        let v = Value::Bytes(vec![72, 101, 108, 108, 111]); // "Hello"
        let json = value_to_json(&v);
        // Should be base64 encoded
        assert!(json.is_string());
        assert_eq!(json.as_str().unwrap(), "SGVsbG8=");
    }

    #[test]
    fn test_value_to_json_sequence() {
        let v = Value::Sequence(vec![Value::Integer(1), Value::Integer(2)]);
        let json = value_to_json(&v);
        assert!(json.is_array());
        assert_eq!(json, serde_json::json!([1, 2]));
    }

    #[test]
    fn test_value_to_json_mapping() {
        let mut map = indexmap::IndexMap::new();
        map.insert("key".to_string(), Value::String("value".into()));
        let v = Value::Mapping(map);
        let json = value_to_json(&v);
        assert!(json.is_object());
        assert_eq!(json["key"], "value");
    }

    #[test]
    fn test_format_json_value_null() {
        let v = serde_json::Value::Null;
        assert_eq!(format_json_value(&v), "null");
    }

    #[test]
    fn test_format_json_value_bool() {
        assert_eq!(format_json_value(&serde_json::json!(true)), "true");
        assert_eq!(format_json_value(&serde_json::json!(false)), "false");
    }

    #[test]
    fn test_format_json_value_number() {
        assert_eq!(format_json_value(&serde_json::json!(42)), "42");
        assert_eq!(format_json_value(&serde_json::json!(2.71)), "2.71");
    }

    #[test]
    fn test_format_json_value_string_simple() {
        assert_eq!(format_json_value(&serde_json::json!("hello")), "hello");
    }

    #[test]
    fn test_format_json_value_string_needs_quoting() {
        // Empty string needs quotes
        assert_eq!(format_json_value(&serde_json::json!("")), "\"\"");
        // Contains colon
        assert_eq!(
            format_json_value(&serde_json::json!("key:value")),
            "\"key:value\""
        );
        // Contains hash
        assert_eq!(
            format_json_value(&serde_json::json!("has#comment")),
            "\"has#comment\""
        );
        // Starts with space
        assert_eq!(
            format_json_value(&serde_json::json!(" leading")),
            "\" leading\""
        );
        // Ends with space
        assert_eq!(
            format_json_value(&serde_json::json!("trailing ")),
            "\"trailing \""
        );
    }

    #[test]
    fn test_format_json_value_string_with_quotes_needing_escape() {
        // String that needs quoting AND contains quotes should escape them
        // Empty string triggers quoting, so let's test that
        let v = serde_json::Value::String("has:\"quotes\"".to_string());
        let formatted = format_json_value(&v);
        // The colon triggers quoting, and the quotes get escaped
        assert!(formatted.contains("\\\""));
        assert!(formatted.starts_with('"'));
    }

    #[test]
    fn test_format_json_value_string_no_quoting_needed() {
        // String without special chars doesn't get quoted
        let v = serde_json::Value::String("has \"quotes\"".to_string());
        let formatted = format_json_value(&v);
        // No colon/hash/spaces so it's returned as-is without quoting
        assert_eq!(formatted, "has \"quotes\"");
    }

    #[test]
    fn test_format_json_value_array_empty() {
        assert_eq!(format_json_value(&serde_json::json!([])), "[]");
    }

    #[test]
    fn test_format_json_value_array_with_items() {
        assert_eq!(
            format_json_value(&serde_json::json!([1, 2, 3])),
            "[1, 2, 3]"
        );
    }

    #[test]
    fn test_format_json_value_object() {
        assert_eq!(format_json_value(&serde_json::json!({})), "{}");
    }

    #[test]
    fn test_get_type_string_basic() {
        let schema = serde_json::json!({"type": "string"});
        assert_eq!(get_type_string(&schema), "string");
    }

    #[test]
    fn test_get_type_string_with_constraints() {
        let schema = serde_json::json!({
            "type": "integer",
            "minimum": 1,
            "maximum": 100
        });
        let type_str = get_type_string(&schema);
        assert!(type_str.contains("integer"));
        assert!(type_str.contains("min: 1"));
        assert!(type_str.contains("max: 100"));
    }

    #[test]
    fn test_get_type_string_with_string_constraints() {
        let schema = serde_json::json!({
            "type": "string",
            "minLength": 1,
            "maxLength": 50,
            "pattern": "^[a-z]+$"
        });
        let type_str = get_type_string(&schema);
        assert!(type_str.contains("string"));
        assert!(type_str.contains("minLength: 1"));
        assert!(type_str.contains("maxLength: 50"));
        assert!(type_str.contains("pattern:"));
    }

    #[test]
    fn test_get_type_string_enum() {
        let schema = serde_json::json!({
            "enum": ["a", "b", "c"]
        });
        let type_str = get_type_string(&schema);
        assert!(type_str.starts_with("enum:"));
        assert!(type_str.contains("\"a\""));
        assert!(type_str.contains("\"b\""));
    }

    #[test]
    fn test_get_type_string_enum_numeric() {
        let schema = serde_json::json!({
            "enum": [1, 2, 3]
        });
        let type_str = get_type_string(&schema);
        assert!(type_str.contains("1"));
        assert!(type_str.contains("2"));
    }

    #[test]
    fn test_get_type_string_no_type() {
        let schema = serde_json::json!({});
        assert_eq!(get_type_string(&schema), "any");
    }

    #[test]
    fn test_schema_default_string_with_default() {
        let schema = serde_json::json!({"default": 42});
        assert_eq!(schema_default_string(&schema), "42");
    }

    #[test]
    fn test_schema_default_string_with_string_default() {
        let schema = serde_json::json!({"default": "hello"});
        assert_eq!(schema_default_string(&schema), "\"hello\"");
    }

    #[test]
    fn test_schema_default_string_no_default() {
        let schema = serde_json::json!({});
        assert_eq!(schema_default_string(&schema), "-");
    }

    #[test]
    fn test_get_template_value_with_default() {
        let schema = serde_json::json!({"type": "string", "default": "myvalue"});
        assert_eq!(get_template_value(&schema, "string"), "myvalue");
    }

    #[test]
    fn test_get_template_value_with_enum() {
        let schema = serde_json::json!({"type": "string", "enum": ["first", "second"]});
        assert_eq!(get_template_value(&schema, "string"), "first");
    }

    #[test]
    fn test_get_template_value_placeholders() {
        assert_eq!(get_template_value(&serde_json::json!({}), "string"), "\"\"");
        assert_eq!(get_template_value(&serde_json::json!({}), "integer"), "0");
        assert_eq!(get_template_value(&serde_json::json!({}), "number"), "0.0");
        assert_eq!(
            get_template_value(&serde_json::json!({}), "boolean"),
            "false"
        );
        assert_eq!(get_template_value(&serde_json::json!({}), "array"), "[]");
        assert_eq!(get_template_value(&serde_json::json!({}), "null"), "null");
        assert_eq!(
            get_template_value(&serde_json::json!({}), "unknown"),
            "null"
        );
    }

    #[test]
    fn test_schema_to_markdown_no_title() {
        // Schema without title should use default
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  name:
    type: string
"#,
        )
        .unwrap();

        let markdown = schema.to_markdown();
        assert!(markdown.contains("# Configuration Reference"));
    }

    #[test]
    fn test_schema_to_markdown_non_object_property() {
        // Top-level property that is not an object
        let schema = Schema::from_yaml(
            r#"
type: object
required:
  - port
properties:
  port:
    type: integer
    description: Server port
"#,
        )
        .unwrap();

        let markdown = schema.to_markdown();
        assert!(markdown.contains("port"));
        assert!(markdown.contains("(required)"));
    }

    #[test]
    fn test_schema_to_template_no_title() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  name:
    type: string
"#,
        )
        .unwrap();

        let template = schema.to_template();
        assert!(template.contains("Configuration template generated from schema"));
    }

    #[test]
    fn test_schema_to_template_with_description() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  name:
    type: string
    description: The name field
"#,
        )
        .unwrap();

        let template = schema.to_template();
        assert!(template.contains("The name field"));
    }

    #[test]
    fn test_schema_to_template_with_default_and_description() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  port:
    type: integer
    description: Server port
    default: 8080
"#,
        )
        .unwrap();

        let template = schema.to_template();
        assert!(template.contains("8080"));
        assert!(template.contains("default:"));
    }

    // Tests for get_default() and allows_null()

    #[test]
    fn test_get_default_simple() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  pool_size:
    type: integer
    default: 10
  timeout:
    type: number
    default: 30.5
  enabled:
    type: boolean
    default: true
  name:
    type: string
    default: "default_name"
"#,
        )
        .unwrap();

        assert_eq!(schema.get_default("pool_size"), Some(Value::Integer(10)));
        assert_eq!(schema.get_default("timeout"), Some(Value::Float(30.5)));
        assert_eq!(schema.get_default("enabled"), Some(Value::Bool(true)));
        assert_eq!(
            schema.get_default("name"),
            Some(Value::String("default_name".into()))
        );
        assert_eq!(schema.get_default("nonexistent"), None);
    }

    #[test]
    fn test_get_default_nested() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  database:
    type: object
    properties:
      host:
        type: string
        default: localhost
      port:
        type: integer
        default: 5432
      pool:
        type: object
        properties:
          size:
            type: integer
            default: 10
"#,
        )
        .unwrap();

        assert_eq!(
            schema.get_default("database.host"),
            Some(Value::String("localhost".into()))
        );
        assert_eq!(
            schema.get_default("database.port"),
            Some(Value::Integer(5432))
        );
        assert_eq!(
            schema.get_default("database.pool.size"),
            Some(Value::Integer(10))
        );
        assert_eq!(schema.get_default("database.nonexistent"), None);
    }

    #[test]
    fn test_get_default_object_level() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  logging:
    type: object
    default:
      level: info
      format: json
"#,
        )
        .unwrap();

        let default = schema.get_default("logging").unwrap();
        match default {
            Value::Mapping(map) => {
                assert_eq!(map.get("level"), Some(&Value::String("info".into())));
                assert_eq!(map.get("format"), Some(&Value::String("json".into())));
            }
            _ => panic!("Expected mapping default"),
        }
    }

    #[test]
    fn test_get_default_null_default() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  optional_value:
    type:
      - string
      - "null"
    default: null
"#,
        )
        .unwrap();

        assert_eq!(schema.get_default("optional_value"), Some(Value::Null));
    }

    #[test]
    fn test_allows_null_single_type() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  required_string:
    type: string
  nullable_string:
    type: "null"
"#,
        )
        .unwrap();

        assert!(!schema.allows_null("required_string"));
        assert!(schema.allows_null("nullable_string"));
    }

    #[test]
    fn test_allows_null_array_type() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  nullable_value:
    type:
      - string
      - "null"
  non_nullable:
    type:
      - string
      - integer
"#,
        )
        .unwrap();

        assert!(schema.allows_null("nullable_value"));
        assert!(!schema.allows_null("non_nullable"));
    }

    #[test]
    fn test_allows_null_nested() {
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  database:
    type: object
    properties:
      connection_string:
        type:
          - string
          - "null"
        default: null
"#,
        )
        .unwrap();

        assert!(schema.allows_null("database.connection_string"));
    }

    #[test]
    fn test_allows_null_no_type_specified() {
        // When type is not specified, null is implicitly allowed
        let schema = Schema::from_yaml(
            r#"
type: object
properties:
  any_value: {}
"#,
        )
        .unwrap();

        assert!(schema.allows_null("any_value"));
    }
}

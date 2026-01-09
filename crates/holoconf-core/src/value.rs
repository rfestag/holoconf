//! Configuration value types
//!
//! Represents parsed configuration values before resolution.
//! Values can be scalars (string, int, float, bool, null),
//! sequences (arrays), or mappings (objects).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::{Error, Result};

/// A configuration value that may contain unresolved interpolations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[derive(Default)]
pub enum Value {
    /// Null value
    #[default]
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value
    Integer(i64),
    /// Floating point value
    Float(f64),
    /// String value (may contain interpolations like ${env:VAR})
    String(String),
    /// Sequence of values
    Sequence(Vec<Value>),
    /// Mapping of string keys to values
    Mapping(IndexMap<String, Value>),
}

impl Value {
    /// Check if this value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Check if this value is a boolean
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// Check if this value is an integer
    pub fn is_integer(&self) -> bool {
        matches!(self, Value::Integer(_))
    }

    /// Check if this value is a float
    pub fn is_float(&self) -> bool {
        matches!(self, Value::Float(_))
    }

    /// Check if this value is a string
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Check if this value is a sequence
    pub fn is_sequence(&self) -> bool {
        matches!(self, Value::Sequence(_))
    }

    /// Check if this value is a mapping
    pub fn is_mapping(&self) -> bool {
        matches!(self, Value::Mapping(_))
    }

    /// Get as boolean if this is a Bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get as i64 if this is an Integer
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Get as f64 if this is a Float or Integer
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Get as str if this is a String
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get as slice if this is a Sequence
    pub fn as_sequence(&self) -> Option<&[Value]> {
        match self {
            Value::Sequence(s) => Some(s),
            _ => None,
        }
    }

    /// Get as mapping if this is a Mapping
    pub fn as_mapping(&self) -> Option<&IndexMap<String, Value>> {
        match self {
            Value::Mapping(m) => Some(m),
            _ => None,
        }
    }

    /// Get a value by path (e.g., "database.host" or "servers[0].name")
    pub fn get_path(&self, path: &str) -> Result<&Value> {
        if path.is_empty() {
            return Ok(self);
        }

        let segments = parse_path(path)?;
        let mut current = self;

        for segment in &segments {
            current = match segment {
                PathSegment::Key(key) => match current {
                    Value::Mapping(map) => map
                        .get(key.as_str())
                        .ok_or_else(|| Error::path_not_found(path))?,
                    _ => return Err(Error::path_not_found(path)),
                },
                PathSegment::Index(idx) => match current {
                    Value::Sequence(seq) => {
                        seq.get(*idx).ok_or_else(|| Error::path_not_found(path))?
                    }
                    _ => return Err(Error::path_not_found(path)),
                },
            };
        }

        Ok(current)
    }

    /// Get a mutable value by path
    pub fn get_path_mut(&mut self, path: &str) -> Result<&mut Value> {
        if path.is_empty() {
            return Ok(self);
        }

        let segments = parse_path(path)?;
        let mut current = self;

        for segment in segments {
            current = match segment {
                PathSegment::Key(key) => match current {
                    Value::Mapping(map) => map
                        .get_mut(&key)
                        .ok_or_else(|| Error::path_not_found(path))?,
                    _ => return Err(Error::path_not_found(path)),
                },
                PathSegment::Index(idx) => match current {
                    Value::Sequence(seq) => seq
                        .get_mut(idx)
                        .ok_or_else(|| Error::path_not_found(path))?,
                    _ => return Err(Error::path_not_found(path)),
                },
            };
        }

        Ok(current)
    }

    /// Set a value at a path, creating intermediate mappings as needed
    pub fn set_path(&mut self, path: &str, value: Value) -> Result<()> {
        if path.is_empty() {
            *self = value;
            return Ok(());
        }

        let segments = parse_path(path)?;
        let mut current = self;

        for (i, segment) in segments.iter().enumerate() {
            let is_last = i == segments.len() - 1;

            if is_last {
                match segment {
                    PathSegment::Key(key) => {
                        if let Value::Mapping(map) = current {
                            map.insert(key.clone(), value);
                            return Ok(());
                        }
                        return Err(Error::path_not_found(path));
                    }
                    PathSegment::Index(idx) => {
                        if let Value::Sequence(seq) = current {
                            if *idx < seq.len() {
                                seq[*idx] = value;
                                return Ok(());
                            }
                        }
                        return Err(Error::path_not_found(path));
                    }
                }
            }

            // Navigate to next level, creating mappings if needed
            current = match segment {
                PathSegment::Key(key) => {
                    if let Value::Mapping(map) = current {
                        // Check what the next segment expects
                        let next_is_index = segments
                            .get(i + 1)
                            .map(|s| matches!(s, PathSegment::Index(_)))
                            .unwrap_or(false);

                        if !map.contains_key(key) {
                            let new_value = if next_is_index {
                                Value::Sequence(vec![])
                            } else {
                                Value::Mapping(IndexMap::new())
                            };
                            map.insert(key.clone(), new_value);
                        }
                        map.get_mut(key).unwrap()
                    } else {
                        return Err(Error::path_not_found(path));
                    }
                }
                PathSegment::Index(idx) => {
                    if let Value::Sequence(seq) = current {
                        seq.get_mut(*idx)
                            .ok_or_else(|| Error::path_not_found(path))?
                    } else {
                        return Err(Error::path_not_found(path));
                    }
                }
            };
        }

        Ok(())
    }

    /// Returns the type name of this value
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Integer(_) => "integer",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Sequence(_) => "sequence",
            Value::Mapping(_) => "mapping",
        }
    }

    /// Merge another value into this one (deep merge per ADR-004)
    ///
    /// Merge semantics:
    /// - Mappings: Deep merge recursively
    /// - Scalars: `other` wins (last-writer-wins)
    /// - Sequences: `other` replaces entirely
    /// - Null in other: Removes the key (handled by caller)
    /// - Type mismatch: `other` wins
    pub fn merge(&mut self, other: Value) {
        match (self, other) {
            // Both are mappings: deep merge
            (Value::Mapping(base), Value::Mapping(overlay)) => {
                for (key, overlay_value) in overlay {
                    if overlay_value.is_null() {
                        // Null removes the key
                        base.shift_remove(&key);
                    } else if let Some(base_value) = base.get_mut(&key) {
                        // Key exists in both: recursive merge
                        base_value.merge(overlay_value);
                    } else {
                        // Key only in overlay: add it
                        base.insert(key, overlay_value);
                    }
                }
            }
            // Any other combination: overlay wins (replacement)
            (this, other) => {
                *this = other;
            }
        }
    }

    /// Create a merged value from two values (non-mutating)
    pub fn merged(mut self, other: Value) -> Value {
        self.merge(other);
        self
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Sequence(seq) => {
                write!(f, "[")?;
                for (i, v) in seq.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Mapping(map) => {
                write!(f, "{{")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
        }
    }
}

// Convenient From implementations
impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Integer(i)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Integer(i as i64)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Sequence(v.into_iter().map(Into::into).collect())
    }
}

impl From<IndexMap<String, Value>> for Value {
    fn from(m: IndexMap<String, Value>) -> Self {
        Value::Mapping(m)
    }
}

/// A segment in a path expression
#[derive(Debug, Clone, PartialEq)]
enum PathSegment {
    /// A key in a mapping (e.g., "database" in "database.host")
    Key(String),
    /// An index in a sequence (e.g., 0 in "servers[0]")
    Index(usize),
}

/// Parse a path string into segments
/// Supports: "key", "key.subkey", "key[0]", "key[0].subkey"
fn parse_path(path: &str) -> Result<Vec<PathSegment>> {
    let mut segments = Vec::new();
    let mut current_key = String::new();
    let mut chars = path.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '.' => {
                if !current_key.is_empty() {
                    segments.push(PathSegment::Key(current_key.clone()));
                    current_key.clear();
                }
            }
            '[' => {
                if !current_key.is_empty() {
                    segments.push(PathSegment::Key(current_key.clone()));
                    current_key.clear();
                }
                // Parse index
                let mut index_str = String::new();
                while let Some(&c) = chars.peek() {
                    if c == ']' {
                        chars.next();
                        break;
                    }
                    index_str.push(chars.next().unwrap());
                }
                let idx: usize = index_str.parse().map_err(|_| {
                    Error::parse(format!("Invalid array index in path: {}", index_str))
                })?;
                segments.push(PathSegment::Index(idx));
            }
            ']' => {
                return Err(Error::parse("Unexpected ']' in path"));
            }
            _ => {
                current_key.push(c);
            }
        }
    }

    if !current_key.is_empty() {
        segments.push(PathSegment::Key(current_key));
    }

    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_path() {
        let segments = parse_path("database").unwrap();
        assert_eq!(segments, vec![PathSegment::Key("database".into())]);
    }

    #[test]
    fn test_parse_dotted_path() {
        let segments = parse_path("database.host").unwrap();
        assert_eq!(
            segments,
            vec![
                PathSegment::Key("database".into()),
                PathSegment::Key("host".into())
            ]
        );
    }

    #[test]
    fn test_parse_array_path() {
        let segments = parse_path("servers[0]").unwrap();
        assert_eq!(
            segments,
            vec![PathSegment::Key("servers".into()), PathSegment::Index(0)]
        );
    }

    #[test]
    fn test_parse_complex_path() {
        let segments = parse_path("servers[0].host").unwrap();
        assert_eq!(
            segments,
            vec![
                PathSegment::Key("servers".into()),
                PathSegment::Index(0),
                PathSegment::Key("host".into())
            ]
        );
    }

    #[test]
    fn test_value_get_path() {
        let mut map = IndexMap::new();
        let mut db = IndexMap::new();
        db.insert("host".into(), Value::String("localhost".into()));
        db.insert("port".into(), Value::Integer(5432));
        map.insert("database".into(), Value::Mapping(db));

        let value = Value::Mapping(map);

        assert_eq!(
            value.get_path("database.host").unwrap().as_str(),
            Some("localhost")
        );
        assert_eq!(
            value.get_path("database.port").unwrap().as_i64(),
            Some(5432)
        );
    }

    #[test]
    fn test_value_get_path_array() {
        let mut map = IndexMap::new();
        map.insert(
            "servers".into(),
            Value::Sequence(vec![
                Value::String("server1".into()),
                Value::String("server2".into()),
            ]),
        );

        let value = Value::Mapping(map);

        assert_eq!(
            value.get_path("servers[0]").unwrap().as_str(),
            Some("server1")
        );
        assert_eq!(
            value.get_path("servers[1]").unwrap().as_str(),
            Some("server2")
        );
    }

    #[test]
    fn test_value_get_path_not_found() {
        let map = IndexMap::new();
        let value = Value::Mapping(map);

        assert!(value.get_path("nonexistent").is_err());
    }

    #[test]
    fn test_value_type_checks() {
        assert!(Value::Null.is_null());
        assert!(Value::Bool(true).is_bool());
        assert!(Value::Integer(42).is_integer());
        assert!(Value::Float(2.5).is_float());
        assert!(Value::String("hello".into()).is_string());
        assert!(Value::Sequence(vec![]).is_sequence());
        assert!(Value::Mapping(IndexMap::new()).is_mapping());
    }

    #[test]
    fn test_value_conversions() {
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
        assert_eq!(Value::Integer(42).as_i64(), Some(42));
        assert_eq!(Value::Float(2.5).as_f64(), Some(2.5));
        assert_eq!(Value::Integer(42).as_f64(), Some(42.0));
        assert_eq!(Value::String("hello".into()).as_str(), Some("hello"));
    }

    #[test]
    fn test_merge_scalars() {
        let mut base = Value::String("base".into());
        base.merge(Value::String("overlay".into()));
        assert_eq!(base.as_str(), Some("overlay"));
    }

    #[test]
    fn test_merge_deep() {
        // Create base: { database: { host: "localhost", port: 5432 } }
        let mut db_base = IndexMap::new();
        db_base.insert("host".into(), Value::String("localhost".into()));
        db_base.insert("port".into(), Value::Integer(5432));
        let mut base = IndexMap::new();
        base.insert("database".into(), Value::Mapping(db_base));
        let mut base = Value::Mapping(base);

        // Create overlay: { database: { host: "prod-db" } }
        let mut db_overlay = IndexMap::new();
        db_overlay.insert("host".into(), Value::String("prod-db".into()));
        let mut overlay = IndexMap::new();
        overlay.insert("database".into(), Value::Mapping(db_overlay));
        let overlay = Value::Mapping(overlay);

        base.merge(overlay);

        // Result should have both host (overwritten) and port (preserved)
        assert_eq!(
            base.get_path("database.host").unwrap().as_str(),
            Some("prod-db")
        );
        assert_eq!(base.get_path("database.port").unwrap().as_i64(), Some(5432));
    }

    #[test]
    fn test_merge_null_removes_key() {
        // Create base: { feature: { enabled: true, config: "value" } }
        let mut feature = IndexMap::new();
        feature.insert("enabled".into(), Value::Bool(true));
        feature.insert("config".into(), Value::String("value".into()));
        let mut base = IndexMap::new();
        base.insert("feature".into(), Value::Mapping(feature));
        let mut base = Value::Mapping(base);

        // Create overlay: { feature: { config: null } }
        let mut feature_overlay = IndexMap::new();
        feature_overlay.insert("config".into(), Value::Null);
        let mut overlay = IndexMap::new();
        overlay.insert("feature".into(), Value::Mapping(feature_overlay));
        let overlay = Value::Mapping(overlay);

        base.merge(overlay);

        // config should be removed, enabled preserved
        assert_eq!(
            base.get_path("feature.enabled").unwrap().as_bool(),
            Some(true)
        );
        assert!(base.get_path("feature.config").is_err());
    }

    #[test]
    fn test_merge_array_replaces() {
        // Create base: { servers: ["a", "b"] }
        let mut base = IndexMap::new();
        base.insert(
            "servers".into(),
            Value::Sequence(vec![Value::String("a".into()), Value::String("b".into())]),
        );
        let mut base = Value::Mapping(base);

        // Create overlay: { servers: ["c"] }
        let mut overlay = IndexMap::new();
        overlay.insert(
            "servers".into(),
            Value::Sequence(vec![Value::String("c".into())]),
        );
        let overlay = Value::Mapping(overlay);

        base.merge(overlay);

        // Array should be replaced, not concatenated
        let servers = base.get_path("servers").unwrap().as_sequence().unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].as_str(), Some("c"));
    }

    #[test]
    fn test_merge_type_mismatch() {
        // Create base: { database: { host: "localhost" } }
        let mut db = IndexMap::new();
        db.insert("host".into(), Value::String("localhost".into()));
        let mut base = IndexMap::new();
        base.insert("database".into(), Value::Mapping(db));
        let mut base = Value::Mapping(base);

        // Create overlay: { database: "connection-string" }
        let mut overlay = IndexMap::new();
        overlay.insert("database".into(), Value::String("connection-string".into()));
        let overlay = Value::Mapping(overlay);

        base.merge(overlay);

        // Scalar should replace mapping
        assert_eq!(
            base.get_path("database").unwrap().as_str(),
            Some("connection-string")
        );
    }

    #[test]
    fn test_merge_adds_new_keys() {
        // Create base: { a: 1 }
        let mut base = IndexMap::new();
        base.insert("a".into(), Value::Integer(1));
        let mut base = Value::Mapping(base);

        // Create overlay: { b: 2 }
        let mut overlay = IndexMap::new();
        overlay.insert("b".into(), Value::Integer(2));
        let overlay = Value::Mapping(overlay);

        base.merge(overlay);

        // Both keys should exist
        assert_eq!(base.get_path("a").unwrap().as_i64(), Some(1));
        assert_eq!(base.get_path("b").unwrap().as_i64(), Some(2));
    }

    // Additional tests for improved coverage

    #[test]
    fn test_from_i32() {
        let v: Value = 42i32.into();
        assert_eq!(v, Value::Integer(42));
    }

    #[test]
    fn test_from_vec_values() {
        let vec: Vec<Value> = vec![Value::Integer(1), Value::Integer(2)];
        let v: Value = vec.into();
        assert!(v.is_sequence());
        assert_eq!(v.as_sequence().unwrap().len(), 2);
    }

    #[test]
    fn test_from_vec_strings() {
        let vec: Vec<&str> = vec!["a", "b", "c"];
        let v: Value = vec.into();
        assert!(v.is_sequence());
        assert_eq!(v.as_sequence().unwrap().len(), 3);
    }

    #[test]
    fn test_from_indexmap() {
        let mut map = IndexMap::new();
        map.insert("key".to_string(), Value::Integer(42));
        let v: Value = map.into();
        assert!(v.is_mapping());
    }

    #[test]
    fn test_display_null() {
        assert_eq!(format!("{}", Value::Null), "null");
    }

    #[test]
    fn test_display_bool() {
        assert_eq!(format!("{}", Value::Bool(true)), "true");
        assert_eq!(format!("{}", Value::Bool(false)), "false");
    }

    #[test]
    fn test_display_integer() {
        assert_eq!(format!("{}", Value::Integer(42)), "42");
        assert_eq!(format!("{}", Value::Integer(-123)), "-123");
    }

    #[test]
    fn test_display_float() {
        let display = format!("{}", Value::Float(1.5));
        assert!(display.starts_with("1.5"));
    }

    #[test]
    fn test_display_string() {
        assert_eq!(format!("{}", Value::String("hello".into())), "hello");
    }

    #[test]
    fn test_display_sequence() {
        let seq = Value::Sequence(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        assert_eq!(format!("{}", seq), "[1, 2, 3]");
    }

    #[test]
    fn test_display_empty_sequence() {
        let seq = Value::Sequence(vec![]);
        assert_eq!(format!("{}", seq), "[]");
    }

    #[test]
    fn test_display_mapping() {
        let mut map = IndexMap::new();
        map.insert("a".to_string(), Value::Integer(1));
        let mapping = Value::Mapping(map);
        assert_eq!(format!("{}", mapping), "{a: 1}");
    }

    #[test]
    fn test_display_empty_mapping() {
        let mapping = Value::Mapping(IndexMap::new());
        assert_eq!(format!("{}", mapping), "{}");
    }

    #[test]
    fn test_as_str_non_string() {
        assert!(Value::Integer(42).as_str().is_none());
        assert!(Value::Bool(true).as_str().is_none());
        assert!(Value::Null.as_str().is_none());
    }

    #[test]
    fn test_as_bool_non_bool() {
        assert!(Value::Integer(42).as_bool().is_none());
        assert!(Value::String("true".into()).as_bool().is_none());
    }

    #[test]
    fn test_as_i64_non_integer() {
        assert!(Value::String("42".into()).as_i64().is_none());
        assert!(Value::Float(42.0).as_i64().is_none());
    }

    #[test]
    fn test_as_f64_non_numeric() {
        assert!(Value::String("3.14".into()).as_f64().is_none());
        assert!(Value::Bool(true).as_f64().is_none());
    }

    #[test]
    fn test_as_sequence_non_sequence() {
        assert!(Value::Integer(42).as_sequence().is_none());
        assert!(Value::Mapping(IndexMap::new()).as_sequence().is_none());
    }

    #[test]
    fn test_as_mapping_non_mapping() {
        assert!(Value::Integer(42).as_mapping().is_none());
        assert!(Value::Sequence(vec![]).as_mapping().is_none());
    }

    #[test]
    fn test_type_name() {
        assert_eq!(Value::Null.type_name(), "null");
        assert_eq!(Value::Bool(true).type_name(), "boolean");
        assert_eq!(Value::Integer(42).type_name(), "integer");
        assert_eq!(Value::Float(1.23).type_name(), "float");
        assert_eq!(Value::String("s".into()).type_name(), "string");
        assert_eq!(Value::Sequence(vec![]).type_name(), "sequence");
        assert_eq!(Value::Mapping(IndexMap::new()).type_name(), "mapping");
    }

    #[test]
    fn test_default() {
        let v: Value = Default::default();
        assert!(v.is_null());
    }

    #[test]
    fn test_get_path_empty() {
        let v = Value::Integer(42);
        assert_eq!(v.get_path("").unwrap(), &Value::Integer(42));
    }

    #[test]
    fn test_get_path_on_non_mapping() {
        let v = Value::Integer(42);
        assert!(v.get_path("key").is_err());
    }

    #[test]
    fn test_get_path_array_out_of_bounds() {
        let mut map = IndexMap::new();
        map.insert(
            "items".to_string(),
            Value::Sequence(vec![Value::Integer(1)]),
        );
        let v = Value::Mapping(map);
        assert!(v.get_path("items[99]").is_err());
    }

    #[test]
    fn test_get_path_array_on_non_sequence() {
        let mut map = IndexMap::new();
        map.insert("key".to_string(), Value::Integer(42));
        let v = Value::Mapping(map);
        assert!(v.get_path("key[0]").is_err());
    }

    #[test]
    fn test_get_path_mut_empty() {
        let mut v = Value::Integer(42);
        let result = v.get_path_mut("").unwrap();
        *result = Value::Integer(100);
        assert_eq!(v, Value::Integer(100));
    }

    #[test]
    fn test_get_path_mut_modify() {
        let mut map = IndexMap::new();
        map.insert("key".to_string(), Value::Integer(42));
        let mut v = Value::Mapping(map);

        *v.get_path_mut("key").unwrap() = Value::Integer(100);
        assert_eq!(v.get_path("key").unwrap().as_i64(), Some(100));
    }

    #[test]
    fn test_get_path_mut_not_found() {
        let mut map = IndexMap::new();
        map.insert("key".to_string(), Value::Integer(42));
        let mut v = Value::Mapping(map);
        assert!(v.get_path_mut("nonexistent").is_err());
    }

    #[test]
    fn test_get_path_mut_on_non_mapping() {
        let mut v = Value::Integer(42);
        assert!(v.get_path_mut("key").is_err());
    }

    #[test]
    fn test_get_path_mut_array() {
        let mut map = IndexMap::new();
        map.insert(
            "items".to_string(),
            Value::Sequence(vec![Value::Integer(1), Value::Integer(2)]),
        );
        let mut v = Value::Mapping(map);

        *v.get_path_mut("items[1]").unwrap() = Value::Integer(99);
        assert_eq!(v.get_path("items[1]").unwrap().as_i64(), Some(99));
    }

    #[test]
    fn test_get_path_mut_array_out_of_bounds() {
        let mut map = IndexMap::new();
        map.insert(
            "items".to_string(),
            Value::Sequence(vec![Value::Integer(1)]),
        );
        let mut v = Value::Mapping(map);
        assert!(v.get_path_mut("items[99]").is_err());
    }

    #[test]
    fn test_get_path_mut_array_on_non_sequence() {
        let mut map = IndexMap::new();
        map.insert("key".to_string(), Value::Integer(42));
        let mut v = Value::Mapping(map);
        assert!(v.get_path_mut("key[0]").is_err());
    }

    #[test]
    fn test_set_path_empty() {
        let mut v = Value::Integer(42);
        v.set_path("", Value::Integer(100)).unwrap();
        assert_eq!(v, Value::Integer(100));
    }

    #[test]
    fn test_set_path_simple() {
        let mut map = IndexMap::new();
        map.insert("key".to_string(), Value::Integer(42));
        let mut v = Value::Mapping(map);

        v.set_path("key", Value::Integer(100)).unwrap();
        assert_eq!(v.get_path("key").unwrap().as_i64(), Some(100));
    }

    #[test]
    fn test_set_path_new_key() {
        let mut map = IndexMap::new();
        map.insert("existing".to_string(), Value::Integer(1));
        let mut v = Value::Mapping(map);

        v.set_path("new_key", Value::Integer(42)).unwrap();
        assert_eq!(v.get_path("new_key").unwrap().as_i64(), Some(42));
    }

    #[test]
    fn test_set_path_creates_intermediate_mappings() {
        let mut v = Value::Mapping(IndexMap::new());
        v.set_path("a.b.c", Value::Integer(42)).unwrap();
        assert_eq!(v.get_path("a.b.c").unwrap().as_i64(), Some(42));
    }

    #[test]
    fn test_set_path_array_element() {
        let mut map = IndexMap::new();
        map.insert(
            "items".to_string(),
            Value::Sequence(vec![Value::Integer(1), Value::Integer(2)]),
        );
        let mut v = Value::Mapping(map);

        v.set_path("items[0]", Value::Integer(99)).unwrap();
        assert_eq!(v.get_path("items[0]").unwrap().as_i64(), Some(99));
    }

    #[test]
    fn test_set_path_array_out_of_bounds() {
        let mut map = IndexMap::new();
        map.insert(
            "items".to_string(),
            Value::Sequence(vec![Value::Integer(1)]),
        );
        let mut v = Value::Mapping(map);
        assert!(v.set_path("items[99]", Value::Integer(42)).is_err());
    }

    #[test]
    fn test_set_path_on_non_mapping() {
        let mut v = Value::Integer(42);
        assert!(v.set_path("key", Value::Integer(1)).is_err());
    }

    #[test]
    fn test_set_path_key_on_non_mapping_intermediate() {
        let mut map = IndexMap::new();
        map.insert("key".to_string(), Value::Integer(42));
        let mut v = Value::Mapping(map);
        // Trying to set "key.subkey" when "key" is an integer
        assert!(v.set_path("key.subkey", Value::Integer(1)).is_err());
    }

    #[test]
    fn test_set_path_index_on_non_sequence() {
        let mut map = IndexMap::new();
        map.insert("key".to_string(), Value::Integer(42));
        let mut v = Value::Mapping(map);
        assert!(v.set_path("key[0]", Value::Integer(1)).is_err());
    }

    #[test]
    fn test_set_path_intermediate_index_out_of_bounds() {
        let mut map = IndexMap::new();
        map.insert(
            "items".to_string(),
            Value::Sequence(vec![Value::Mapping(IndexMap::new())]),
        );
        let mut v = Value::Mapping(map);
        assert!(v.set_path("items[99].key", Value::Integer(1)).is_err());
    }

    #[test]
    fn test_merged_non_mutating() {
        let mut base_map = IndexMap::new();
        base_map.insert("a".to_string(), Value::Integer(1));
        let base = Value::Mapping(base_map);

        let mut overlay_map = IndexMap::new();
        overlay_map.insert("b".to_string(), Value::Integer(2));
        let overlay = Value::Mapping(overlay_map);

        let result = base.merged(overlay);

        assert_eq!(result.get_path("a").unwrap().as_i64(), Some(1));
        assert_eq!(result.get_path("b").unwrap().as_i64(), Some(2));
    }

    #[test]
    fn test_parse_path_invalid_index() {
        assert!(parse_path("items[abc]").is_err());
    }

    #[test]
    fn test_parse_path_unexpected_bracket() {
        assert!(parse_path("items]").is_err());
    }

    #[test]
    fn test_is_type_negative_cases() {
        let integer = Value::Integer(42);
        assert!(!integer.is_null());
        assert!(!integer.is_bool());
        assert!(!integer.is_float());
        assert!(!integer.is_string());
        assert!(!integer.is_sequence());
        assert!(!integer.is_mapping());
    }
}

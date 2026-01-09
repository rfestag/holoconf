//! Interpolation parsing per ADR-011
//!
//! Parses interpolation expressions like:
//! - `${env:VAR}` - resolver with argument
//! - `${env:VAR,default}` - resolver with default
//! - `${path.to.value}` - self-reference
//! - `${.sibling}` - relative self-reference
//! - `\${escaped}` - escaped (literal) interpolation
//! - `${env:VAR,${env:OTHER,fallback}}` - nested interpolations

use crate::error::{Error, Result};
use std::collections::HashMap;

/// A parsed interpolation expression
#[derive(Debug, Clone, PartialEq)]
pub enum Interpolation {
    /// A literal string (no interpolation or escaped interpolation)
    Literal(String),
    /// A resolver call: ${resolver:arg1,arg2,key=value}
    Resolver {
        /// Resolver name (e.g., "env", "file")
        name: String,
        /// Positional arguments
        args: Vec<InterpolationArg>,
        /// Keyword arguments
        kwargs: HashMap<String, InterpolationArg>,
    },
    /// A self-reference: ${path.to.value}
    SelfRef {
        /// The path to reference
        path: String,
        /// Whether this is a relative path (starts with .)
        relative: bool,
    },
    /// A concatenation of multiple parts
    Concat(Vec<Interpolation>),
}

/// An argument to an interpolation (may itself contain interpolations)
#[derive(Debug, Clone, PartialEq)]
pub enum InterpolationArg {
    /// A literal string value
    Literal(String),
    /// A nested interpolation
    Nested(Box<Interpolation>),
}

impl InterpolationArg {
    /// Check if this argument is a simple literal
    pub fn is_literal(&self) -> bool {
        matches!(self, InterpolationArg::Literal(_))
    }

    /// Get the literal value if this is a literal
    pub fn as_literal(&self) -> Option<&str> {
        match self {
            InterpolationArg::Literal(s) => Some(s),
            _ => None,
        }
    }
}

/// Parser for interpolation expressions
pub struct InterpolationParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> InterpolationParser<'a> {
    /// Create a new parser for the given input
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    /// Parse the entire input string
    pub fn parse(&mut self) -> Result<Interpolation> {
        let mut parts = Vec::new();

        while !self.is_eof() {
            if self.check_escape() {
                // \${ -> literal ${
                self.advance(); // skip backslash
                self.advance(); // skip $
                self.advance(); // skip {
                parts.push(Interpolation::Literal("${".to_string()));
            } else if self.check_interpolation_start() {
                parts.push(self.parse_interpolation()?);
            } else {
                // Collect literal text until next interpolation or end
                let literal = self.collect_literal();
                if !literal.is_empty() {
                    parts.push(Interpolation::Literal(literal));
                }
            }
        }

        // Simplify result
        match parts.len() {
            0 => Ok(Interpolation::Literal(String::new())),
            1 => Ok(parts.remove(0)),
            _ => {
                // Merge adjacent literals
                let merged = merge_adjacent_literals(parts);
                if merged.len() == 1 {
                    Ok(merged.into_iter().next().unwrap())
                } else {
                    Ok(Interpolation::Concat(merged))
                }
            }
        }
    }

    /// Check if we're at end of input
    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    /// Get current character
    fn current(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    /// Peek at the next character
    fn peek(&self) -> Option<char> {
        let mut chars = self.input[self.pos..].chars();
        chars.next();
        chars.next()
    }

    /// Peek at character n positions ahead
    fn peek_n(&self, n: usize) -> Option<char> {
        self.input[self.pos..].chars().nth(n)
    }

    /// Advance by one character
    fn advance(&mut self) {
        if let Some(c) = self.current() {
            self.pos += c.len_utf8();
        }
    }

    /// Check if we're at an escape sequence (\${)
    fn check_escape(&self) -> bool {
        self.current() == Some('\\') && self.peek() == Some('$') && self.peek_n(2) == Some('{')
    }

    /// Check if we're at an interpolation start (${)
    fn check_interpolation_start(&self) -> bool {
        self.current() == Some('$') && self.peek() == Some('{')
    }

    /// Collect literal text until interpolation or end
    fn collect_literal(&mut self) -> String {
        let mut result = String::new();

        while !self.is_eof() {
            if self.check_escape() {
                break;
            }
            if self.check_interpolation_start() {
                break;
            }
            if let Some(c) = self.current() {
                result.push(c);
                self.advance();
            }
        }

        result
    }

    /// Parse an interpolation expression (starting at ${)
    fn parse_interpolation(&mut self) -> Result<Interpolation> {
        // Skip ${
        self.advance(); // $
        self.advance(); // {

        // Skip whitespace
        self.skip_whitespace();

        if self.is_eof() {
            return Err(Error::parse("Unexpected end of input in interpolation"));
        }

        // Check for relative path (.sibling or ..parent)
        if self.current() == Some('.') {
            return self.parse_self_ref(true);
        }

        // Collect the identifier (resolver name or path)
        let identifier = self.collect_identifier();

        if identifier.is_empty() {
            return Err(Error::parse("Empty interpolation expression"));
        }

        self.skip_whitespace();

        // Check what follows the identifier
        match self.current() {
            Some(':') => {
                // This is a resolver call: ${resolver:args}
                self.advance(); // skip :
                self.parse_resolver_call(identifier)
            }
            Some('}') => {
                // This is a simple self-reference: ${path.to.value}
                self.advance(); // skip }
                Ok(Interpolation::SelfRef {
                    path: identifier,
                    relative: false,
                })
            }
            Some(',') => {
                // Could be a self-reference with default? Not supported in standard syntax.
                // Treat as resolver with empty name - will fail at resolution time
                Err(Error::parse(format!(
                    "Unexpected ',' after identifier '{}'. Did you mean to use a resolver?",
                    identifier
                )))
            }
            Some(c) => Err(Error::parse(format!(
                "Unexpected character '{}' in interpolation",
                c
            ))),
            None => Err(Error::parse("Unexpected end of input in interpolation")),
        }
    }

    /// Parse a self-reference (possibly relative)
    fn parse_self_ref(&mut self, relative: bool) -> Result<Interpolation> {
        let mut path = String::new();

        // Collect the full path including dots
        while !self.is_eof() {
            match self.current() {
                Some('}') => {
                    self.advance();
                    break;
                }
                Some(c) if c.is_alphanumeric() || c == '_' || c == '.' || c == '[' || c == ']' => {
                    path.push(c);
                    self.advance();
                }
                Some(c) => {
                    return Err(Error::parse(format!("Invalid character '{}' in path", c)));
                }
                None => {
                    return Err(Error::parse("Unexpected end of input in path"));
                }
            }
        }

        Ok(Interpolation::SelfRef { path, relative })
    }

    /// Parse a resolver call after the colon
    fn parse_resolver_call(&mut self, name: String) -> Result<Interpolation> {
        let mut args = Vec::new();
        let mut kwargs = HashMap::new();

        // Parse arguments separated by commas
        loop {
            self.skip_whitespace();

            if self.current() == Some('}') {
                self.advance();
                break;
            }

            if !args.is_empty() || !kwargs.is_empty() {
                // Expect comma separator
                if self.current() != Some(',') {
                    return Err(Error::parse("Expected ',' between arguments"));
                }
                self.advance(); // skip comma
                self.skip_whitespace();
            }

            // Parse argument
            let arg = self.parse_argument()?;

            // Check if this is a kwarg (look for = before the value)
            // For simplicity, we check if the arg is a literal and contains =
            if let InterpolationArg::Literal(s) = &arg {
                if let Some(eq_pos) = s.find('=') {
                    let key = s[..eq_pos].to_string();
                    let value = s[eq_pos + 1..].to_string();
                    kwargs.insert(key, InterpolationArg::Literal(value));
                    continue;
                }
            }

            args.push(arg);
        }

        Ok(Interpolation::Resolver { name, args, kwargs })
    }

    /// Parse a single argument (may be literal or nested interpolation)
    fn parse_argument(&mut self) -> Result<InterpolationArg> {
        self.skip_whitespace();

        if self.check_interpolation_start() {
            // Nested interpolation
            let nested = self.parse_interpolation()?;
            Ok(InterpolationArg::Nested(Box::new(nested)))
        } else {
            // Literal argument - collect until , or }
            let mut value = String::new();
            let mut depth = 0; // Track nested braces

            while !self.is_eof() {
                match self.current() {
                    Some('$') if self.peek() == Some('{') => {
                        // Nested interpolation - parse it
                        let nested = self.parse_interpolation()?;
                        return Ok(InterpolationArg::Nested(Box::new(if value.is_empty() {
                            nested
                        } else {
                            // Concatenation: literal prefix + nested
                            Interpolation::Concat(vec![Interpolation::Literal(value), nested])
                        })));
                    }
                    Some('{') => {
                        depth += 1;
                        value.push('{');
                        self.advance();
                    }
                    Some('}') => {
                        if depth == 0 {
                            break;
                        }
                        depth -= 1;
                        value.push('}');
                        self.advance();
                    }
                    Some(',') if depth == 0 => {
                        break;
                    }
                    Some(c) => {
                        value.push(c);
                        self.advance();
                    }
                    None => break,
                }
            }

            Ok(InterpolationArg::Literal(value.trim().to_string()))
        }
    }

    /// Collect an identifier (alphanumeric, _, ., [, ])
    fn collect_identifier(&mut self) -> String {
        let mut result = String::new();

        while !self.is_eof() {
            match self.current() {
                Some(c) if c.is_alphanumeric() || c == '_' || c == '.' || c == '[' || c == ']' => {
                    result.push(c);
                    self.advance();
                }
                _ => break,
            }
        }

        result
    }

    /// Skip whitespace characters
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.current() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }
}

/// Merge adjacent literal parts
fn merge_adjacent_literals(parts: Vec<Interpolation>) -> Vec<Interpolation> {
    let mut result = Vec::new();
    let mut current_literal = String::new();

    for part in parts {
        match part {
            Interpolation::Literal(s) => {
                current_literal.push_str(&s);
            }
            other => {
                if !current_literal.is_empty() {
                    result.push(Interpolation::Literal(current_literal));
                    current_literal = String::new();
                }
                result.push(other);
            }
        }
    }

    if !current_literal.is_empty() {
        result.push(Interpolation::Literal(current_literal));
    }

    result
}

/// Parse an interpolation string
pub fn parse(input: &str) -> Result<Interpolation> {
    InterpolationParser::new(input).parse()
}

/// Check if a string contains any interpolation expressions (unescaped ${...})
pub fn contains_interpolation(input: &str) -> bool {
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            // Skip escaped characters
            chars.next();
        } else if c == '$' && chars.peek() == Some(&'{') {
            return true;
        }
    }

    false
}

/// Check if a string needs processing (has interpolations OR escape sequences)
pub fn needs_processing(input: &str) -> bool {
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'$') {
            // Has an escape sequence
            return true;
        } else if c == '$' && chars.peek() == Some(&'{') {
            // Has an interpolation
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_literal() {
        let result = parse("hello world").unwrap();
        assert_eq!(result, Interpolation::Literal("hello world".into()));
    }

    #[test]
    fn test_parse_empty() {
        let result = parse("").unwrap();
        assert_eq!(result, Interpolation::Literal("".into()));
    }

    #[test]
    fn test_parse_env_resolver() {
        let result = parse("${env:MY_VAR}").unwrap();
        assert_eq!(
            result,
            Interpolation::Resolver {
                name: "env".into(),
                args: vec![InterpolationArg::Literal("MY_VAR".into())],
                kwargs: HashMap::new(),
            }
        );
    }

    #[test]
    fn test_parse_env_with_default() {
        let result = parse("${env:MY_VAR,default_value}").unwrap();
        assert_eq!(
            result,
            Interpolation::Resolver {
                name: "env".into(),
                args: vec![
                    InterpolationArg::Literal("MY_VAR".into()),
                    InterpolationArg::Literal("default_value".into()),
                ],
                kwargs: HashMap::new(),
            }
        );
    }

    #[test]
    fn test_parse_self_reference() {
        let result = parse("${database.host}").unwrap();
        assert_eq!(
            result,
            Interpolation::SelfRef {
                path: "database.host".into(),
                relative: false,
            }
        );
    }

    #[test]
    fn test_parse_relative_self_reference() {
        let result = parse("${.sibling}").unwrap();
        // The path includes the leading dot(s) for relative references
        assert_eq!(
            result,
            Interpolation::SelfRef {
                path: ".sibling".into(),
                relative: true,
            }
        );
    }

    #[test]
    fn test_parse_array_access() {
        let result = parse("${servers[0].host}").unwrap();
        assert_eq!(
            result,
            Interpolation::SelfRef {
                path: "servers[0].host".into(),
                relative: false,
            }
        );
    }

    #[test]
    fn test_parse_escaped() {
        let result = parse(r"\${not_interpolated}").unwrap();
        assert_eq!(result, Interpolation::Literal("${not_interpolated}".into()));
    }

    #[test]
    fn test_parse_concatenation() {
        let result = parse("prefix_${env:VAR}_suffix").unwrap();
        assert!(matches!(result, Interpolation::Concat(_)));

        if let Interpolation::Concat(parts) = result {
            assert_eq!(parts.len(), 3);
            assert_eq!(parts[0], Interpolation::Literal("prefix_".into()));
            assert!(matches!(parts[1], Interpolation::Resolver { .. }));
            assert_eq!(parts[2], Interpolation::Literal("_suffix".into()));
        }
    }

    #[test]
    fn test_parse_nested_interpolation() {
        let result = parse("${env:VAR,${env:DEFAULT,fallback}}").unwrap();

        if let Interpolation::Resolver { name, args, .. } = result {
            assert_eq!(name, "env");
            assert_eq!(args.len(), 2);
            assert!(matches!(args[0], InterpolationArg::Literal(_)));
            assert!(matches!(args[1], InterpolationArg::Nested(_)));
        } else {
            panic!("Expected Resolver, got {:?}", result);
        }
    }

    #[test]
    fn test_parse_kwargs() {
        let result = parse("${file:./config.yaml,parse=yaml}").unwrap();

        if let Interpolation::Resolver { kwargs, .. } = result {
            assert!(kwargs.contains_key("parse"));
        } else {
            panic!("Expected Resolver");
        }
    }

    #[test]
    fn test_contains_interpolation() {
        assert!(contains_interpolation("${env:VAR}"));
        assert!(contains_interpolation("prefix ${env:VAR} suffix"));
        assert!(!contains_interpolation("no interpolation"));
        assert!(!contains_interpolation(r"\${escaped}"));
        assert!(!contains_interpolation("just $dollar"));
    }

    #[test]
    fn test_parse_unclosed_interpolation() {
        let result = parse("${env:VAR");
        assert!(result.is_err());
    }

    // Edge case tests for improved coverage

    #[test]
    fn test_parse_empty_interpolation() {
        let result = parse("${}");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Empty"));
    }

    #[test]
    fn test_parse_resolver_no_args() {
        // Resolver with colon but no args - returns empty arg list
        let result = parse("${env:}").unwrap();
        if let Interpolation::Resolver { name, args, .. } = result {
            assert_eq!(name, "env");
            // Empty resolver has no args (or one empty arg depending on implementation)
            // The current behavior returns an empty arg list
            assert!(
                args.is_empty()
                    || (args.len() == 1 && args[0] == InterpolationArg::Literal("".into()))
            );
        } else {
            panic!("Expected Resolver");
        }
    }

    #[test]
    fn test_parse_whitespace_in_interpolation() {
        let result = parse("${ env:VAR }").unwrap();
        if let Interpolation::Resolver { name, .. } = result {
            assert_eq!(name, "env");
        } else {
            panic!("Expected Resolver");
        }
    }

    #[test]
    fn test_parse_multiple_escapes() {
        let result = parse(r"\${first}\${second}").unwrap();
        assert_eq!(result, Interpolation::Literal("${first}${second}".into()));
    }

    #[test]
    fn test_parse_interpolation_at_start() {
        let result = parse("${env:VAR}suffix").unwrap();
        if let Interpolation::Concat(parts) = result {
            assert_eq!(parts.len(), 2);
            assert!(matches!(parts[0], Interpolation::Resolver { .. }));
            assert_eq!(parts[1], Interpolation::Literal("suffix".into()));
        } else {
            panic!("Expected Concat");
        }
    }

    #[test]
    fn test_parse_interpolation_at_end() {
        let result = parse("prefix${env:VAR}").unwrap();
        if let Interpolation::Concat(parts) = result {
            assert_eq!(parts.len(), 2);
            assert_eq!(parts[0], Interpolation::Literal("prefix".into()));
            assert!(matches!(parts[1], Interpolation::Resolver { .. }));
        } else {
            panic!("Expected Concat");
        }
    }

    #[test]
    fn test_parse_adjacent_interpolations() {
        let result = parse("${env:A}${env:B}").unwrap();
        if let Interpolation::Concat(parts) = result {
            assert_eq!(parts.len(), 2);
            assert!(matches!(parts[0], Interpolation::Resolver { .. }));
            assert!(matches!(parts[1], Interpolation::Resolver { .. }));
        } else {
            panic!("Expected Concat");
        }
    }

    #[test]
    fn test_parse_deeply_nested_path() {
        let result = parse("${a.b.c.d.e.f.g.h}").unwrap();
        if let Interpolation::SelfRef { path, relative } = result {
            assert_eq!(path, "a.b.c.d.e.f.g.h");
            assert!(!relative);
        } else {
            panic!("Expected SelfRef");
        }
    }

    #[test]
    fn test_parse_multiple_array_indices() {
        let result = parse("${matrix[0][1][2]}").unwrap();
        if let Interpolation::SelfRef { path, .. } = result {
            assert_eq!(path, "matrix[0][1][2]");
        } else {
            panic!("Expected SelfRef");
        }
    }

    #[test]
    fn test_parse_mixed_path_and_array() {
        let result = parse("${data.items[0].nested[1].value}").unwrap();
        if let Interpolation::SelfRef { path, .. } = result {
            assert_eq!(path, "data.items[0].nested[1].value");
        } else {
            panic!("Expected SelfRef");
        }
    }

    #[test]
    fn test_parse_underscore_in_identifiers() {
        let result = parse("${my_var.some_path}").unwrap();
        if let Interpolation::SelfRef { path, .. } = result {
            assert_eq!(path, "my_var.some_path");
        } else {
            panic!("Expected SelfRef");
        }
    }

    #[test]
    fn test_parse_resolver_with_multiple_args() {
        let result = parse("${resolver:arg1,arg2,arg3}").unwrap();
        if let Interpolation::Resolver { name, args, .. } = result {
            assert_eq!(name, "resolver");
            assert_eq!(args.len(), 3);
        } else {
            panic!("Expected Resolver");
        }
    }

    #[test]
    fn test_parse_mixed_escaped_and_interpolation() {
        let result = parse(r"literal \${escaped} ${env:VAR} more").unwrap();
        if let Interpolation::Concat(parts) = result {
            assert!(parts.len() >= 3);
        } else {
            panic!("Expected Concat");
        }
    }

    #[test]
    fn test_needs_processing() {
        assert!(needs_processing("${env:VAR}"));
        assert!(needs_processing(r"\${escaped}"));
        assert!(!needs_processing("no special chars"));
        assert!(!needs_processing("just $dollar"));
    }

    #[test]
    fn test_parse_invalid_char_in_path() {
        let result = parse("${path!invalid}");
        assert!(result.is_err());
    }

    #[test]
    fn test_interpolation_arg_methods() {
        let lit = InterpolationArg::Literal("test".into());
        assert!(lit.is_literal());
        assert_eq!(lit.as_literal(), Some("test"));

        let nested = InterpolationArg::Nested(Box::new(Interpolation::Literal("x".into())));
        assert!(!nested.is_literal());
        assert_eq!(nested.as_literal(), None);
    }

    #[test]
    fn test_parse_relative_parent_reference() {
        let result = parse("${..parent.value}").unwrap();
        if let Interpolation::SelfRef { path, relative } = result {
            assert!(relative);
            assert_eq!(path, "..parent.value");
        } else {
            panic!("Expected relative SelfRef");
        }
    }
}

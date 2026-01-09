# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### Configuration Loading & Parsing
- Load configuration from YAML and JSON files with automatic format detection
- Parse configuration into a tree structure supporting scalars (null, bool, int, float, string) and collections (sequences, mappings)
- Path-based value access using dot notation (`database.host`) and array indexing (`servers[0].name`)
- Type-safe accessors: `get_string()`, `get_i64()`, `get_f64()`, `get_bool()`
- Load and deep-merge multiple configuration files with `load_merged()`

#### Resolver System
- **Environment Variable Resolver (`env`)**: Access environment variables with optional defaults
  - Syntax: `${env:VAR_NAME}` or `${env:VAR_NAME,default_value}`
  - Sensitivity marking: `${env:SECRET,sensitive=true}` for automatic redaction
- **Self-Reference Resolver**: Reference other values within the same configuration
  - Absolute paths: `${path.to.value}`
  - Relative paths: `${.sibling}` or `${..parent.value}`
  - Array access: `${servers[0].host}`
  - Circular reference detection with helpful error messages
- **File Resolver (`file`)**: Include content from external files
  - Syntax: `${file:./path/to/config.yaml}`
  - Automatic format detection (YAML, JSON, or plain text)
  - Explicit parse mode: `${file:./data.json,parse=json}`
- **HTTP Resolver (`http`)**: Fetch configuration from remote URLs (disabled by default for security)
  - Requires explicit `allow_http=true` option
  - URL allowlist support with glob patterns
- **Custom Resolvers**: Register custom resolver functions via the resolver registry

#### Interpolation & Templating
- String interpolation with resolver calls: `${resolver:arg1,arg2,key=value}`
- Nested interpolations: `${env:VAR,${env:FALLBACK,default}}`
- String concatenation: `prefix_${env:VAR}_suffix`
- Escape sequences: `\${literal}` prevents interpolation
- Keyword argument support for resolver options

#### Schema Validation
- JSON Schema validation (Draft 2020-12 compatible)
- Two-phase validation:
  - Structural validation: validates raw config, allows interpolation placeholders
  - Type/value validation: validates fully resolved values
- Load schemas from YAML or JSON files
- Collect all validation errors (not just first failure)
- Support for: `type`, `required`, `properties`, `enum`, `pattern`, `minimum`, `maximum`, `minLength`, `maxLength`

#### Configuration Merging
- Deep merge multiple configuration files
- Merge semantics:
  - Mappings: recursively merged (deep merge)
  - Scalars: last-writer-wins
  - Arrays: replaced entirely (not concatenated)
  - Null values: remove keys from result

#### Serialization & Export
- Export to YAML: `to_yaml()`, `to_yaml_raw()`, `to_yaml_redacted()`
- Export to JSON: `to_json()`, `to_json_raw()`, `to_json_redacted()`
- Export to native values: `to_value()`, `to_dict()` (Python)
- Automatic redaction of sensitive values to `[REDACTED]`

#### Value Resolution & Caching
- Lazy resolution: values only resolved when accessed
- Automatic caching of resolved values for performance
- Resolution stack tracking for circular reference detection
- Cache clearing on merge operations

#### CLI (`holoconf`)
- `holoconf validate` - Validate configuration against a schema
- `holoconf dump` - Export configuration in YAML or JSON format
- `holoconf get` - Retrieve specific values by path
- `holoconf check` - Quick syntax validation
- Output formats: text, JSON, YAML
- Safe redaction of sensitive values by default

#### Python Bindings
- Full Python API via PyO3 bindings
- `Config.loads()`, `Config.load()`, `Config.load_merged()`
- Type-safe accessors: `get_string()`, `get_int()`, `get_float()`, `get_bool()`
- Schema validation: `config.validate(schema)`
- Export methods: `to_yaml()`, `to_json()`, `to_dict()`
- Python exception hierarchy: `HoloconfError`, `ParseError`, `ValidationError`, `ResolverError`, `PathNotFoundError`, `CircularReferenceError`, `TypeCoercionError`

#### Type Coercion
- Strict boolean coercion: only `"true"` and `"false"` strings convert to boolean
- Flexible numeric coercion: string numbers convert to int/float
- String fallback: any value can be converted to string representation

#### Security Features
- HTTP resolver disabled by default with explicit opt-in
- URL allowlist with glob pattern matching
- Sensitive value marking and automatic redaction
- Configurable file roots for sandboxed file access

#### Thread Safety
- Thread-safe resolution cache with `Arc<RwLock<>>`
- All resolvers are `Send + Sync` compatible
- Safe for concurrent access in multi-threaded applications

#### Error Handling
- Structured error types with context and source location
- Error categories: Parse, Resolver, Validation, PathNotFound, CircularReference, TypeCoercion, IO
- Actionable error messages with suggestions
- Full error context preservation

### Documentation
- Comprehensive documentation site with MkDocs Material
- Architecture Decision Records (ADRs) for design decisions
- Feature specifications for planned features
- API reference documentation
- Quick start guide and examples

## [0.1.0] - Unreleased

Initial release with core functionality.

[Unreleased]: https://github.com/holoconf/holoconf/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/holoconf/holoconf/releases/tag/v0.1.0

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-02-11

### Added
- **Config Reference Defaults**: Config references now support optional `default=` parameter for handling missing values (#25)
  - Use `${path.to.value,default=fallback}` to provide a fallback when path doesn't exist
  - Works with explicit `${ref:path,default=fallback}` syntax as well
  - Defaults only apply when path is missing or null
  - Supports nested defaults: `${config.timeout,default=${defaults.timeout}}`
  - Honors schema defaults as fallback when no explicit default provided
  - Framework-level `sensitive=` flag also supported for consistency
  - See [Interpolation Guide](guide/interpolation.md) for full details
- **Transformation Resolvers**: New resolvers for parsing structured data (#26)
  - `${json:text}` - Parse JSON strings into structured data
  - `${yaml:text}` - Parse YAML strings into structured data
  - `${split:text}` - Split strings into arrays with customizable delimiters
  - `${csv:text}` - Parse CSV data with header support
  - `${base64:text}` - Decode base64-encoded strings (auto-detects UTF-8 vs binary)
  - All transformation resolvers support chaining: `${json:${file:config.json}}`
  - CSV values returned as strings (use schema validation for type coercion)
  - Base64 automatically returns UTF-8 strings when possible, falls back to bytes for binary data
- **Archive Extraction Resolver**: New `extract` resolver for extracting files from archives
  - Supports ZIP, TAR, and TAR.GZ formats with automatic format detection
  - Syntax: `${extract:${file:archive.zip,encoding=binary},path=config.json}`
  - Extract specific files from archives by name (no filesystem extraction)
  - Password support for encrypted ZIP files: `password=secret` (supports both ZipCrypto and AES)
  - Returns extracted file contents as bytes
  - Chain with transformation resolvers: `${json:${extract:${file:data.zip,encoding=binary},path=config.json}}`
  - Works with remote archives: `${extract:${https:releases.example.com/v1.0.0.tar.gz,parse=binary},path=config.json}`
  - **Security:** Zip bomb protection with 10MB per-file limit and 100:1 compression ratio check
  - **Security:** Warn users about weak ZipCrypto encryption (prefer AES or GPG)
  - Requires `archive` feature flag (enabled by default in Python package)
  - Feature adds dependencies: `tar`, `zip`, `flate2`, `infer`
- **AWS Resolver Configuration API**: Two-tier configuration system for AWS resolvers (#27)
  - Global configuration: `holoconf_aws.configure(region="us-east-1", profile="prod")` sets defaults for all AWS services
  - Service-specific configuration: `holoconf_aws.s3(endpoint="http://localhost:5000")` overrides for individual services
  - `holoconf_aws.reset()` clears all configuration and client cache (useful for test isolation)
  - Four-level precedence: resolver kwargs > service config > global config > AWS SDK defaults
  - All AWS resolvers now support `endpoint=` kwarg for per-call endpoint overrides
  - Enables testing with moto/LocalStack via custom endpoint URLs
  - Configuration is additive: calling `configure(region=None)` leaves existing values unchanged
  - See [Configuration API guide](guide/resolvers-aws.md#configuration-api) for full details

### Changed
- **BREAKING: File Resolver Auto-Parsing Removed**: File resolver no longer automatically parses JSON/YAML based on file extension (#26)
  - Old behavior: `${file:config.json}` → automatically parsed as JSON
  - New behavior: `${file:config.json}` → returns JSON as a string
  - Migration: Use transformation resolvers: `${json:${file:config.json}}`
  - `parse=text` is now the default (explicit text, no parsing)
  - `parse=none` returns raw bytes (alias for `encoding=binary`)
  - `parse=json` and `parse=yaml` parameters removed
  - `parse=auto` parameter removed (no longer auto-detects from extension)
- **BREAKING: HTTP/HTTPS Resolver Auto-Parsing Removed**: HTTP/HTTPS resolvers no longer automatically parse JSON/YAML (#26)
  - Old behavior: Content-Type header or URL extension triggered auto-parsing
  - New behavior: Always returns response body as text
  - Migration: Use transformation resolvers: `${json:${http:api.com/data}}`
  - `parse=text` is now the default
  - `parse=binary` still supported for binary content
  - `parse=json` and `parse=yaml` parameters removed
  - `parse=auto` parameter removed (no longer auto-detects)
- **Internal Optimization: Streaming Binary Data** _(not user-facing)_: File and HTTP/HTTPS resolvers now use streaming for binary data transfer
  - Implementation detail: Binary resolvers return streams internally, materialized before caching
  - No API changes: Users always receive materialized `Value::Bytes`, never streams
  - Significant memory efficiency: Defers I/O until needed, then reads in chunks
  - Internal: `Value::Stream(Box<dyn Read + Send + Sync>)` variant added for resolver → cache pipeline
  - Note: Streams are fully internal - they never escape to public API or Python bindings
- **Nested Path Access**: Config paths now work with transformation resolver output (#26)
  - `data: ${json:${env:CONFIG}}` followed by `config.get("data.name")` now works correctly
  - Paths like `users[0].email` navigate into resolved structures seamlessly
  - Enables natural access patterns for transformed data

## [0.4.0] - 2026-01-19

### Added
- **HTTPS Resolver**: New `https` resolver that auto-prepends `https://` to URLs (#43)
  - Use `${https:example.com/config}` instead of `${http:https://example.com/config}`
  - Separate resolver for clarity and convenience
  - Same security model as `http` resolver (disabled by default, requires `allow_http=True`)
- **File Resolver RFC 8089 Support**: File resolver now supports RFC 8089 file: URI syntax (#43)
  - `${file:///absolute/path}` - Absolute path with empty authority
  - `${file://localhost/absolute/path}` - Absolute path with explicit localhost
  - `${file://127.0.0.1/path}` - Localhost via IPv4 loopback
  - `${file://::1/path}` - Localhost via IPv6 loopback
  - `${file:/absolute/path}` - Absolute path (minimal form)
  - Remote file URIs (`file://hostname/path`) are rejected with clear error
  - Plain paths continue to work as before
- **Certificate Variables for HTTPS**: HTTPS resolver now accepts certificate/key content from variables, not just file paths (#39)
  - Pass PEM certificates directly from environment variables or other resolvers
  - Support for P12/PFX binary certificates via `parse=binary` from file resolver
  - Auto-detection: `-----BEGIN` marker = PEM content, otherwise = file path
  - P12 binary content auto-detected when passed as bytes
  - Fully backwards compatible - existing file paths continue to work
  - Example: `${https:api.com/config,client_cert=${env:CERT_PEM},client_key=${env:KEY_PEM}}`
  - Example: `${https:api.com/config,client_cert=${file:./id.p12,parse=binary}}`

### Changed
- **HTTP/HTTPS URL Normalization**: HTTP and HTTPS resolvers now auto-prepend protocol schemes (#43)
  - Old syntax still works: `${http:https://example.com}` → `http://example.com`
  - New clean syntax: `${https:example.com}` → `https://example.com`
  - Fully backwards compatible with existing configurations
  - Invalid URL syntax (like `///example.com`) now returns a clear error

### Fixed
- **CLI HTTP Feature**: CLI now enables HTTP/HTTPS resolvers by default via the `http` feature (#43)

### Security
- **File Resolver Null Byte Validation**: File paths with null bytes are now rejected to prevent potential path traversal attacks (#43)
- **Enhanced Localhost Detection**: File resolver now recognizes IPv4 (127.x.x.x) and IPv6 (::1) localhost addresses in addition to hostname "localhost" (#43)

## [0.3.0] - 2026-01-17

### Security

#### File Resolver Path Traversal Protection (Breaking Change)
- **File access now sandboxed by default** - The `file` resolver restricts access to the config file's parent directory to prevent path traversal attacks
  - Automatically allows: Files in the same directory as the config file and its subdirectories
  - Blocks by default: Absolute paths like `/etc/passwd` or paths outside the config directory
  - Relative paths are resolved relative to the config file's directory
  - Symlinks are resolved and validated against allowed roots
- **New `file_roots` parameter** - Explicitly allow access to additional directories
  - Python: `Config.load("config.yaml", file_roots=["/etc/myapp", "/var/lib/myapp"])`
  - Rust: `ConfigOptions { file_roots: vec![PathBuf::from("/etc/myapp")], .. }`
- **Automatic root accumulation** - Parent directories are automatically added to allowed roots when loading files
- **Merge behavior** - When merging configs, `file_roots` from both configs are combined (union)
- **Breaking change**: Configs that reference files outside their directory will now fail unless `file_roots` is specified

## [0.2.0] - 2026-01-17

### Changed

#### Simplified Config Loading API (Breaking Change)
- **Removed `FileSpec` from public API** - `FileSpec` was a Rust-ism that leaked to Python; the concept is now internal
  - Old: `FileSpec.required("path")`, `FileSpec.optional("path")`
  - New: `Config.load("path")` for required, `Config.optional("path")` for optional
- **Removed `load_merged()` and `load_merged_with_specs()`** - Use explicit load + merge pattern instead
  - Old: `Config.load_merged(["base.yaml", "override.yaml"])`
  - New: `config = Config.load("base.yaml"); config.merge(Config.load("override.yaml"))`
- **Added `Config.optional(path)`** - Returns empty Config if file doesn't exist (no error)
- **Added `Config.required(path)`** - Alias for `Config.load()` for symmetry with `optional()`
- **Updated `Config.load(path)`** - Now returns a proper "file not found" error for missing files

#### Resolver Syntax (Breaking Change)
- **Keyword-only syntax for default and sensitive options** - Resolver options now use keyword argument syntax exclusively
  - Old syntax: `${env:VAR,fallback_value}` (positional default)
  - New syntax: `${env:VAR,default=fallback_value}` (keyword default)
  - Sensitive flag: `${env:SECRET,sensitive=true}`
  - Both combined: `${env:VAR,default=fallback,sensitive=true}`
- **Framework-level default/sensitive handling** - All resolvers now consistently support `default=` and `sensitive=` options
  - Lazy evaluation: default values only resolved when primary resolver fails
  - Works with any resolver: `${file:./config.yaml,default={}}`, `${http:...,default=fallback}`
  - Sensitive marking propagates correctly through resolution chain

### Added

#### HTTP Resolver TLS/Proxy Enhancements
- **Proxy support** - Configure HTTP/SOCKS proxies for HTTP resolver requests
  - `http_proxy="http://proxy:8080"` or `http_proxy="socks5://proxy:1080"`
  - `http_proxy_from_env=True` to auto-detect from HTTP_PROXY/HTTPS_PROXY environment variables
  - Per-request override: `${http:url,proxy=http://proxy:8080}`
- **Custom CA certificates** - Use internal/corporate CAs or self-signed certificates
  - `http_ca_bundle="/path/to/ca.pem"` - Replace default root certificates
  - `http_extra_ca_bundle="/path/to/extra.pem"` - Add to default root certificates
  - Per-request override: `${http:url,ca_bundle=/path}` or `${http:url,extra_ca_bundle=/path}`
- **Mutual TLS (mTLS) / Client certificates** - Authenticate with client certificates
  - `http_client_cert="/path/to/cert.pem"` - Client certificate (PEM or P12/PFX)
  - `http_client_key="/path/to/key.pem"` - Private key (not needed for P12/PFX)
  - `http_client_key_password="secret"` - Password for encrypted keys or P12/PFX
  - Supports: unencrypted PEM, encrypted PKCS#8 PEM, P12/PFX bundles
  - Per-request override: `${http:url,client_cert=/path,client_key=/path,key_password=secret}`
- **TLS verification bypass** - For development with self-signed certs (DANGEROUS)
  - `http_insecure=True` - Skip all TLS certificate verification
  - Per-request: `${http:url,insecure=true}`
  - **WARNING**: Never use in production - exposes to MITM attacks
- **FIPS-compliant TLS** - Uses rustls with aws-lc-rs crypto backend

#### Glob Pattern Support
- **Glob patterns in `Config.load()` and `Config.optional()`** - Load and merge multiple files matching a pattern
  - `Config.load("config/*.yaml")` - Load all matching files, error if none match
  - `Config.optional("config/*.yaml")` - Load all matching files, empty config if none match
- **Supported patterns**: `*` (any chars), `**` (recursive), `?` (single char), `[abc]` (char class)
- **Alphabetical merge order** - Files are sorted before merging, so `00-base.yaml` loads before `99-local.yaml`

#### Schema Default Values
- **Schema defaults for missing paths** - When a schema is attached to a config, accessing a missing path returns the schema default instead of `PathNotFoundError`
  - `Config.load("config.yaml", schema="schema.yaml")` - Load with schema attached
  - `config.set_schema(schema)` - Attach schema to existing config
  - `config.get_schema()` - Retrieve attached schema
- **Null-aware default lookup** - If a config value is `null` and the schema doesn't allow `null`, the schema default is used
- **validate() uses attached schema** - Call `config.validate()` with no args to use the attached schema
- **CLI --schema flag** - `holoconf get` and `holoconf dump` now accept `--schema` for default values
- **Enhanced validate output** - `holoconf validate` now shows all validation errors with paths

#### HTTP Resolver
- **Full HTTP resolver implementation** - Fetch configuration from remote URLs
  - Disabled by default for security - enable with `allow_http=True`
  - URL allowlist support for restricting which URLs can be fetched
  - Parse modes: `auto`, `yaml`, `json`, `text`, `binary`
  - Configurable timeout via `timeout=<seconds>` parameter
  - Custom header support via `header=Name:Value` parameter
  - Auto-detection from Content-Type and URL extension
- **Security controls**:
  - `allow_http` option to explicitly enable HTTP resolver
  - `http_allowlist` option to restrict accessible URLs with glob patterns
- **Example usage**:
  ```yaml
  config: ${http:https://config.example.com/settings.yaml}
  api_key: ${http:https://api.example.com/key,header=Authorization:Bearer token}
  ```

#### Custom Resolver Registration
- **Global resolver registry** - Register resolvers once, use everywhere
  - `holoconf.register_resolver(name, func, force=False)` - Python API
  - `register_global(resolver, force)` - Rust API
- **Async resolver support** - Async functions automatically awaited via `asyncio.run()`
  - `async def my_resolver(key, **kwargs): return await fetch(key)`
- **Return types** - Resolvers can return scalars, lists, dicts, or `ResolvedValue` for sensitive data
- **KeyError for default handling** - Custom resolvers raise `KeyError` to trigger framework default
- **Plugin discovery** - Automatic discovery of installed resolver plugins
  - `holoconf.discover_plugins()` - Load all plugins via entry points
  - Entry point group: `holoconf.resolvers`
- **holoconf-aws package** - AWS resolvers for SSM, CloudFormation, and S3
  - Rust crate: `holoconf-aws` with SSM, CFN, and S3 resolvers
  - Python package: `holoconf-aws` with auto-discovery via entry points
  - **SSM Parameter Store resolver (`ssm`)**: Fetch parameters from AWS Systems Manager
    - Automatic SecureString sensitivity detection
    - StringList to array conversion
    - Region/profile per-parameter overrides
  - **CloudFormation resolver (`cfn`)**: Fetch stack outputs
    - Syntax: `${cfn:stack-name/OutputKey}`
    - Region/profile overrides
  - **S3 resolver (`s3`)**: Fetch and parse S3 objects
    - Syntax: `${s3:bucket/key}`
    - Parse modes: `auto`, `yaml`, `json`, `text`, `binary`
    - Auto-detection from file extension and Content-Type
    - Region/profile overrides

#### Optional File Support
- **Optional files in config loading** - Load files that can be missing without causing errors
  - `Config.load(path)` / `Config.required(path)` - File must exist (error if missing)
  - `Config.optional(path)` - Returns empty Config if file doesn't exist
  - Use `config.merge(other)` to combine multiple configs
- Common use case: local developer overrides that aren't committed to version control
- Example pattern:
  ```python
  config = Config.load("base.yaml")
  local = Config.optional("local.yaml")  # Empty if missing
  config.merge(local)
  ```
- **CLI `--ignore-missing` flag** - Skip missing files when loading multiple config files
  - `holoconf dump --ignore-missing base.yaml local.yaml` - Works even if `local.yaml` doesn't exist
  - At least one file must load successfully; fails if all files are missing
  - Available on: `dump`, `get`, `validate`, `check` commands

#### Source Tracking
- **File-level source tracking** for merged configurations - track which file each value came from
  - `config.get_source("path.to.value")` - Returns the filename that provided this value
  - `config.dump_sources()` - Returns a map of all config paths to their source filenames
  - Always enabled with low overhead (no opt-in required)
- **CLI `--sources` flag** for `holoconf dump` - Output source files instead of values
  - `holoconf dump --sources base.yaml override.yaml` - Shows which file each value came from
  - Supports both text and JSON output formats

#### CLI
- `holoconf schema template` - Generate a YAML config template from a JSON Schema with defaults and required field markers

#### File Resolver
- Encoding options for the file resolver:
  - `encoding=utf-8` (default) - Read file as UTF-8 text
  - `encoding=ascii` - Read file as ASCII, stripping non-ASCII characters
  - `encoding=base64` - Read file as binary and return base64-encoded string
  - `encoding=binary` - Read file as binary and return raw bytes (`Value::Bytes`)

#### Value System
- Added `Value::Bytes` variant for native binary data support
  - Serializes to base64 in YAML/JSON output
  - Returns native Python `bytes` in `to_dict()`
  - Accessible via `value.is_bytes()` and `value.as_bytes()` in Rust

### Fixed

- **Circular reference detection** - Fixed a critical bug where transitive circular references (e.g., A→B→C→A) caused a stack overflow/segfault instead of returning a proper error message. The resolution now correctly tracks the full resolution stack and detects cycles.

### Tests

- Added unit tests for source tracking (5 tests covering merged configs, single file, null removal, and array replacement)
- Added unit tests for circular reference detection (4 tests covering direct, chain, self, and nested cycles)
- Added unit tests for file resolver encoding options (4 tests)
- Added unit tests for boolean coercion edge cases (case-insensitivity and invalid value rejection)
- Added acceptance tests for file resolver encoding (3 tests)
- Added acceptance test for sensitivity inheritance via self-references
- Re-enabled previously disabled circular reference acceptance tests (2 tests)
- Added acceptance tests for optional file support (10 tests covering missing files, merge behavior, deep merge)

## [0.1.3] - 2026-01-09

No change, debugging release process

## [0.1.2] - 2026-01-08

No change, debugging release process

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

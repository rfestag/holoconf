# ADR-008: Error Handling Strategy

## Status

- **Proposed by:** Ryan on 2026-01-07
- **Accepted on:** 2026-01-07

## Context

holoconf operations can fail in various ways:

- Config file not found or unreadable
- YAML/JSON parse errors
- Invalid interpolation syntax
- Resolver failures (network errors, missing secrets, etc.)
- Schema validation failures
- Circular reference detection
- Type coercion failures

We need a consistent error handling strategy that:

- Provides clear, actionable error messages
- Works across FFI boundaries (Rust → Python/JS/etc.)
- Helps users quickly identify and fix problems
- Distinguishes between different failure modes

## Alternatives Considered

### Alternative 1: Single Error Type

All errors are the same type with an error code/message.

```python
try:
    config = Config.load("config.yaml")
except holoconf.Error as e:
    print(e.code, e.message)
```

- **Pros:** Simple FFI, easy to implement
- **Cons:** Hard to handle specific errors programmatically
- **Rejected:** Users need to distinguish resolver failures from parse errors

### Alternative 2: Error Hierarchy (Inheritance)

Structured error types with inheritance.

```python
try:
    config = Config.load("config.yaml")
except holoconf.ResolverError as e:
    # Handle resolver-specific failure
except holoconf.ParseError as e:
    # Handle parse failure
except holoconf.Error as e:
    # Catch-all
```

- **Pros:** Familiar pattern, selective catching, type-safe
- **Cons:** More complex FFI mapping
- **Chosen:** Best balance of usability and type safety

### Alternative 3: Result Type (No Exceptions)

Return result objects instead of throwing.

```python
result = Config.load("config.yaml")
if result.is_err():
    print(result.error)
else:
    config = result.value
```

- **Pros:** Explicit error handling, no hidden control flow
- **Cons:** Unfamiliar in Python/JS, verbose
- **Rejected:** Not idiomatic for target languages

## Open Questions (Proposal Phase)

*All resolved - see Decision section.*

## Next Steps (Proposal Phase)

- [ ] Implement error types in holoconf-core
- [ ] Prototype FFI error mapping in PyO3 and NAPI-RS
- [ ] Add help text for common error scenarios

## Decision

**Error Hierarchy with Rich Context**

- Use inheritance-based error hierarchy for type-safe selective catching
- Include rich context: message, code, path, source file/line, cause chain, help text
- Help text included for common errors; defaults to message when not provided
- Errors map to native exceptions in each language binding

## Design

### Error Hierarchy

```
HoloconfError (base)
├── ConfigError
│   ├── FileNotFoundError
│   ├── ParseError (YAML/JSON syntax)
│   └── MergeError
├── InterpolationError
│   ├── SyntaxError (malformed ${...})
│   ├── CircularReferenceError
│   └── UnknownResolverError
├── ResolverError
│   ├── TimeoutError
│   ├── NetworkError
│   └── NotFoundError (SSM key doesn't exist, etc.)
└── ValidationError
    ├── StructuralValidationError
    └── TypeValidationError
```

### Error Information

Each error includes:

```python
class HoloconfError(Exception):
    message: str          # Human-readable description
    code: str             # Machine-readable code (e.g., "RESOLVER_TIMEOUT")
    path: str | None      # Config path where error occurred (e.g., "database.password")
    source_file: str | None  # Which config file
    source_line: int | None  # Line number if applicable
    cause: Exception | None  # Underlying error (e.g., network error)
    help: str | None      # Recovery suggestion
```

### Error Message Format

```
ResolverError: Failed to resolve SSM parameter
  Path: database.password
  Resolver: ssm
  Key: /prod/db/password
  Source: config.yaml:15
  Cause: AccessDeniedException - User lacks ssm:GetParameter permission
  Help: Ensure IAM role has ssm:GetParameter permission for /prod/db/*
```

### FFI Mapping

Rust errors map to native exceptions:

```rust
// Rust
#[derive(Error, Debug)]
pub enum HoloconfError {
    #[error("Failed to parse config: {message}")]
    ParseError { message: String, line: Option<u32> },

    #[error("Resolver failed: {message}")]
    ResolverError { message: String, path: String, cause: Option<String> },
    // ...
}
```

```python
# Python (via PyO3)
class ParseError(HoloconfError):
    pass

class ResolverError(HoloconfError):
    pass
```

```javascript
// JavaScript (via NAPI-RS)
class ParseError extends HoloconfError {}
class ResolverError extends HoloconfError {}
```

## Rationale

- **Error hierarchy is idiomatic** in Python and JavaScript, enabling selective catching
- **Rich context helps debugging** - users can see exactly where and why errors occurred
- **Help text reduces support burden** - common mistakes get actionable suggestions
- **Cause chain preserves information** - underlying errors aren't lost

## Trade-offs Accepted

- **More complex FFI mapping** in exchange for **type-safe error handling**
- **Larger error objects** in exchange for **better debugging experience**
- **Help text maintenance burden** in exchange for **better user experience**

## Migration

N/A - This is the initial architecture decision.

## Consequences

- **Positive:** Clear error identification, actionable messages, familiar exception patterns
- **Negative:** More code to maintain in FFI layer, help text needs curation
- **Neutral:** Error hierarchy may evolve as new error cases are discovered

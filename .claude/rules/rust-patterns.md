# Rust Patterns

## Project-Specific Conventions

### Error Handling
- Use `thiserror` for error types with `#[derive(Error)]`
- Error hierarchy: `HoloconfError` > `ParseError`, `ValidationError`, `ResolverError`, `PathNotFoundError`
- Use `?` for propagation; explicit `match` only when recovery is needed

### Thread Safety
- Config uses `Arc<RwLock<_>>` for interior mutability
- Resolvers MUST be `Send + Sync` (async execution across threads)
- Prefer `Arc<T>` over `Rc<T>` for shared ownership

### Key Architectural Patterns
- **Config merging** (ADR-004): Later files override. Mappings deep-merge, arrays replace, null removes keys.
- **Lazy resolution** (ADR-005): Values resolve on `get()`, cached after first access.
- **Sensitive values** (ADR-009): Use `ResolvedValue::sensitive(value)`. Shows as `[REDACTED]`.

## Clippy & Formatting (project config)

### Clippy Thresholds (clippy.toml)
- `cognitive-complexity-threshold = 20` - Break up complex functions
- `too-many-lines-threshold = 80` - Extract logic into helpers
- `too-many-arguments-threshold = 6` - Use builder or config struct

### Formatting (rustfmt.toml)
- `max_width = 100` - Line length limit
- `edition = "2021"` - Rust 2021 edition

### Common Clippy Lints to Respect
- `clippy::unwrap_used` - Prefer `expect()` with context or `?`
- `clippy::clone_on_ref_ptr` - Avoid cloning Arc/Rc unnecessarily
- `clippy::large_enum_variant` - Box large variants

## Idiomatic Rust

### Prefer
- `impl Trait` over explicit generics where possible
- `Into<T>` / `AsRef<T>` for flexible APIs
- Builder pattern for types with many optional fields
- `#[must_use]` on functions returning values that shouldn't be ignored

### Avoid
- `unwrap()` in library code - use `expect()` or proper error handling
- `.clone()` unless necessary - prefer references
- Raw string manipulation - use `std::path::Path` for paths

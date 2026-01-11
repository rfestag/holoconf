# CLAUDE.md for holoconf

Cross-language hierarchical configuration library. Rust core with Python bindings (PyO3).

## Build & Test Commands

```bash
make build              # Build Rust + Python bindings
make test               # Run all tests
make lint               # Lint and format check
make check              # Full pre-commit check (lint + security + test)
make test-acceptance    # YAML-driven acceptance tests only

# Rust-specific
cargo test -p holoconf-core
cargo test -p holoconf-core -- test_name   # Filter by name

# Python development
cd packages/python/holoconf
source .venv/bin/activate && maturin develop
```

## Code Layout

- `crates/holoconf-core/` - Rust core library (config, value, resolver, schema)
- `crates/holoconf-cli/` - CLI tool
- `crates/holoconf-python/` - PyO3 bindings
- `packages/python/holoconf/` - Python package with type stubs (`.pyi`)
- `tests/acceptance/` - YAML-driven acceptance tests
- `docs/` - Documentation site (MkDocs)
- `docs/adr` - Architecture Design Records - documentation on high-level design and workflow decisions for the project.
- `docs/sepcs/features` - Feature specifications for core features.

## Adding Features (Test-Driven)

Follow test-driven development: **write tests first, then implement until they pass**.

### New Features

1. **Spec first**: Check/create spec in `docs/specs/features/FEAT-xxx-name.md`
2. **ADR if architectural**: Create `docs/adr/ADR-xxx-topic.md` for design decisions
3. **Write acceptance tests FIRST**: Add tests in `tests/acceptance/` that define expected behavior
4. **Write unit tests FIRST**: Add Rust tests for internal implementation details
5. **Run tests to confirm they fail**: `make test-acceptance` and `cargo test -p holoconf-core`
6. **Implement**: Rust core first, then Python bindings if needed
7. **Iterate until tests pass**: Fix implementation until all tests are green
8. **Type stubs**: Update `packages/python/holoconf/src/holoconf/_holoconf.pyi`
9. **Changelog**: Add entry under `[Unreleased]` in `CHANGELOG.md`
10. **Docs site**: Update relevant pages in `docs/` (see below)
11. **Verify**: Run `make check`

### Modifying Existing Behavior

1. **Find existing tests**: Search `tests/acceptance/` and unit tests for coverage
2. **Update tests FIRST**: Modify tests to reflect the new expected behavior
3. **Run tests to confirm they fail**: Verify tests now fail with old behavior
4. **Implement the change**: Update code until tests pass
5. **Check for regressions**: Run full test suite with `make test`

### Test Guidelines

- **Acceptance tests** (`tests/acceptance/`): For any feature exposed across runtimes (Rust, Python, CLI)
- **Unit tests** (in `#[cfg(test)]` modules): For internal logic, edge cases, regression prevention
- **Coverage target**: 90% line coverage for core modules (see ADR-013)

### Coverage Commands

```bash
make coverage-html       # Unit tests only (fast)
make coverage-full-html  # Unit + acceptance tests (comprehensive)
make coverage-full       # Generate LCOV file for CI
```

Acceptance tests contribute to Rust coverage via the PyO3 bindings. Use `coverage-full-html` to see the complete picture.

## Documentation Site

The docs site is built with MkDocs from `docs/`. When adding user-facing features:

```bash
make docs-build         # Build and verify docs
mkdocs serve            # Preview locally at localhost:8000
```

**Update these pages for new features:**
- `docs/index.md` - Overview/getting started
- `docs/configuration.md` - Config loading, merging, interpolation syntax
- `docs/resolvers.md` - Resolver types (env, file, http, custom)
- `docs/cli.md` - CLI command reference
- `docs/python.md` - Python API examples
- `docs/schema.md` - JSON Schema validation

Add new pages to `mkdocs.yml` nav section if needed.

## Key Patterns

**Config merging** (ADR-004): Later files override earlier. Mappings deep-merge, arrays replace entirely, null removes keys.

**Lazy resolution** (ADR-005): Values resolve on `get()`, not at parse time. Cached after first access.

**Sensitive values** (ADR-009): Use `ResolvedValue::sensitive(value)` for secrets. They show as `[REDACTED]` with `redact=true`.

**Error hierarchy**: `HoloconfError` > `ParseError`, `ValidationError`, `ResolverError`, `PathNotFoundError`, etc.

**Thread safety**: Config uses `Arc<RwLock<_>>` for cache. Resolvers must be `Send + Sync`.

## Interpolation Syntax

```yaml
host: ${env:DB_HOST}                       # Environment variable
host: ${env:DB_HOST,default=localhost}     # With default value (keyword syntax)
secret: ${env:API_KEY,sensitive=true}      # Mark as sensitive for redaction
config: ${file:./other.yaml}               # File include
url: postgres://${.host}:5432              # Self-reference (relative path)
url: ${database.host}                      # Self-reference (absolute path)
```

Note: `default` and `sensitive` are framework-level kwargs handled uniformly across all resolvers (ADR-011).

## Acceptance Test Format

Location: `tests/acceptance/`. Tests are YAML files:

```yaml
name: descriptive_test_name
given:
  env: { VAR: "value" }
  config: |
    key: ${env:VAR}
when:
  access: key
then:
  value: "value"
```

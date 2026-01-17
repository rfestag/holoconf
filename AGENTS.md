# holoconf

Cross-language hierarchical configuration library. Rust core with Python bindings (PyO3).

## Quick Reference

```bash
make check              # Full pre-commit validation
make test               # Run all tests
make build              # Build Rust + Python
```

## Code Layout

- `crates/holoconf-core/` - Rust core library
- `crates/holoconf-cli/` - CLI tool
- `crates/holoconf-python/` - PyO3 bindings
- `packages/python/holoconf/` - Python package with type stubs
- `tests/acceptance/` - YAML-driven acceptance tests
- `docs/adr/` - Architecture Decision Records
- `docs/specs/features/` - Feature specifications

## Rules

@.claude/rules/tdd-workflow.md
@.claude/rules/github-workflow.md
@.claude/rules/rust-patterns.md
@.claude/rules/python-patterns.md
@.claude/rules/build-test.md
@.claude/rules/acceptance-tests.md

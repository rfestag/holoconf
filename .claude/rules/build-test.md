# Build & Test Commands

## Quick Commands
```bash
make build              # Build Rust + Python bindings
make test               # Run all tests
make lint               # Lint and format check
make check              # Full pre-commit check
make test-acceptance    # YAML-driven acceptance tests
```

## Rust-specific
```bash
cargo test -p holoconf-core
cargo test -p holoconf-core -- test_name
```

## Python development
```bash
cd packages/python/holoconf
source .venv/bin/activate && maturin develop
```

## Coverage
```bash
make coverage-html       # Unit tests only
make coverage-full-html  # Unit + acceptance tests
```

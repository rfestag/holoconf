# Code Coverage

holoconf tracks code coverage for both Rust and Python code to ensure comprehensive testing.

!!! tip "Generate reports first"
    Run `make coverage` to generate the coverage data shown below.

## Coverage Summary

| Package | Coverage |
|---------|----------|
| Rust (holoconf-core) | <!-- coverage:rust:summary --> |
| Python (holoconf) | <!-- coverage:python:summary --> |

For detailed file-level coverage, see the [Rust API docs](../api/rust/index.md#code-coverage) and [Python API docs](../api/python/index.md#code-coverage).

## Acceptance Tests

Acceptance tests validate end-to-end behavior across language bindings. The matrix below shows which tests pass for each driver.

<!-- acceptance:matrix -->

!!! note "Run acceptance tests"
    Run `make test-acceptance` to generate test results, or `make test-acceptance-json` to update the matrix data.

---

## Running Coverage Locally

### Generate Coverage Reports

```bash
make coverage            # Unit tests only (Rust + Python)
make coverage-acceptance # Acceptance tests with Rust instrumentation
make coverage-full       # Combined: Rust unit tests + acceptance tests
```

This generates machine-readable coverage data:

- `coverage/rust-lcov.info` - Rust coverage in LCOV format
- `coverage/python-coverage.xml` - Python coverage in Cobertura XML format
- `coverage/acceptance-lcov.info` - Acceptance test coverage in LCOV format

The tables above are generated from these files at documentation build time.

### Generate HTML Reports

For detailed line-by-line coverage reports:

```bash
make coverage-html
```

This generates interactive HTML reports in `docs/coverage/`:

- **Rust**: `docs/coverage/rust/html/index.html`
- **Python**: `docs/coverage/python/index.html`

View in docs with live reload:

```bash
make docs-serve
# Open http://127.0.0.1:8000/contributing/coverage/
```

## Coverage Architecture

### Why Multiple Coverage Reports?

holoconf uses a **Rust core with Python bindings** architecture (see [ADR-001](../adr/ADR-001-multi-language-architecture.md)). This means:

1. **Rust unit tests** exercise the core library directly
2. **Python unit tests** exercise the PyO3 bindings
3. **Acceptance tests** validate end-to-end behavior through language bindings

### Tools Used

| Language | Tool | Output |
|----------|------|--------|
| Rust | [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) | LCOV, HTML |
| Python | [pytest-cov](https://pytest-cov.readthedocs.io/) | XML, HTML |

## Coverage Goals

We aim for:

- **80%+ line coverage** on `holoconf-core`
- **Meaningful coverage** - focus on testing behavior, not just lines
- **No coverage regressions** on pull requests

Coverage requirements are informational, not blocking, to avoid gaming the metrics.

## Installing Coverage Tools

Coverage tools are installed automatically with:

```bash
make install-tools
```

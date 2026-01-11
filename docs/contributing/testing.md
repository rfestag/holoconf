# Testing

HoloConf uses a multi-layered testing strategy to ensure correctness across all language bindings.

## Test Layers

| Layer | Purpose | Location |
|-------|---------|----------|
| **Unit tests** | Test individual components | `crates/*/src/`, `packages/*/tests/` |
| **Acceptance tests** | Cross-language behavior verification | `tests/acceptance/` |
| **Integration tests** | End-to-end scenarios | `tests/integration/` |

## Running Tests

### All Tests

```bash
make test
```

### Rust Unit Tests

```bash
cargo test --all
```

### Python Unit Tests

```bash
cd packages/python/holoconf
pytest tests/ -v
```

### Acceptance Tests

```bash
# Run with Rust driver
python tools/test_runner.py --driver rust 'tests/acceptance/**/*.yaml' -v

# Run with Python driver
python tools/test_runner.py --driver python 'tests/acceptance/**/*.yaml' -v

# Run a specific test file
python tools/test_runner.py --driver python tests/acceptance/resolvers/env.yaml -v
```

See [Acceptance Tests](acceptance-tests.md) for test format details and the full test matrix.

## Writing Good Tests

### Do

- Test one behavior per test case
- Use descriptive test names
- Cover both success and error cases
- Test edge cases and boundaries
- Keep test data minimal but realistic

### Don't

- Don't test implementation details
- Don't rely on external services
- Don't use random/time-dependent data
- Don't write overly complex test configs

## Debugging Tests

### Verbose Output

```bash
python tools/test_runner.py --driver python -vv tests/acceptance/...
```

### Run Single Test

```bash
python tools/test_runner.py --driver python --test "specific test name" tests/acceptance/...
```

### Debug in Python

```python
from holoconf import Config

config = Config.from_string("""
database:
  host: ${env:DB_HOST,default=localhost}
""")

# Inspect values
print(config.dump(resolve=True))
```

## Code Coverage

### Coverage Goals

| Package | Target | Notes |
|---------|--------|-------|
| **holoconf-core** (Rust) | 80% | Core logic, all public APIs |
| **holoconf** (Python) | 80% | Bindings and Python-specific code |
| **holoconf-cli** | 70% | CLI commands and argument handling |

### Generating Coverage Reports

#### Rust Coverage

```bash
# Generate HTML coverage report
make coverage-rust

# View report
open target/llvm-cov/html/index.html
```

Requires `cargo-llvm-cov`:
```bash
cargo install cargo-llvm-cov
```

#### Python Coverage

```bash
cd packages/python/holoconf
pytest --cov=holoconf --cov-report=html tests/
open htmlcov/index.html
```

#### Full Coverage (Rust + Python + Acceptance)

```bash
make coverage-full
```

This generates a combined report covering:

- Rust unit tests
- Python unit tests
- Acceptance tests run through both drivers

### Coverage in CI

Coverage reports are automatically generated on pull requests. The CI will:

1. Run all test suites with coverage instrumentation
2. Generate combined coverage reports
3. Post coverage summary as a PR comment
4. Fail if coverage drops below thresholds

## See Also

- [Acceptance Tests](acceptance-tests.md) - Cross-language acceptance test details
- [ADR-013 Testing Architecture](../adr/ADR-013-testing-architecture.md) - Design rationale for the testing approach

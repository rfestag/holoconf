# Testing

holoconf uses a multi-layered testing strategy to ensure correctness across all language bindings.

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

## Acceptance Test Format

Acceptance tests are defined in YAML and run against all language bindings. This ensures consistent behavior across languages.

### Basic Structure

```yaml
name: "Feature name"
description: "What this test file covers"

tests:
  - name: "Test case description"
    config:
      key: value
      nested:
        path: "${env:HOME}"
    assertions:
      - path: "key"
        expected: "value"
```

### Assertions

```yaml
assertions:
  # Exact value match
  - path: "database.port"
    expected: 5432

  # Pattern match (regex)
  - path: "app.version"
    expected_pattern: "^\\d+\\.\\d+\\.\\d+$"

  # Type check
  - path: "app.debug"
    expected_type: "boolean"

  # Error expectation
  - path: "missing.key"
    expected_error: "PathNotFoundError"
```

### Environment Setup

```yaml
tests:
  - name: "Environment variable resolution"
    env:
      DATABASE_URL: "postgres://localhost/test"
    config:
      db_url: "${env:DATABASE_URL}"
    assertions:
      - path: "db_url"
        expected: "postgres://localhost/test"
```

### File Fixtures

```yaml
tests:
  - name: "File include"
    files:
      secrets.yaml: |
        api_key: "secret123"
    config:
      secrets: "${file:secrets.yaml}"
    assertions:
      - path: "secrets.api_key"
        expected: "secret123"
```

## Test Organization

```
tests/acceptance/
├── api/                  # API behavior tests
├── interpolation/        # Interpolation syntax tests
├── merging/              # Config merging tests
├── resolvers/            # Resolver tests
│   ├── env.yaml          # Environment resolver
│   ├── file.yaml         # File resolver
│   └── self-ref.yaml     # Self-reference resolver
├── schema/               # Schema validation tests
└── serialization/        # Export/dump tests
```

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

## Test Coverage

### Adding Tests for New Features

1. **Create acceptance tests first** (TDD approach):
   ```bash
   # Create test file
   touch tests/acceptance/feature/new-feature.yaml
   ```

2. **Define expected behavior** in YAML

3. **Run tests** (they should fail initially):
   ```bash
   python tools/test_runner.py --driver python tests/acceptance/feature/new-feature.yaml
   ```

4. **Implement the feature**

5. **Verify tests pass** for all drivers

### Test Requirements for PRs

- All new features must have acceptance tests
- Bug fixes should include regression tests
- Tests must pass for all language bindings
- No decrease in overall test coverage

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
  host: ${env:DB_HOST,localhost}
""")

# Inspect values
print(config.dump(resolve=True))
```

## See Also

- [ADR-013 Testing Architecture](../adr/ADR-013-testing-architecture.md) - Design rationale for the testing approach

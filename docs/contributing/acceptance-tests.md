# Acceptance Tests

Acceptance tests validate end-to-end behavior across all language bindings. They ensure that HoloConf behaves consistently whether you're using Rust, Python, or any other supported language.

!!! tip "Run acceptance tests"
    Run `make test-acceptance` to run all acceptance tests, or `make test-acceptance-json` to generate the results matrix.

## Test Matrix

The matrix below shows which tests pass for each driver:

<!-- acceptance:matrix -->

## Running Acceptance Tests

### All Tests

```bash
make test-acceptance
```

### By Driver

```bash
# Run with Rust driver
python tools/test_runner.py --driver rust 'tests/acceptance/**/*.yaml' -v

# Run with Python driver
python tools/test_runner.py --driver python 'tests/acceptance/**/*.yaml' -v
```

### Specific Test File

```bash
python tools/test_runner.py --driver python tests/acceptance/resolvers/env.yaml -v
```

## Test Organization

Acceptance tests are organized by feature area:

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

## Test Format

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

## Adding New Tests

1. Create a YAML file in the appropriate `tests/acceptance/` subdirectory
2. Define test cases with config and assertions
3. Run against all drivers to verify consistent behavior:

```bash
python tools/test_runner.py --driver rust tests/acceptance/your-test.yaml -v
python tools/test_runner.py --driver python tests/acceptance/your-test.yaml -v
```

## See Also

- [Testing](testing.md) - Full testing guide including coverage goals
- [ADR-013 Testing Architecture](../adr/ADR-013-testing-architecture.md) - Design rationale

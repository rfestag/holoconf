---
description: Run targeted tests
---

# Run Tests: $ARGUMENTS

Run tests with optional filtering.

## Usage

| Command | Action |
|---------|--------|
| `/test` or `/test all` | Run all tests (`make test`) |
| `/test unit` | Rust unit tests only (`cargo test -p holoconf-core`) |
| `/test acceptance` | Acceptance tests only (`make test-acceptance`) |
| `/test <pattern>` | Filter tests by name (`cargo test -p holoconf-core -- <pattern>`) |

## Steps

1. **Parse arguments**:
   - Empty or `all` → `make test`
   - `unit` → `cargo test -p holoconf-core`
   - `acceptance` → `make test-acceptance`
   - Other → `cargo test -p holoconf-core -- $ARGUMENTS`

2. **Run the appropriate command**

3. **On failure**:
   - Show the failing test output
   - Identify the test file and line number
   - Analyze the failure and suggest potential fixes

4. **On success**:
   - Report test count and timing
   - Suggest next steps if appropriate

## Examples

```
/test                     # Run all tests
/test unit                # Rust unit tests
/test acceptance          # YAML acceptance tests
/test interpolation       # Tests matching "interpolation"
/test resolver::env       # Tests in resolver::env module
```

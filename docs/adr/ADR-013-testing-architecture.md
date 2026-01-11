# ADR-013: Testing Architecture

## Status

- **Proposed by:** Ryan on 2026-01-07
- **Accepted on:** 2026-01-07

## Context

Holoconf is a multi-language configuration library with a Rust core and bindings for Python, JavaScript, and Go. We need a testing strategy that:

1. Ensures feature parity across all language implementations
2. Expresses tests in domain language (configuration, resolvers, schemas, etc.)
3. Allows writing tests once and running them against all implementations
4. Separates test logic from implementation details

This is critical because:
- Bugs in one binding but not another are hard to catch without shared tests
- Configuration semantics must be identical across languages
- Maintaining separate test suites per language leads to divergence

## Alternatives Considered

### Alternative 1: Language-Specific Test Suites

Each language binding has its own independent test suite written idiomatically.

- **Pros:** Idiomatic tests, easy to write, no shared infrastructure
- **Cons:** No feature parity guarantee, duplicate effort, tests diverge over time

### Alternative 2: Rust-Only Testing

Test only the Rust core extensively; assume bindings are thin wrappers.

- **Pros:** Simple, fast, single test suite
- **Cons:** Binding bugs undetected, FFI edge cases missed, no validation that bindings work correctly

### Alternative 3: Cucumber/Gherkin

Use Gherkin syntax for test definitions with Cucumber runners per language.

- **Pros:** Human-readable "Given/When/Then" format, established tooling
- **Cons:** Step definition mapping overhead, natural language parsing required, Gherkin's strength (stakeholder readability) isn't valuable when our audience is developers, adds dependency across all languages

### Alternative 4: Four-Tier Testing Architecture with YAML

A layered architecture separating test definitions from execution, using YAML for definitions.

```
┌─────────────────────────────────────────────────────────┐
│                    Test Definitions                      │
│              (YAML - what to test, expected results)     │
├─────────────────────────────────────────────────────────┤
│                    DSL Layer                             │
│    (Domain operations: given_config, when_resolved...)   │
├──────────┬──────────┬──────────┬──────────┬────────────┤
│  Rust    │  Python  │    JS    │    Go    │   Future   │
│  Driver  │  Driver  │  Driver  │  Driver  │   Driver   │
├──────────┼──────────┼──────────┼──────────┼────────────┤
│  Rust    │  Python  │    JS    │    Go    │   Future   │
│  Library │ Bindings │ Bindings │ Bindings │  Bindings  │
└──────────┴──────────┴──────────┴──────────┴────────────┘
```

- **Pros:** Feature parity enforced, write once run everywhere, domain-focused tests, YAML matches our config domain
- **Cons:** Upfront infrastructure investment, shared test format design needed

## Open Questions (Proposal Phase)

*All resolved - see Decision section.*

## Next Steps (Proposal Phase)

- [ ] Define the DSL operations vocabulary
- [ ] Implement Rust driver as reference
- [ ] Implement one binding driver (Python) to validate architecture
- [ ] Create initial acceptance test suite

## Decision

**Four-Tier Testing Architecture** with YAML test definitions, covering acceptance tests, unit tests, and performance benchmarks.

### Why YAML over Gherkin

1. **Config files are already YAML** - Test definitions like `given: config: |` feel natural and consistent with the domain
2. **No step definition mapping** - YAML is already structured; drivers consume it directly without parsing natural language
3. **Universal parsing** - Every language has robust YAML support; Cucumber support varies
4. **Developer audience** - Gherkin's strength is stakeholder readability; our users are developers comfortable with structured data

## Design

### Tier 1: Test Definitions (Universal)

Tests are defined in YAML that describes:
- Setup (config files, environment, resolvers)
- Actions (load, access, resolve, validate)
- Assertions (expected values, errors, behaviors)

```yaml
# tests/acceptance/resolvers/env_resolver.yaml
suite: env_resolver
description: Environment variable resolver behavior
tests:
  - name: resolves_environment_variable
    given:
      env:
        PORT: "8080"
      config: |
        port: ${env:PORT}
    when:
      access: port
    then:
      value: "8080"

  - name: uses_default_when_missing
    given:
      config: |
        port: ${env:UNDEFINED_VAR,default=3000}
    when:
      access: port
    then:
      value: "3000"

  - name: errors_when_missing_no_default
    given:
      config: |
        port: ${env:UNDEFINED_VAR}
    when:
      access: port
    then:
      error:
        type: ResolverError
        message_contains: "Environment variable not found"
```

### Tier 2: DSL Layer (Per-Language)

The DSL provides domain operations in each language. These are high-level, readable operations that map to the test definition vocabulary.

**Python DSL Example:**
```python
# holoconf_test/dsl.py
class ConfigTestDSL:
    def given_env(self, env: dict[str, str]) -> Self: ...
    def given_config(self, yaml_content: str) -> Self: ...
    def given_config_file(self, path: str) -> Self: ...
    def given_schema(self, schema: dict) -> Self: ...
    def given_resolver(self, name: str, resolver: Callable) -> Self: ...

    def when_load(self) -> Self: ...
    def when_access(self, path: str) -> Self: ...
    def when_resolve_all(self) -> Self: ...
    def when_validate(self) -> Self: ...
    def when_export(self, format: str) -> Self: ...

    def then_value_equals(self, expected: Any) -> Self: ...
    def then_value_is_type(self, expected_type: type) -> Self: ...
    def then_error(self, error_type: str, message_contains: str = None) -> Self: ...
    def then_config_equals(self, expected: dict) -> Self: ...
```

**Rust DSL Example:**
```rust
// holoconf_test/src/dsl.rs
impl ConfigTestDSL {
    pub fn given_env(mut self, env: HashMap<String, String>) -> Self { ... }
    pub fn given_config(mut self, yaml: &str) -> Self { ... }
    pub fn given_schema(mut self, schema: Value) -> Self { ... }

    pub fn when_load(mut self) -> Self { ... }
    pub fn when_access(mut self, path: &str) -> Self { ... }

    pub fn then_value_equals<T: PartialEq>(self, expected: T) -> Self { ... }
    pub fn then_error(self, error_type: &str) -> Self { ... }
}
```

### Tier 3: Drivers (Per-Language)

Drivers translate DSL operations into actual library calls. They handle language-specific details like error handling, type conversion, and FFI boundaries.

Drivers are **hand-written** for each language binding. This is preferred over auto-generation because:
- Only 4 languages to support
- Driver logic involves language-specific idioms
- Auto-generation would require a meta-schema adding complexity

**Python Driver:**
```python
# holoconf_test/driver.py
class HoloconfDriver:
    def __init__(self):
        self._config = None
        self._last_result = None
        self._last_error = None

    def load_config(self, yaml_content: str, **options):
        """Actually calls holoconf.Config.load()"""
        try:
            self._config = Config.loads(yaml_content, **options)
        except Exception as e:
            self._last_error = e

    def access_path(self, path: str):
        """Actually accesses config.path.to.value"""
        try:
            self._last_result = self._config.get(path)
        except Exception as e:
            self._last_error = e

    def get_last_result(self): return self._last_result
    def get_last_error(self): return self._last_error
```

### Tier 4: System Under Test

The actual holoconf libraries - Rust core and language bindings.

## Test Categories

### Acceptance Tests (Cross-Language)

High-level tests that verify documented behavior from a user's perspective. Run against all implementations to ensure feature parity.

**Characteristics:**
- Written in YAML, executed via the four-tier architecture
- Test observable behavior, not implementation details
- One test definition runs against Rust, Python, JS, and Go
- If it's in the documentation, there's an acceptance test for it

**What they cover:**
- Resolver behavior (env, self, file, http)
- Merging semantics
- Schema validation
- Type coercion
- Error messages and help text
- Interpolation syntax
- API surface (load, access, export)

**Location:** `tests/acceptance/`

```
tests/acceptance/
├── resolvers/
│   ├── env_resolver.yaml
│   ├── self_resolver.yaml
│   ├── file_resolver.yaml
│   └── http_resolver.yaml
├── merging/
│   ├── deep_merge.yaml
│   └── override_semantics.yaml
├── schema/
│   ├── type_validation.yaml
│   └── coercion.yaml
├── interpolation/
│   ├── syntax.yaml
│   ├── escaping.yaml
│   └── nesting.yaml
└── errors/
    ├── resolver_errors.yaml
    └── validation_errors.yaml
```

### Unit Tests (Language-Specific)

Low-level tests for implementation details, internal functions, and edge cases not visible through the public API.

**Characteristics:**
- Written idiomatically in each language's test framework
- Test internal implementation details
- May test private/internal APIs
- Not required to be cross-language

**What they cover:**
- Parser internals and edge cases
- Memory management (Rust)
- FFI boundary correctness (bindings)
- Language-specific type conversions
- Internal helper functions

**Location:**
- Rust: `crates/holoconf-core/src/**/*_test.rs` or `crates/holoconf-core/tests/`
- Python: `bindings/python/tests/unit/`
- JS: `bindings/js/tests/unit/`
- Go: `bindings/go/**/*_test.go`

### Performance Tests

Benchmark tests that verify performance characteristics. These use the same four-tier architecture with performance-specific assertions.

**Characteristics:**
- Written in YAML like acceptance tests
- Use `then: performance:` assertions instead of value assertions
- Run separately from functional tests (longer execution time)
- Establish baselines and detect regressions

**What they cover:**
- Config loading time (small, medium, large files)
- Resolution latency (single value, bulk resolution)
- Memory usage during resolution
- Concurrent access performance

**Location:** `tests/performance/`

```yaml
# tests/performance/loading.yaml
suite: config_loading_performance
tests:
  - name: small_config_loads_quickly
    given:
      config_file: fixtures/small_config.yaml  # ~50 keys
    when:
      load: {}
    then:
      performance:
        max_duration_ms: 10
        max_memory_mb: 5

  - name: large_config_loads_reasonably
    given:
      config_file: fixtures/large_config.yaml  # ~10,000 keys
    when:
      load: {}
    then:
      performance:
        max_duration_ms: 500
        max_memory_mb: 50

  - name: bulk_resolution_scales_linearly
    given:
      config_file: fixtures/many_resolvers.yaml  # 1000 env vars
      env_from_file: fixtures/env_vars.json
    when:
      resolve_all: {}
    then:
      performance:
        max_duration_ms: 200
```

## Test Runner

A universal test runner loads YAML test definitions and executes them against each driver:

```python
# tools/test_runner.py
def run_tests(driver: str, test_files: list[str], category: str = "acceptance"):
    """
    Run tests against a specific driver.

    Usage:
        python tools/test_runner.py --driver python tests/acceptance/**/*.yaml
        python tools/test_runner.py --driver rust tests/performance/**/*.yaml
    """
    driver_impl = load_driver(driver)  # Python, Rust, JS, Go

    for test_file in test_files:
        suite = load_test_suite(test_file)
        for test in suite.tests:
            dsl = ConfigTestDSL(driver_impl)
            execute_test(dsl, test)
```

## Coverage Requirements

| Category | Coverage Target | Rationale |
|----------|----------------|-----------|
| Acceptance Tests | 100% of documented features | Feature parity guarantee |
| Unit Tests (Core) | 90% line coverage | Critical path coverage |
| Unit Tests (Bindings) | 80% line coverage | FFI edge cases |
| Performance Tests | Key operations baselined | Regression detection |

## CI Integration

```yaml
# .github/workflows/test.yml
jobs:
  acceptance-tests:
    strategy:
      matrix:
        driver: [rust, python, js, go]
    steps:
      - run: python tools/test_runner.py --driver ${{ matrix.driver }} tests/acceptance/**/*.yaml

  unit-tests-rust:
    steps:
      - run: cargo test --workspace

  unit-tests-bindings:
    strategy:
      matrix:
        binding: [python, js, go]
    steps:
      - run: cd bindings/${{ matrix.binding }} && make test

  performance-tests:
    # Run on main branch only, or on-demand
    if: github.ref == 'refs/heads/main'
    strategy:
      matrix:
        driver: [rust, python]  # Primary implementations
    steps:
      - run: python tools/test_runner.py --driver ${{ matrix.driver }} tests/performance/**/*.yaml
```

## Rationale

- **Four-tier separation** keeps test logic independent of implementation
- **YAML over Gherkin** because our domain is already YAML-based and our audience is developers
- **Acceptance + Unit separation** allows high-level feature parity tests while still testing implementation details
- **Performance in same architecture** reuses infrastructure while keeping concerns separate
- **Hand-written drivers** are simpler than auto-generation for 4 languages

## Trade-offs Accepted

- **Upfront infrastructure investment** to build test framework, in exchange for **long-term maintainability and feature parity**
- **YAML less readable than Gherkin** for non-developers, in exchange for **simpler implementation and domain consistency**
- **Acceptance tests may not cover all edge cases**, addressed by **language-specific unit tests**
- **Performance tests in YAML** have limited expressiveness, in exchange for **consistent tooling**

## Migration

N/A - This is the initial architecture decision.

## Consequences

- **Positive:** Feature parity enforced across all bindings
- **Positive:** Tests written once, run everywhere
- **Positive:** Adding new language bindings just requires writing a driver
- **Positive:** Clear separation between "what behavior" (acceptance) and "how implemented" (unit)
- **Negative:** Initial setup requires building test infrastructure
- **Neutral:** Test failures clearly indicate which binding has issues

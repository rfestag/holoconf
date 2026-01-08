# ADR-014: Code Quality and Security Tooling

## Status

- **Proposed by:** Claude on 2026-01-08
- **Accepted on:** 2026-01-08

## Context

As holoconf grows with multiple language bindings (Rust core, Python, future Node.js/Go), we need consistent code quality and security standards across all languages. Without standardized tooling:

1. Code style varies between contributors
2. Security vulnerabilities may go undetected
3. Unused dependencies accumulate
4. License compliance is not verified
5. Code complexity grows unchecked

## Alternatives Considered

### Alternative 1: Manual Code Review Only

- **Pros:** No tooling overhead, flexible
- **Cons:** Inconsistent, error-prone, doesn't scale, security issues missed

### Alternative 2: Language-Specific Tools (No Standardization)

- **Pros:** Each language uses its best-of-breed tools
- **Cons:** Inconsistent CI configuration, harder to maintain, no unified quality gate

### Alternative 3: Unified Quality Pipeline with Language-Specific Tools

- **Pros:** Best tools for each language, unified CI structure, comprehensive coverage
- **Cons:** More CI configuration, multiple tools to learn

## Decision

Adopt **Alternative 3**: Unified quality pipeline with best-of-breed tools for each language.

## Design

### Rust Tooling

| Tool | Purpose | Configuration |
|------|---------|---------------|
| **rustfmt** | Code formatting | `rustfmt.toml` |
| **clippy** | Linting, complexity | `clippy.toml` |
| **cargo-deny** | License & advisory audit | `deny.toml` |
| **cargo-audit** | CVE vulnerability scan | - |
| **cargo-machete** | Unused dependency detection | - |

#### Clippy Configuration (`clippy.toml`)

```toml
cognitive-complexity-threshold = 20
too-many-lines-threshold = 80
too-many-arguments-threshold = 6
```

#### Cargo Deny Configuration (`deny.toml`)

- Deny known vulnerabilities
- Warn on unmaintained crates
- Allow only approved licenses (MIT, Apache-2.0, BSD, ISC, etc.)
- Warn on multiple versions of same crate

### Python Tooling

| Tool | Purpose | Configuration |
|------|---------|---------------|
| **ruff** | Linting + formatting | `pyproject.toml` |
| **pip-audit** | Dependency security scan | - |

#### Ruff Configuration

```toml
[tool.ruff.lint]
select = [
    "E",      # pycodestyle errors
    "W",      # pycodestyle warnings
    "F",      # Pyflakes
    "I",      # isort
    "B",      # flake8-bugbear
    "C4",     # flake8-comprehensions
    "UP",     # pyupgrade
    "S",      # flake8-bandit (security)
    "SIM",    # flake8-simplify
    "RUF",    # Ruff-specific rules
]
```

### CI Pipeline Structure

```
quality.yml
├── rust-lint (rustfmt, clippy)
├── rust-security (cargo-deny, cargo-audit)
├── rust-unused (cargo-machete)
├── python-lint (ruff check, ruff format)
├── python-security (pip-audit)
└── complexity (tokei - informational)
```

### Local Development Commands

A `Makefile` provides unified commands across all languages:

```bash
make help          # Show all available commands
make lint          # Run all linters (Rust + Python)
make format        # Format all code
make security      # Run all security checks
make test          # Run all tests
make check         # Run everything (lint + security + test)
```

Individual language commands are also available:

```bash
# Rust
make lint-rust     # clippy + fmt check
make format-rust   # cargo fmt
make security-rust # cargo-deny + cargo-audit
make test-rust     # cargo test

# Python
make lint-python     # ruff check + format check
make format-python   # ruff format + fix
make security-python # pip-audit
make test-python     # pytest
```

Or run tools directly:

```bash
# Rust
cargo fmt --all                     # Format
cargo clippy --all-targets          # Lint
cargo deny check                    # License/security
cargo audit                         # CVE scan

# Python (from packages/python/holoconf/)
ruff check src/ tests/              # Lint
ruff format src/ tests/             # Format
```

## Rationale

1. **Rust-focused complexity checks** - Most logic is in Rust, so clippy's cognitive complexity and function length limits are the primary complexity gates

2. **Security at dependency level** - Both cargo-audit/cargo-deny (Rust) and pip-audit (Python) catch CVEs in dependencies before they ship

3. **Ruff over Black/flake8** - Ruff is significantly faster and combines formatting + linting in one tool

4. **Separate CI jobs** - Allows parallel execution and clear failure identification

## Trade-offs Accepted

- **More CI time** in exchange for **comprehensive quality gates**
- **Learning multiple tools** in exchange for **best-of-breed per language**
- **Stricter defaults** in exchange for **cleaner codebase** (can allow specific lints when justified)

## Migration

N/A - This ADR establishes tooling for an existing codebase. Initial cleanup was performed:
- Fixed clippy warnings (PI constant in tests, wildcard patterns)
- Fixed ruff issues (import sorting, deprecated typing imports)

## Consequences

- **Positive:**
  - Consistent code style across all contributors
  - Security vulnerabilities caught before merge
  - License compliance verified automatically
  - Code complexity stays manageable
  - Unused dependencies removed

- **Negative:**
  - CI runs take longer (~2-3 min additional)
  - Contributors must install tools locally for pre-commit checks
  - Some false positives may require `#[allow(...)]` annotations

- **Neutral:**
  - Existing code required minor cleanup to pass new lints

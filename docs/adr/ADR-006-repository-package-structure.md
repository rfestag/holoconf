# ADR-006: Repository and Package Structure

## Status

- **Proposed by:** Ryan on 2026-01-07
- **Accepted on:** 2026-01-07

## Context

holoconf is a multi-language configuration library with:

- A Rust core (`holoconf-core`)
- Rust resolver packages (`holoconf-aws`, `holoconf-vault`, etc.)
- Language bindings (Python, JavaScript, Go, etc.)
- Language-specific resolver wrapper packages

We need to define how the repository is organized and how packages relate to each other.

## Alternatives Considered

### Alternative 1: Polyrepo (separate repositories)

Each component in its own repository:
- `holoconf-core` repo
- `holoconf-python` repo
- `holoconf-js` repo
- `holoconf-aws` repo

- **Pros:** Independent versioning, clear ownership, smaller clones
- **Cons:** Cross-repo changes are painful, version coordination is complex, harder to ensure consistency
- **Rejected:** Too much coordination overhead for a project where core changes often require binding updates

### Alternative 2: Language-first monorepo

Organize by language:
```
python/
  holoconf/
  holoconf-aws/
javascript/
  holoconf/
  holoconf-aws/
rust/
  holoconf-core/
```

- **Pros:** Language teams can work independently
- **Cons:** Duplicates resolver logic across languages, harder to see the full picture
- **Rejected:** Resolvers are Rust crates shared across languages, not per-language

## Decision

**Component-first monorepo with Cargo workspace**

All code lives in a single repository, organized by component type.

Key decisions:
- **Conformance tests at top-level** in `tests/conformance/` with shared fixtures
- **Language-specific resolver wrappers** in `packages/<language>/holoconf-<resolver>/` that re-export Rust resolvers

## Design

### Repository Layout

```
holoconf/
├── docs/                          # Documentation (exists)
│   ├── adr/                       # Architecture Decision Records
│   ├── specs/features/            # Feature specifications
│   ├── PROBLEM_STATEMENT.md
│   ├── REQUIREMENTS.md
│   └── CONSTRAINTS.md
│
├── crates/                        # Rust crates (Cargo workspace)
│   ├── holoconf-core/             # Core library: parsing, merging, resolution
│   ├── holoconf-aws/              # AWS resolvers (SSM, S3, CloudFormation)
│   ├── holoconf-gcp/              # GCP resolvers (future)
│   ├── holoconf-vault/            # HashiCorp Vault resolvers (future)
│   ├── holoconf-python/           # Python bindings (PyO3)
│   └── holoconf-js/               # JavaScript bindings (NAPI-RS)
│
├── packages/                      # Language-specific package metadata/wrappers
│   ├── python/                    # Python package (pyproject.toml, re-exports)
│   │   ├── holoconf/              # Main Python package
│   │   └── holoconf-aws/          # AWS resolver Python wrapper
│   └── javascript/                # npm packages
│       ├── holoconf/              # Main npm package
│       └── holoconf-aws/          # AWS resolver npm wrapper
│
├── tests/                         # Cross-language conformance tests
│   └── conformance/               # Shared test fixtures (YAML in, expected out)
│
├── examples/                      # Usage examples per language
│   ├── python/
│   └── javascript/
│
├── Cargo.toml                     # Cargo workspace root
├── Cargo.lock
└── README.md
```

### Conformance Tests

Conformance tests ensure all language bindings behave identically. They live at the top level (`tests/conformance/`) rather than under each binding because:

- **Shared fixtures** - Same YAML input files and expected outputs for all languages
- **Cross-language validation** - Easy to verify Python and JS produce identical results
- **Single source of truth** - Adding a test case automatically applies to all languages

```
tests/conformance/
├── fixtures/
│   ├── basic/
│   │   ├── input.yaml
│   │   └── expected.json
│   ├── merging/
│   │   ├── base.yaml
│   │   ├── override.yaml
│   │   └── expected.json
│   └── resolvers/
│       ├── env/
│       └── self-ref/
├── python/
│   └── test_conformance.py      # Runs fixtures against Python binding
└── javascript/
    └── conformance.test.js      # Runs fixtures against JS binding
```

### Language-Specific Resolver Wrappers

Resolver wrappers (e.g., `holoconf-aws` on PyPI) are thin packages that:
1. Depend on the main `holoconf` package
2. Re-export the Rust resolver registrations
3. Provide language-idiomatic installation (`pip install holoconf-aws`)

```python
# packages/python/holoconf-aws/src/holoconf_aws/__init__.py
from holoconf._native import aws_resolvers

# Re-export resolver registration functions
ssm = aws_resolvers.ssm
s3 = aws_resolvers.s3
cloudformation = aws_resolvers.cloudformation

# For package-level registration
__holoconf_resolvers__ = {
    "ssm": ssm,
    "s3": s3,
    "cfn": cloudformation,
}
```

This approach:
- Keeps resolver logic in Rust (consistent behavior)
- Provides familiar package installation patterns per language
- Allows `holoconf.register(holoconf_aws)` pattern from ADR-002

### Package Relationships

```
                    ┌─────────────────────┐
                    │   holoconf-core     │
                    │   (Rust crate)      │
                    └──────────┬──────────┘
                               │
           ┌───────────────────┼───────────────────┐
           │                   │                   │
           ▼                   ▼                   ▼
   ┌───────────────┐   ┌───────────────┐   ┌───────────────┐
   │ holoconf-aws  │   │holoconf-python│   │  holoconf-js  │
   │ (Rust crate)  │   │ (Rust crate)  │   │ (Rust crate)  │
   │ SSM, S3, CF   │   │ PyO3 bindings │   │ NAPI bindings │
   └───────┬───────┘   └───────┬───────┘   └───────┬───────┘
           │                   │                   │
           │           ┌───────┴───────┐           │
           │           ▼               ▼           │
           │   ┌─────────────┐ ┌─────────────┐     │
           │   │  holoconf   │ │  holoconf   │     │
           │   │  (PyPI)     │ │  (npm)      │◄────┘
           │   └──────┬──────┘ └─────────────┘
           │          │
           ▼          ▼
   ┌─────────────────────────┐
   │     holoconf-aws        │
   │  (PyPI) - re-exports    │
   │  Rust AWS resolvers     │
   └─────────────────────────┘
```

### Package Naming Convention

| Component | Rust Crate | PyPI Package | npm Package |
|-----------|------------|--------------|-------------|
| Core | `holoconf-core` | `holoconf` | `holoconf` |
| Python bindings | `holoconf-python` | (built into `holoconf`) | - |
| JS bindings | `holoconf-js` | - | (built into `holoconf`) |
| AWS resolvers | `holoconf-aws` | `holoconf-aws` | `@holoconf/aws` |
| GCP resolvers | `holoconf-gcp` | `holoconf-gcp` | `@holoconf/gcp` |
| Vault resolvers | `holoconf-vault` | `holoconf-vault` | `@holoconf/vault` |

### Cargo Workspace

```toml
# /Cargo.toml
[workspace]
members = [
    "crates/holoconf-core",
    "crates/holoconf-aws",
    "crates/holoconf-python",
    "crates/holoconf-js",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
repository = "https://github.com/org/holoconf"
```

### Python Package Structure

```
packages/python/holoconf/
├── pyproject.toml          # maturin build config
├── src/
│   └── holoconf/
│       ├── __init__.py     # Re-exports from Rust binding
│       └── py.typed        # PEP 561 marker
└── tests/
```

```toml
# pyproject.toml
[build-system]
requires = ["maturin>=1.0,<2.0"]
build-backend = "maturin"

[project]
name = "holoconf"
requires-python = ">=3.9"

[tool.maturin]
manifest-path = "../../crates/holoconf-python/Cargo.toml"
```

### JavaScript Package Structure

```
packages/javascript/holoconf/
├── package.json
├── index.js                # Re-exports from Rust binding
├── index.d.ts              # TypeScript definitions
└── tests/
```

## Rationale

- **Single repo** enables atomic changes across core + bindings
- **`crates/` directory** keeps Rust code together, managed by Cargo workspace
- **`packages/` directory** holds language-specific packaging (pyproject.toml, package.json)
- **Top-level conformance tests** ensure cross-language consistency
- **Resolver crates are separate** from core to keep binary size small
- **Language bindings are Rust crates** that compile to native extensions
- **Thin resolver wrappers** provide idiomatic installation while keeping logic in Rust
- **npm scoped packages** (`@holoconf/aws`) prevent naming conflicts

## Trade-offs Accepted

- **Larger repository** with all languages in exchange for **atomic cross-language changes**
- **More complex CI** (must test all languages) in exchange for **consistency guarantees**
- **Single version number** across packages in exchange for **simpler dependency management**
- **Thin wrapper packages** add indirection in exchange for **idiomatic installation patterns**

## Migration

N/A - This is the initial structure decision.

## Consequences

- **Positive:** Easy to make coordinated changes, single source of truth, shared CI/CD, conformance testing built-in
- **Negative:** Contributors need familiarity with monorepo tooling, larger clone size
- **Neutral:** Requires workspace-aware tooling (Cargo workspaces, potentially nx/turborepo for JS)

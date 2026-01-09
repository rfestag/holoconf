# ADR-017: Release Process

## Status

- **Proposed by:** Ryan on 2026-01-08
- **Accepted on:** 2026-01-08

## Context

Holoconf is a multi-language project with packages for:
- **Rust**: `holoconf-core` and `holoconf-cli` crates (crates.io)
- **Python**: `holoconf` package (PyPI)
- **Node.js**: Future npm package

We need a consistent, automated release process that:
1. Keeps all package versions synchronized
2. Updates the changelog appropriately
3. Publishes to all package registries
4. Creates GitHub releases with binaries

## Alternatives Considered

### Alternative 1: Manual Releases

- **Description**: Developer manually updates versions, changelog, tags, and publishes each package
- **Pros**: Simple, no tooling needed
- **Cons**: Error-prone, inconsistent, easy to forget steps, packages can get out of sync

### Alternative 2: cargo-release

- **Description**: Use `cargo-release` to automate Rust releases with hooks for other languages
- **Pros**: Well-maintained, handles Rust crates well
- **Cons**: Rust-centric, hooks for Python/npm are awkward, doesn't integrate with GitHub releases

### Alternative 3: release-please

- **Description**: Google's release automation tool that generates releases from conventional commits
- **Pros**: Fully automated, generates changelogs
- **Cons**: Requires conventional commit format, more complex setup, opinionated workflow

### Alternative 4: Makefile + Tag-Triggered CI

- **Description**: `make release` for local preparation, GitHub Actions for publishing
- **Pros**: Simple, predictable, works across all languages, no commit format requirements
- **Cons**: Requires manual changelog entries

## Decision

Use a two-part release process:
1. **Local preparation** via `make release VERSION=x.y.z`
2. **CI publishing** via tag-triggered GitHub Actions

## Design

### Pre-Release Check

Before releasing, run all checks without making changes:

```bash
make release-check
```

This runs:
1. **Working directory check** - Warns if uncommitted changes exist
2. **Branch check** - Warns if not on `main`
3. **Full test suite** - Rust, Python, and acceptance tests
4. **Semver compatibility** - Checks for breaking changes
5. **Changelog coverage** - Reviews changes that may need documentation

### Release Command

```bash
make release VERSION=0.2.0
```

This command:
1. **Pre-flight checks**:
   - Verifies no uncommitted changes
   - Verifies on `main` branch
   - Runs full test suite locally
2. **Updates versions** in:
   - `Cargo.toml` (workspace version)
   - `packages/python/holoconf/pyproject.toml`
3. **Updates CHANGELOG.md**:
   - Moves `[Unreleased]` section to `[VERSION] - DATE`
   - Adds new empty `[Unreleased]` section
4. **Creates commit and tag**:
   - Commit: `chore: release vX.Y.Z`
   - Tag: `vX.Y.Z`
5. **Prints next step**: `git push origin main --tags`

### CI Workflow (release.yml)

Triggered by pushing a `v*` tag:

```
validate ─────────────────────────────────────────────────────────┐
    │                                                              │
    ├──► build-python (parallel)                                   │
    │      ├─ x86_64-linux                                         │
    │      ├─ aarch64-linux                                        │
    │      ├─ x86_64-darwin                                        │
    │      ├─ aarch64-darwin                                       │
    │      └─ x86_64-windows                                       │
    │                                                              │
    ├──► build-cli (parallel)                                      │
    │      ├─ linux (x86_64, aarch64)                              │
    │      ├─ macos (x86_64, aarch64)                              │
    │      └─ windows (x86_64)                                     │
    │                                                              │
    └──► publish-crates ──► publish-pypi ──► github-release ◄──────┘
```

### Version Synchronization

All packages share the same version number, derived from the git tag:
- Tag: `v0.2.0`
- Cargo.toml: `version = "0.2.0"`
- pyproject.toml: `version = "0.2.0"`

### Changelog Format

Using [Keep a Changelog](https://keepachangelog.com/) format:

```markdown
## [Unreleased]

## [0.2.0] - 2026-01-15

### Added
- New feature X

### Fixed
- Bug Y
```

Changelog entries are added manually during development.

### Changelog Review Tool

Before releasing, run `make changelog-check` to review changes that may need changelog entries:

```bash
make changelog-check
```

This analyzes changes since the last release tag and reports:
- **Feature spec status changes** (e.g., Draft → Implemented)
- **New/modified acceptance tests**
- **New Rust unit tests** (functions with `#[test]`)
- **New Python unit tests** (functions matching `test_*`)
- **New feature specs and ADRs**

Example output:
```
══════════════════════════════════════════════════════════════════════
Changelog Review for Next Release
══════════════════════════════════════════════════════════════════════

Comparing against: v0.1.0

Current [Unreleased] section:
----------------------------------------
  - Added env resolver with default value support
  - Fixed schema validation for nested objects

Changes since v0.1.0 that may need changelog entries:

  Feature Specs:
    ! FEAT-002: Draft -> Implemented

  Acceptance Tests (3 new/modified):
    + tests/acceptance/resolvers/env_resolver.yaml
    + tests/acceptance/resolvers/env_defaults.yaml
    ~ tests/acceptance/schema/nested_validation.yaml

  Rust Unit Tests (2 new #[test]):
    + crates/holoconf-core/src/resolvers/env.rs (2 new)

══════════════════════════════════════════════════════════════════════
Review: Do the changelog entries above cover these changes?
══════════════════════════════════════════════════════════════════════
```

This is a review tool, not a gate—use your judgment to determine if changes warrant changelog entries.

### Required Secrets

| Secret | Purpose | Configuration |
|--------|---------|---------------|
| `CARGO_REGISTRY_TOKEN` | Publish to crates.io | GitHub repo secrets |
| PyPI OIDC | Publish to PyPI | PyPI trusted publishing (no secret needed) |
| `NPM_TOKEN` (future) | Publish to npm | GitHub repo secrets |

## Rationale

This approach was chosen because:

1. **Simplicity**: Two clear phases (local prep, CI publish) with no magic
2. **Multi-language support**: Works equally well for Rust, Python, and future npm packages
3. **No commit format requirements**: Developers write changelog entries naturally
4. **Safety**: Local tests must pass before tagging; CI tests run again before publishing
5. **Manual push**: Final checkpoint before irreversible publishing to registries

## Trade-offs Accepted

- **Manual changelog entries** in exchange for **no conventional commit requirement**
- **Two-step process** (make release + git push) in exchange for **final safety checkpoint**
- **sed-based version updates** in exchange for **no additional tooling dependencies**

## Migration

N/A - This is a new process for the initial release.

## Consequences

- **Positive:** Consistent releases across all package types
- **Positive:** Single command prepares everything locally
- **Positive:** CI handles all publishing automatically
- **Positive:** Changelog review tool helps catch undocumented changes
- **Negative:** Must remember to add changelog entries before releasing (mitigated by `make changelog-check`)
- **Neutral:** Version must be specified explicitly (no auto-increment)

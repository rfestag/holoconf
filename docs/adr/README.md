# Architecture Decision Records

This directory contains Architecture Decision Records (ADRs) for the HoloConf project.

## What is an ADR?

An ADR is a document that captures an important architectural decision made along with its context and consequences. ADRs help us:

- Document the reasoning behind decisions
- Provide context for future developers
- Track the evolution of the architecture
- Enable informed discussions about changes

## ADR Status

- **Proposed** - Under discussion, not yet accepted
- **Accepted** - Approved and in effect
- **Superseded** - Replaced by a newer ADR
- **Deprecated** - No longer relevant

## Architecture Decisions

<div class="searchable-table" data-page-size="10" markdown>

| ADR | Title | Status |
|-----|-------|--------|
| [ADR-001](ADR-001-multi-language-architecture.md) | Multi-Language Architecture | **Accepted** |
| [ADR-002](ADR-002-resolver-architecture.md) | Resolver Architecture | **Accepted** |
| [ADR-003](ADR-003-async-execution-model.md) | Async Execution Model | **Accepted** |
| [ADR-004](ADR-004-config-merging.md) | Config Merging Semantics | **Accepted** |
| [ADR-005](ADR-005-resolver-timing.md) | Resolver Timing (Lazy Resolution) | **Accepted** |
| [ADR-006](ADR-006-repository-package-structure.md) | Repository and Package Structure | **Accepted** |
| [ADR-007](ADR-007-schema-validation.md) | Schema and Validation | **Accepted** |
| [ADR-008](ADR-008-error-handling.md) | Error Handling Strategy | **Accepted** |
| [ADR-009](ADR-009-serialization-export.md) | Serialization and Export | **Accepted** |
| [ADR-010](ADR-010-thread-safety.md) | Thread Safety and Concurrency | **Accepted** |
| [ADR-011](ADR-011-interpolation-syntax.md) | Interpolation Syntax | **Accepted** |
| [ADR-012](ADR-012-type-coercion.md) | Type Coercion | **Accepted** |
| [ADR-013](ADR-013-testing-architecture.md) | Testing Architecture | **Accepted** |
| [ADR-014](ADR-014-code-quality-tooling.md) | Code Quality Tooling | **Accepted** |
| [ADR-015](ADR-015-documentation-site.md) | Documentation Site | **Accepted** |
| [ADR-016](ADR-016-pyo3-api-documentation.md) | PyO3 API Documentation | **Accepted** |
| [ADR-017](ADR-017-release-process.md) | Release Process | **Accepted** |
| [ADR-018](ADR-018-git-workflow.md) | Pull Request and Merge Process | **Accepted** |
| [ADR-019](ADR-019-resolver-extension-packages.md) | Resolver Extension Packages | **Accepted** |

</div>

## Creating a New ADR

1. Copy `template.md` to `ADR-NNN-short-title.md`
2. Fill in all sections
3. Submit for review
4. Update this index

## Template

See [template.md](template.md) for the ADR template.

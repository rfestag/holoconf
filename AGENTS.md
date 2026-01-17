# holoconf

Cross-language hierarchical configuration library. Rust core with Python and TypeScript bindings.

## Quick Reference

```bash
make check              # Full pre-commit validation (lint + security + test + audit)
make test               # Run all tests
make build              # Build Rust + Python
```

## Code Layout

- `crates/holoconf-core/` - Rust core library
- `crates/holoconf-cli/` - CLI tool
- `crates/holoconf-python/` - PyO3 bindings
- `crates/holoconf-node/` - NAPI-RS bindings (planned)
- `packages/python/holoconf/` - Python package with type stubs
- `packages/node/holoconf/` - Node.js package (planned)
- `tests/acceptance/` - YAML-driven acceptance tests
- `docs/adr/` - Architecture Decision Records
- `docs/specs/features/` - Feature specifications

## Specialized Agents

Use these agents for focused expertise. Claude will delegate automatically based on context.

| Agent | Purpose | When Used |
|-------|---------|-----------|
| `rust-expert` | Memory safety, performance, idiomatic Rust | Changes to `crates/` |
| `python-expert` | PyO3 patterns, type stubs, Pythonic APIs | Changes to Python bindings |
| `typescript-expert` | NAPI-RS patterns, TS types, Node.js APIs | Changes to Node bindings |
| `doc-writer` | Documentation style, narrative flow, examples | User-facing feature changes |
| `security-reviewer` | Vulnerability assessment, secrets detection | Security audits, PR reviews |
| `pr-reviewer` | Code quality, standards compliance | PR reviews |
| `acceptance-test-specialist` | Test design, YAML scenarios | Writing acceptance tests |

### Parallel Agent Patterns

When updating Rust core, spawn binding agents in parallel:

```
Main: Implement feature in Rust core
  → rust-expert: Review/implement core changes

Main: Update all bindings (in parallel)
  → python-expert: Update PyO3 bindings + type stubs
  → typescript-expert: Update NAPI-RS bindings + TS types

Main: Update documentation (for user-facing changes)
  → doc-writer: Update guides, API docs, ensure style consistency
```

### Pre-PR Validation

Always run `make check` before creating a PR. This runs:
- Linting (Rust + Python)
- Security checks (cargo audit, cargo deny, pip-audit)
- All tests (unit + acceptance)
- Unsafe code audit

## Rules

@.claude/rules/tdd-workflow.md
@.claude/rules/github-workflow.md
@.claude/rules/rust-patterns.md
@.claude/rules/python-patterns.md
@.claude/rules/typescript-patterns.md
@.claude/rules/documentation-style.md
@.claude/rules/build-test.md
@.claude/rules/acceptance-tests.md

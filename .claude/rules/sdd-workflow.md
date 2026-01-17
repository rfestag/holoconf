# Spec-Driven Development

**SDD wraps TDD: Specs define what to build, tests verify how it's built.**

> **Agents**: Use `acceptance-test-specialist` for test design, `rust-expert` or `python-expert` for implementation.

## Workflow Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    SPEC-DRIVEN DEVELOPMENT                  │
├─────────────────────────────────────────────────────────────┤
│  1. Spec Identification                                     │
│  2. Spec Updates (create/modify FEAT or ADR)                │
│     ┌─────────────────────────────────────────────────┐     │
│     │            TEST-DRIVEN DEVELOPMENT              │     │
│     │  3a. Write tests FIRST (acceptance + unit)      │     │
│     │  3b. Verify tests FAIL                          │     │
│     │  3c. Implement until tests PASS                 │     │
│     └─────────────────────────────────────────────────┘     │
│  4. Spec Validation (verify against FEAT requirements)      │
│  5. PR (with spec links)                                    │
└─────────────────────────────────────────────────────────────┘
```

## Phase 1: Spec Identification (BEFORE any code)

1. Search `docs/specs/features/` for relevant FEATs
2. Search `docs/adr/` for relevant architectural decisions
3. Consult Key Specs Reference in AGENTS.md for commonly-applicable ADRs
4. Document which specs apply to this change

## Phase 2: Spec Updates (BEFORE implementation)

**New feature** (no existing spec):
- Create `docs/specs/features/FEAT-NNN-name.md` from template
- Define requirements, API contract, test scenarios

**Enhancement** (existing spec covers it):
- Update existing FEAT with new/modified requirements
- Update "Last Updated" date

**Architectural change**:
- Create new ADR: `docs/adr/ADR-NNN-topic.md`
- Update old ADR Status to: "Superseded by ADR-NNN on [date]"
- Do NOT modify original ADR's Decision/Design sections

## Phase 3: TDD Implementation

1. Write acceptance tests FIRST: `tests/acceptance/`
2. Write unit tests FIRST
3. Run tests - confirm they FAIL
4. Implement until tests pass
5. Update type stubs: `packages/python/holoconf/src/holoconf/_holoconf.pyi`

## Phase 4: Spec Validation (BEFORE PR)

- [ ] All FEAT requirements implemented
- [ ] No ADR violations
- [ ] Tests cover FEAT test scenarios
- [ ] CHANGELOG.md updated
- [ ] Docs updated (if user-facing)

## Phase 5: PR

- Include links to relevant ADRs/FEATs in PR description
- Verify spec checkboxes are marked in PR template
- Run `make check`

## Modifying Existing Behavior

1. Find relevant FEAT spec
2. Update spec requirements FIRST
3. Find and update existing tests
4. Confirm tests fail with old behavior
5. Implement changes
6. Run full test suite

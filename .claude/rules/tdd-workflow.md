# Test-Driven Development

**Write tests FIRST, then implement until they pass.**

## New Features
1. Spec: `docs/specs/features/FEAT-xxx-name.md`
2. ADR if architectural: `docs/adr/ADR-xxx-topic.md`
3. Write acceptance tests FIRST: `tests/acceptance/`
4. Write unit tests FIRST
5. Run tests - confirm they FAIL
6. Implement until tests pass
7. Update type stubs: `packages/python/holoconf/src/holoconf/_holoconf.pyi`
8. Update CHANGELOG.md
9. Update docs (if user-facing)
10. Run `make check`

## Modifying Behavior
1. Find existing tests
2. Update tests FIRST
3. Confirm tests fail with old behavior
4. Implement changes
5. Run full test suite

---
description: Review a pull request
---

# Review PR: $ARGUMENTS

Analyze and review a pull request for code quality, correctness, and project standards.

## Steps

1. **Get PR information**:
   ```bash
   gh pr view $ARGUMENTS --json title,body,files,commits,author,baseRefName,headRefName
   ```

2. **Get the diff**:
   ```bash
   gh pr diff $ARGUMENTS
   ```

3. **Check CI status**:
   ```bash
   gh pr checks $ARGUMENTS
   ```

4. **Analyze changes by category**:
   - **Rust core** (`crates/`): Check error handling, thread safety, API consistency
   - **Python bindings** (`crates/holoconf-python/`, `packages/python/`): Check PyO3 patterns, type stubs
   - **Tests** (`tests/`): Check coverage, test quality
   - **Documentation** (`docs/`): Check accuracy, completeness

5. **Review checklist**:
   - [ ] Missing tests for new functionality?
   - [ ] Breaking API changes?
   - [ ] Security concerns (sensitive data handling, path traversal, etc.)?
   - [ ] Documentation updates needed?
   - [ ] CHANGELOG.md updated?
   - [ ] Type stubs updated (if Python API changed)?
   - [ ] Follows project patterns (ADRs, specs)?

6. **Summarize findings**:
   - List issues with specific `file:line` references
   - Categorize as: blocking, should-fix, nitpick
   - Highlight positive aspects too

## Output
Provide a structured review summary with:
- Overall assessment (approve, request changes, needs discussion)
- Specific feedback items
- Questions for the author (if any)

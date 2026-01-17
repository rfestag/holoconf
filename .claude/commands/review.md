---
description: Review a pull request
---

# Review PR: $ARGUMENTS

Analyze and review a pull request for code quality, correctness, and project standards.

## Agent Delegation

This review uses specialized agents for thorough analysis:
- **pr-reviewer**: Overall code review and quality assessment
- **rust-expert**: Deep Rust analysis (if `crates/` changed)
- **python-expert**: Python bindings review (if `packages/python/` changed)
- **security-reviewer**: Security assessment (run in parallel)

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

4. **Delegate to specialized agents based on changed files**:

   **If Rust files changed** (`crates/`):
   - Use `rust-expert` agent for memory safety, error handling, API design

   **If Python files changed** (`packages/python/`, `crates/holoconf-python/`):
   - Use `python-expert` agent for PyO3 patterns, type stubs, Pythonic design

   **Always**:
   - Use `security-reviewer` agent for vulnerability assessment
   - Use `pr-reviewer` agent for overall quality and standards

5. **Review checklist**:
   - [ ] Missing tests for new functionality?
   - [ ] Breaking API changes?
   - [ ] Security concerns (sensitive data handling, path traversal, etc.)?
   - [ ] Documentation updates needed?
   - [ ] CHANGELOG.md updated?
   - [ ] Type stubs updated (if Python API changed)?
   - [ ] Follows project patterns (ADRs, specs)?

6. **Summarize findings from all agents**:
   - Consolidate issues with specific `file:line` references
   - Categorize as: blocking, should-fix, nitpick
   - Highlight positive aspects too

## Output

Provide a structured review summary with:
- Overall assessment (approve, request changes, needs discussion)
- Findings organized by severity
- Security assessment summary
- Specific feedback items with file:line references
- Questions for the author (if any)

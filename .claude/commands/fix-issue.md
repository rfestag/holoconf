---
description: Create branch and start working on a GitHub issue
---

# Fix Issue: $ARGUMENTS

Create a branch and start working on a GitHub issue.

## Pre-checks

1. **Validate input**: Ensure `$ARGUMENTS` is a valid issue number
2. **Check for uncommitted changes**:
   ```bash
   git status --short
   ```
   If changes exist, ask user to commit or stash first
3. **Ensure on main branch**:
   ```bash
   git branch --show-current
   ```
   If not on main, ask user to switch to main or suggest running `/worktree` instead

## Steps

1. **Fetch issue details**:
   ```bash
   gh issue view $ARGUMENTS --json title,body,labels,comments
   ```

2. **Determine branch type from labels**:
   - `bug` label → `fix/$ARGUMENTS-<short-description>`
   - `enhancement` label → `feat/$ARGUMENTS-<short-description>`
   - Otherwise → `chore/$ARGUMENTS-<short-description>`

3. **Create and switch to branch**:
   ```bash
   git fetch origin main
   git checkout -b $BRANCH_NAME origin/main
   ```

4. **Analyze the issue**:
   - Understand the problem from description and comments
   - Search codebase for relevant files
   - Determine if this needs:
     - Spec update (`docs/specs/features/`)
     - ADR (`docs/adr/`)
     - New acceptance tests

5. **Follow TDD workflow**:
   - Write failing test first
   - Implement fix
   - Verify tests pass
   - Update CHANGELOG.md

## Output
- Report branch created: `$BRANCH_NAME`
- Show first steps from issue analysis
- Suggest: "Run `/pr` when ready to create a pull request"
- **Optional**: "Run `/worktree` to work on this in a separate directory"

## Edge Cases
- **Issue not found**: Report error and suggest checking issue number
- **Branch already exists**: Offer to switch to it with `git checkout $BRANCH_NAME`
- **Not on main**: Suggest switching to main first or using `/worktree` for parallel work

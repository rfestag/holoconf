---
description: Create branch and start working on a GitHub issue
---

# Fix Issue: $ARGUMENTS

Create a worktree/branch and start working on a GitHub issue.

## Pre-checks

1. **Validate input**: Ensure `$ARGUMENTS` is a valid issue number
2. **Check for uncommitted changes**:
   ```bash
   git status --short
   ```
   If changes exist, ask user to commit or stash first

## Steps

1. **Fetch issue details**:
   ```bash
   gh issue view $ARGUMENTS --json title,body,labels,comments
   ```

2. **Determine branch type from labels**:
   - `bug` label → `fix/$ARGUMENTS-<short-description>`
   - `enhancement` label → `feat/$ARGUMENTS-<short-description>`
   - Otherwise → `chore/$ARGUMENTS-<short-description>`

3. **Create worktree and branch**:
   ```bash
   git fetch origin main
   git worktree add /home/ryan/Code/holoconf-$BRANCH_NAME -b $BRANCH_NAME origin/main
   cd /home/ryan/Code/holoconf-$BRANCH_NAME
   ```

4. **Setup development environment** (if needed):
   ```bash
   make install-tools
   ```

5. **Analyze the issue**:
   - Understand the problem from description and comments
   - Search codebase for relevant files
   - Determine if this needs:
     - Spec update (`docs/specs/features/`)
     - ADR (`docs/adr/`)
     - New acceptance tests

6. **Follow TDD workflow**:
   - Write failing test first
   - Implement fix
   - Verify tests pass
   - Update CHANGELOG.md

## Output
- Report worktree location: `/home/ryan/Code/holoconf-$BRANCH_NAME/`
- Suggest: "Open in new VS Code window (SCM view > Worktrees > Open in New Window)"
- Suggest: "Run `/pr` when ready to create a pull request"

## Edge Cases
- **Issue not found**: Report error and suggest checking issue number
- **Branch already exists**: Check for existing worktree with `git worktree list` and offer to switch
- **Worktree path exists**: Suggest removing stale worktree or using different name

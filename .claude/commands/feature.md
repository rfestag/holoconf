---
description: Create a new feature branch with git worktree for parallel development
---

# Start New Feature: $ARGUMENTS

Create a git worktree for parallel feature development.

## Steps

1. **Validate**: Ensure feature name is provided. If `$ARGUMENTS` is empty, ask the user for a name.

2. **Check state**:
   - Run `git status --short` to check for uncommitted changes
   - If changes exist, ask user to commit or stash first

3. **Setup**:
   ```bash
   git fetch origin main
   git branch feature/$ARGUMENTS origin/main
   git worktree add ../holoconf-$ARGUMENTS feature/$ARGUMENTS
   ```

4. **Report**: Tell user the worktree is at `/home/ryan/Code/holoconf-$ARGUMENTS/` and that you will now work there.

## Notes
- If branch already exists, check for existing worktree with `git worktree list` and offer to switch
- All future work for this feature happens in the worktree directory
- User should open the worktree in a new VS Code window (SCM view > Worktrees > Open in New Window)
- Run `/feature-done` when ready to create a PR

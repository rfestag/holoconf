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

3. **Create worktree**:
   ```bash
   git fetch origin main
   git branch feature/$ARGUMENTS origin/main
   git worktree add ../holoconf-$ARGUMENTS feature/$ARGUMENTS
   ```

4. **Setup development environment**:
   ```bash
   cd ../holoconf-$ARGUMENTS
   make install-tools
   ```

5. **Report**: Tell user:
   - The worktree is ready at `/home/ryan/Code/holoconf-$ARGUMENTS/`
   - Run `make check` to verify everything works
   - All future work for this feature happens in the worktree directory

## Notes
- If branch already exists, check for existing worktree with `git worktree list` and offer to switch
- User should open the worktree in a new VS Code window (SCM view > Worktrees > Open in New Window)
- Run `/feature-done` when ready to create a PR

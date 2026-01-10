---
description: Complete the current feature - squash commits, push, and create PR
---

# Complete Current Feature

Finalize the feature with a single commit and create a pull request.

## Pre-checks

1. Run `git branch --show-current` to verify current branch is `feature/*` (abort if on `main`)
2. Check `git status` for uncommitted changes

## Steps

1. **Handle uncommitted changes**: If present, ask user whether to include them in the commit or stash them

2. **Squash commits** (if multiple):
   ```bash
   # Count commits ahead of main
   git rev-list --count origin/main..HEAD

   # If > 1, show what will be squashed:
   git log origin/main..HEAD --oneline

   # Squash into single commit:
   git reset --soft origin/main
   git commit -m "<type>: <summary message>"
   ```

   Use conventional commit format: `feat:`, `fix:`, `docs:`, `refactor:`, etc.

3. **Push**:
   ```bash
   git push -u origin <branch-name>
   ```

4. **Create PR**:
   ```bash
   gh pr create --base main --title "<type>: <summary>" --body "<description>"
   ```

5. **Cleanup** (optional): Ask if user wants to remove the worktree:
   ```bash
   cd /home/ryan/Code/holoconf
   git worktree remove ../holoconf-<feature>
   ```

## Output
Report the PR URL and whether cleanup was performed.

## Edge Cases
- **Not on feature branch**: List available feature branches and abort
- **No commits ahead of main**: Inform user there's nothing to commit
- **Push rejected**: Offer to force push with `--force-with-lease` (with warning)

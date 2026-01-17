---
description: Create a git worktree for parallel work on a branch
---

# Create Worktree: $ARGUMENTS

Create a git worktree to work on a branch in a separate directory (for parallel work).

## Usage

- `/worktree` - Create worktree for current branch
- `/worktree <branch-name>` - Create worktree for specified branch
- `/worktree <issue-number>` - Create worktree and new branch for issue

## Pre-checks

1. **Check git repository**:
   ```bash
   git rev-parse --is-inside-work-tree
   ```

2. **Validate worktree doesn't already exist**:
   ```bash
   git worktree list
   ```

## Steps

### If $ARGUMENTS is empty (use current branch)

1. **Get current branch**:
   ```bash
   BRANCH=$(git branch --show-current)
   ```

2. **Create worktree**:
   ```bash
   git worktree add /home/ryan/Code/holoconf-$BRANCH $BRANCH
   ```

### If $ARGUMENTS is a branch name

1. **Verify branch exists**:
   ```bash
   git rev-parse --verify $ARGUMENTS
   ```

2. **Create worktree**:
   ```bash
   git worktree add /home/ryan/Code/holoconf-$ARGUMENTS $ARGUMENTS
   ```

### If $ARGUMENTS is an issue number

1. **Fetch issue and determine branch name** (same logic as `/fix-issue`):
   ```bash
   gh issue view $ARGUMENTS --json title,labels
   ```

2. **Create new branch in worktree**:
   ```bash
   git fetch origin main
   git worktree add /home/ryan/Code/holoconf-$BRANCH_NAME -b $BRANCH_NAME origin/main
   ```

## Output

- Report worktree location: `/home/ryan/Code/holoconf-$BRANCH_NAME/`
- Suggest: "Open in new VS Code window: `code /home/ryan/Code/holoconf-$BRANCH_NAME`"
- **Cleanup reminder**: "Remove worktree when done: `git worktree remove <path>`"

## Edge Cases

- **Worktree already exists**: Report existing worktree path
- **Path exists but not a worktree**: Suggest using different name or removing directory
- **Branch doesn't exist**: For branch name argument, create it first
- **Not in git repo**: Report error

## Cleanup

When done with the worktree:

```bash
# From any directory
git worktree remove /home/ryan/Code/holoconf-$BRANCH_NAME

# Or prune all removed worktrees
git worktree prune
```

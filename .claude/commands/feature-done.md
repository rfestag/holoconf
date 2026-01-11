---
description: Complete the current feature - push and create PR
---

# Complete Current Feature

Push the feature branch and create a pull request. GitHub will squash commits at merge time (see ADR-018).

## Pre-checks

1. Run `git branch --show-current` to verify not on `main` (abort if on `main`)
2. Check `git status` for uncommitted changes
3. Check if in a worktree: `git rev-parse --git-common-dir` differs from `git rev-parse --git-dir`

## Steps

1. **Handle uncommitted changes**: If present, ask user whether to commit them or stash them

2. **Run checks**:
   ```bash
   make check
   ```
   If checks fail, help user fix the issues before proceeding.

3. **Push**:
   ```bash
   git push -u origin <branch-name>
   ```

4. **Analyze changes**:
   - Run `git log main..HEAD --oneline` to see commits
   - Run `git diff main --stat` to see which files/directories changed
   - Determine PR type from changes: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`

5. **Read the PR template**:
   ```bash
   cat .github/PULL_REQUEST_TEMPLATE.md
   ```

6. **Fill in the PR template** based on your analysis:
   - Parse each section of the template
   - For "Summary": Write 1-3 sentences derived from commit messages
   - For "Related Issue": Link if mentioned in commits, otherwise leave blank
   - For "Spec / ADR": Link if changes implement a spec or ADR
   - For checkbox sections: Mark `[x]` for items that apply based on files changed
   - Remove HTML comments from the filled template

7. **Create PR**:
   ```bash
   gh pr create --base main --title "<type>: <summary>" --body "<filled-template>"
   ```
   Use conventional commit format for title: `feat:`, `fix:`, `docs:`, `refactor:`, etc.

8. **Cleanup** (only if in a worktree): Ask if user wants to remove the worktree:
   ```bash
   cd /home/ryan/Code/holoconf
   git worktree remove <current-worktree-path>
   ```

## Output
Report the PR URL and whether cleanup was performed.

## Edge Cases
- **On main branch**: Abort and inform user to create a feature branch first
- **No commits ahead of main**: Inform user there's nothing to push
- **Push rejected**: May need to force push with `--force-with-lease` if branch was rebased (with warning)
- **No PR template**: Create a simple PR with just a summary

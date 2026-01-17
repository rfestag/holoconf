---
description: Push branch and create pull request
---

# Create Pull Request

Push the current branch and create a pull request following the project's PR template.

## Pre-checks

1. **Verify not on main**:
   ```bash
   git branch --show-current
   ```
   Abort if on `main` - inform user to create a feature branch first

2. **Check for uncommitted changes**:
   ```bash
   git status --short
   ```
   If changes exist, ask whether to commit or stash them

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
   - For "Related Issue": Link if mentioned in commits (Fixes #N), otherwise leave blank
   - For "Spec / ADR":
     - **Required for new features**: Link to FEAT spec in `docs/specs/features/`
     - **Required for architectural changes**: Link to ADR in `docs/adr/`
     - For bug fixes or small changes: Write "N/A - bug fix" or similar
     - Mark the spec compliance checkboxes
   - For checkbox sections: Mark `[x]` for items that apply based on files changed
   - Remove HTML comments from the filled template

7. **Create PR**:
   ```bash
   gh pr create --base main --title "<type>: <summary>" --body "<filled-template>"
   ```
   Use conventional commit format for title: `feat:`, `fix:`, `docs:`, `refactor:`, etc.

## Output
Report the PR URL.

## Edge Cases
- **On main branch**: Abort and inform user to create a feature branch first
- **No commits ahead of main**: Inform user there's nothing to push
- **Push rejected**: May need to force push with `--force-with-lease` if branch was rebased (with warning)
- **No PR template**: Create a simple PR with just a summary

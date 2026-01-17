---
description: Create a major release (breaking changes)
---

# Create Major Release

Create a major version release (X.Y.Z → X+1.0.0) for breaking changes.

## Pre-checks

1. **Verify on main branch**:
   ```bash
   git branch --show-current
   ```
   Must be on `main`

2. **Check for uncommitted changes**:
   ```bash
   git status --short
   ```
   Must be clean

3. **Verify up to date**:
   ```bash
   git fetch origin main
   git status
   ```

## Auto-Detection Logic

Major release is appropriate when `[Unreleased]` in CHANGELOG.md contains:
- Breaking API changes
- Removed features
- Incompatible behavior changes
- Any entry marked with "BREAKING:" prefix

## Steps

1. **Get current version**:
   ```bash
   grep '^version' Cargo.toml | head -1
   ```

2. **Calculate new version**: `X.Y.Z` → `(X+1).0.0`

3. **Parse CHANGELOG.md** to identify breaking changes

4. **Show what will be released**:
   - List all changes from `[Unreleased]` section
   - **Highlight breaking changes prominently**
   - Confirm with user: "This is a MAJOR version bump. Are you sure?"

5. **Run release**:
   ```bash
   make release VERSION=<new-version>
   ```

6. **Create GitHub release** (optional):
   ```bash
   gh release create v<new-version> --title "v<new-version>" --notes "<changelog-section>"
   ```

## Output
- Report the new version
- Provide link to GitHub release
- Remind about:
  - Migration guide (if needed)
  - Announcing breaking changes to users
  - Updating dependent projects

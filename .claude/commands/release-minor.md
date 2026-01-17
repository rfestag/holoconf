---
description: Create a minor release (new features, backward compatible)
---

# Create Minor Release

Create a minor version release (X.Y.Z → X.Y+1.0) for new features and enhancements.

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

Minor release is appropriate when `[Unreleased]` in CHANGELOG.md contains:
- New features (entries under `### Added`)
- Non-breaking enhancements (entries under `### Changed`)
- Deprecations (but NOT removals)

**Warning**: If there are breaking changes, suggest `/release-major` instead.

## Steps

1. **Get current version**:
   ```bash
   grep '^version' Cargo.toml | head -1
   ```

2. **Calculate new version**: `X.Y.Z` → `X.(Y+1).0`

3. **Parse CHANGELOG.md** to verify minor-appropriate changes

4. **Show what will be released**:
   - List all changes from `[Unreleased]` section
   - Highlight new features
   - Confirm with user

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
- Remind about post-release tasks (if any)

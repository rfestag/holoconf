---
description: Create a patch release (bug fixes only)
---

# Create Patch Release

Create a patch version release (X.Y.Z → X.Y.Z+1) for bug fixes and minor improvements.

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

Patch release is appropriate when `[Unreleased]` in CHANGELOG.md contains ONLY:
- Bug fixes (entries under `### Fixed`)
- Documentation updates (entries under `### Documentation`)
- Internal refactors with no API changes

**Warning**: If there are entries under `### Added` or `### Changed`, suggest `/release-minor` instead.

## Steps

1. **Get current version**:
   ```bash
   grep '^version' Cargo.toml | head -1
   ```

2. **Calculate new version**: `X.Y.Z` → `X.Y.(Z+1)`

3. **Parse CHANGELOG.md** to verify patch-appropriate changes

4. **Show what will be released**:
   - List changes from `[Unreleased]` section
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

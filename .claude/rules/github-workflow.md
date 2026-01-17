# GitHub Workflow

## Creating Issues

**IMPORTANT:** Always use the correct issue template based on type.

### Bug Reports
Use `gh issue create` with these fields (from `.github/ISSUE_TEMPLATE/1-bug.yml`):
- Title prefix: `[Bug]: `
- Labels: `bug`, `triage`
- Required fields: Affected Package, Bug Description, Minimal Reproduction, Version Information

### Feature Requests
Use `gh issue create` with these fields (from `.github/ISSUE_TEMPLATE/2-feature.yml`):
- Title prefix: `[Feature]: `
- Labels: `enhancement`, `triage`
- Required fields: Target Package(s), Feature Type, Problem Statement

## Creating Pull Requests

**IMPORTANT:** Always read and follow `.github/PULL_REQUEST_TEMPLATE.md`.

Before creating a PR:
1. Read the template: `cat .github/PULL_REQUEST_TEMPLATE.md`
2. Fill in ALL sections:
   - Summary (brief description)
   - Related Issue (Fixes #N or Relates to #N)
   - Spec/ADR links (if applicable)
   - Changes checkboxes (mark what changed)
   - Checklist (mark completed items)
3. Remove HTML comments from filled template

Use: `gh pr create --base main --title "<type>: <summary>" --body "<filled-template>"`

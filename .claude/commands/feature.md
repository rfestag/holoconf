---
description: Create a new feature request issue
---

# Create Feature Issue: $ARGUMENTS

Create a GitHub issue for a new feature request using the project's feature template.

## Steps

1. **Validate input**: If `$ARGUMENTS` is empty, ask user for feature description

2. **Read the issue template**:
   ```bash
   cat .github/ISSUE_TEMPLATE/2-feature.yml
   ```

3. **Gather required fields**:
   - **Target Package(s)**: Ask or infer from description (holoconf-core, holoconf-cli, holoconf-python, Documentation)
   - **Feature Type**: New Feature / Enhancement to existing feature / Breaking change
   - **Problem Statement**: Derive from `$ARGUMENTS`
   - **Proposed Solution**: Optional, ask user or leave for later
   - **Alternatives Considered**: Optional

4. **Create issue**:
   ```bash
   gh issue create \
     --title "[Feature]: <concise summary>" \
     --label "enhancement,triage" \
     --body "<filled-template>"
   ```

5. **Report**: Show issue number and URL

## Notes
- Use conventional commit style in title: descriptive but concise
- Include any relevant spec references (FEAT-xxx) in the body
- If the feature is large, suggest breaking it into sub-issues

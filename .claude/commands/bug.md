---
description: Create a new bug report issue
---

# Create Bug Issue: $ARGUMENTS

Create a GitHub issue for a bug report using the project's bug template.

## Steps

1. **Validate input**: If `$ARGUMENTS` is empty, ask user for bug description

2. **Read the issue template**:
   ```bash
   cat .github/ISSUE_TEMPLATE/1-bug.yml
   ```

3. **Gather required fields**:
   - **Affected Package**: Ask or infer (holoconf-core, holoconf-cli, holoconf (Python), Documentation)
   - **Bug Description**: Derive from `$ARGUMENTS`
   - **Minimal Reproduction**: Ask user or note "To be determined"
   - **Version Information**:
     ```bash
     cargo metadata --format-version 1 | jq '.packages[] | select(.name == "holoconf-core") | .version'
     ```

4. **Create issue**:
   ```bash
   gh issue create \
     --title "[Bug]: <concise summary>" \
     --label "bug,triage" \
     --body "<filled-template>"
   ```

5. **Report**: Show issue number and URL

## Notes
- Ensure bug title clearly describes the symptom
- Include error messages if available
- Reference related issues if this might be a regression

---
description: Update or review documentation
---

# Documentation: $ARGUMENTS

Update, review, or create documentation using the `doc-writer` agent.

## Usage

- `/docs` - Review all documentation for consistency
- `/docs guide/interpolation` - Update a specific guide
- `/docs api/python` - Update API reference
- `/docs adr new` - Create a new ADR
- `/docs feature FEAT-xxx` - Update a feature spec

## Agent Delegation

Delegate this task to the `doc-writer` agent which specializes in:
- Rust Book-style narrative for user guides
- Formal style for ADRs and feature specs
- Multi-language code examples (Python, Rust, CLI)
- Cross-referencing and navigation

## Steps

1. **Determine scope from $ARGUMENTS**:
   - If empty: Review all docs for style consistency
   - If path specified: Focus on that documentation area
   - If "adr new" or "feature new": Create from template

2. **For updates**:
   - Read existing documentation
   - Apply appropriate style (Rust Book for guides, formal for ADRs/specs)
   - Ensure multi-language examples are present and correct
   - Add cross-references to related documentation
   - Update navigation in `mkdocs.yml` if needed

3. **For new documentation**:
   - Read the appropriate template:
     - ADR: `docs/adr/template.md`
     - Feature spec: `docs/specs/features/template.md`
   - Create new file following template structure
   - Add to `mkdocs.yml` navigation

4. **Validate**:
   ```bash
   make docs-build
   ```

5. **If implementing a feature**, ensure:
   - User guide covers the feature
   - API reference documents all public methods
   - Examples are correct and runnable

## Style Reference

| Doc Type | Style | Voice |
|----------|-------|-------|
| `docs/guide/` | Rust Book | "You" - friendly, progressive |
| `docs/api/` | Rust Book | Clear, example-driven |
| `docs/adr/` | Formal | Third-person, decision-focused |
| `docs/specs/features/` | Formal | Third-person, requirements-focused |

## Output

Report:
- Files created/modified
- Style applied
- Cross-references added
- Build status (`make docs-build`)

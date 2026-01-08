# ADR-015: Cross-Language Documentation Site

## Status

Accepted

## Context

holoconf is a multi-language configuration library with a Rust core and bindings for Python (with JS, Go, Java, C planned). We need a documentation site that:

1. Provides consistent documentation across all supported languages
2. Allows users to select their language once and see relevant examples site-wide
3. Supports versioned documentation
4. Publishes to GitHub Pages
5. Includes existing ADRs and feature specs for contributors

## Decision

Use **MkDocs with Material theme** for the documentation site.

### Rationale

- **Persistent language tabs**: Material's `content.tabs.link` feature syncs tab selection across all pages via localStorage - users click "Python" once and all code examples show Python
- **Clean syntax**: Pure Markdown (`===` tabs) without JSX/MDX
- **Versioning**: mike plugin is mature and battle-tested
- **Python ecosystem alignment**: Natural fit given Python bindings and tooling
- **GitHub Pages**: Simple deployment via `mkdocs gh-deploy` or GitHub Actions

## Alternatives Considered

### Docusaurus

**Pros:**
- Built-in versioning (best-in-class)
- Large ecosystem and community
- React-based for JS-heavy teams

**Cons:**
- Requires MDX/JSX for tabs
- React knowledge helpful for customization
- Heavier build times

### Starlight (Astro)

**Pros:**
- Fastest builds
- Modern DX

**Cons:**
- Immature versioning support
- Limited monorepo support

### VitePress

**Pros:**
- Clean markdown syntax
- Fast builds (Vite-powered)

**Cons:**
- Versioning not built-in
- Vue knowledge helpful

## Consequences

### Positive

- Documentation is consistent across languages with persistent language selection
- Clean markdown-based authoring experience
- Existing ADRs and specs are integrated into the site
- GitHub Actions automates deployment

### Negative

- Requires mike plugin for versioning (not built-in)
- Python/mkdocs dependency for local development
- No auto-generation of API docs from Rust (must maintain manually)

## Implementation

### Directory Structure

```
holoconf/
├── mkdocs.yml              # Site configuration (project root)
├── docs/
│   ├── index.md            # Landing page
│   ├── guide/              # User guides with language tabs
│   ├── api/                # API reference
│   ├── contributing/       # Contributor docs
│   ├── adr/                # Architecture decisions (existing)
│   ├── specs/              # Feature specs (existing)
│   └── changelog.md
└── site/                   # Built output (gitignored)
```

### Key Configuration

```yaml
theme:
  name: material
  features:
    - content.tabs.link      # Persistent language selection
    - content.code.copy
    - navigation.instant
```

### Language Tab Convention

All code examples use consistent labels:
- "Python"
- "Rust"
- "CLI"
- "JavaScript" (future)
- "Go" (future)

### Deployment

GitHub Actions workflow deploys on push to main:
1. Installs mkdocs-material
2. Builds with `mkdocs build --strict`
3. Deploys to GitHub Pages

## References

- [MkDocs Material - Content Tabs](https://squidfunk.github.io/mkdocs-material/reference/content-tabs/)
- [mike - MkDocs versioning](https://github.com/jimporter/mike)
- [Docusaurus Tabs](https://docusaurus.io/docs/markdown-features/tabs)

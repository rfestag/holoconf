# Documentation Style

> **Agent**: For documentation updates and style consistency, use the `doc-writer` agent.

## Two Styles

### Rust Book Style (User-Facing Docs)

Used in: `docs/guide/`, `docs/api/`

- Address reader as "you"
- Friendly, progressive complexity
- Start with "why" before "how"
- Cross-reference related guides

### Formal Style (Design Docs)

Used in: `docs/adr/`, `docs/specs/features/`

- Third-person voice
- Follow templates strictly
- Precise, unambiguous language

## Multi-Language Examples

ALWAYS include Python, Rust, and CLI examples using tabs:

```markdown
=== "Python"
    ```python
    config = Config.load("config.yaml")
    ```

=== "Rust"
    ```rust
    let config = Config::load("config.yaml")?;
    ```

=== "CLI"
    ```bash
    holoconf get config.yaml key
    ```
```

## Validation

```bash
make docs-build  # Must pass before PR
```

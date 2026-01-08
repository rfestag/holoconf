# ADR-016: PyO3 Extension API Documentation

## Status

- **Proposed by:** Claude on 2026-01-08
- **Accepted on:** 2026-01-08

## Context

The holoconf Python package uses PyO3 to expose Rust functionality to Python. This creates a challenge for API documentation:

1. The compiled extension module (`.so` file) cannot be introspected by standard Python documentation tools
2. Docstrings defined in Rust code via PyO3's `#[pyo3(text_signature = "...")]` and `///` doc comments are available at runtime
3. mkdocstrings/griffe cannot resolve type aliases from compiled extensions
4. Users expect navigable API documentation like other Python packages

We need a solution that provides:
- Auto-generated API documentation (not manually maintained)
- Proper type annotations visible in docs
- Docstrings from the actual implementation
- Standard Python documentation structure (classes, methods, parameters)

## Alternatives Considered

### Alternative 1: Manual API Documentation (Tables)

Write API documentation entirely by hand using markdown tables.

- **Pros:** Full control over presentation, no tooling complexity
- **Cons:** Documentation drifts from code, high maintenance burden, no validation

### Alternative 2: Sphinx with autodoc

Use Sphinx instead of MkDocs with autodoc for Python documentation.

- **Pros:** More mature, widely used
- **Cons:** Different tech stack from MkDocs, same griffe/introspection issues with `.so` files

### Alternative 3: Runtime Docstring Extraction

Build a custom script to extract docstrings at runtime and generate markdown.

- **Pros:** Uses actual runtime docstrings
- **Cons:** Complex to maintain, still need type information

### Alternative 4: Python Stub Files (.pyi) with mkdocstrings

Create `.pyi` stub files that griffe can parse, containing type annotations and docstrings.

- **Pros:** Standard Python pattern for type hints, works with mkdocstrings, single source of truth for types
- **Cons:** Must keep stubs in sync with Rust implementation

## Decision

Adopt **Alternative 4**: Use Python stub files (`.pyi`) with mkdocstrings.

## Design

### Directory Structure

```
packages/python/holoconf/src/holoconf/
├── __init__.py              # Re-exports from _holoconf
├── _holoconf.cpython-*.so   # Compiled extension (generated)
└── _holoconf.pyi            # Type stubs with docstrings
```

### Stub File Format

The `.pyi` file contains:

1. **Class definitions** with docstrings
2. **Method signatures** with full type annotations
3. **Docstrings** in Google style format
4. **Exception classes** with inheritance

Example:

```python
class Config:
    """Configuration object for loading and accessing configuration values.

    Example:
        >>> config = Config.load("config.yaml")
        >>> host = config.get("database.host")
    """

    @staticmethod
    def load(path: str, allow_http: bool = False) -> "Config":
        """Load configuration from a YAML file.

        Args:
            path: Path to the YAML file
            allow_http: Enable HTTP resolver (disabled by default)

        Returns:
            A new Config object

        Raises:
            ParseError: If the file cannot be parsed
        """
        ...
```

### mkdocs.yml Configuration

```yaml
plugins:
  - search
  - mkdocstrings:
      default_handler: python
      handlers:
        python:
          paths:
            - packages/python/holoconf/src
          options:
            show_source: false
            show_bases: true
            heading_level: 2
            members_order: source
            docstring_style: google
            docstring_section_style: spacy
            show_signature_annotations: true
            separate_signature: true
```

### Documentation Structure

The API documentation uses a categorized navigation structure similar to AWS CDK docs:

```
docs/api/python/
├── index.md                    # Package overview with quick start
├── classes/
│   ├── config.md              # Config class (auto-generated)
│   └── schema.md              # Schema class (auto-generated)
└── exceptions/
    ├── holoconf-error.md      # Base exception
    ├── parse-error.md
    ├── validation-error.md
    ├── resolver-error.md
    ├── path-not-found-error.md
    ├── circular-reference-error.md
    └── type-coercion-error.md
```

Each class page uses a simple mkdocstrings directive:

```markdown
# Config

::: holoconf.Config
    options:
      show_root_heading: false
      members_order: source
      group_by_category: true
      show_category_heading: true
```

Exception pages include contextual documentation around the mkdocstrings directive, since PyO3 exception docstrings are typically brief:

```markdown
# ParseError

Raised when YAML or JSON content cannot be parsed due to syntax errors.

## When It's Raised

- Invalid YAML syntax (missing colons, bad indentation, etc.)
- Invalid JSON syntax (missing quotes, trailing commas, etc.)
- Encoding errors in the configuration file

## Example

\`\`\`python
from holoconf import Config, ParseError

try:
    config = Config.loads("invalid: yaml: content")
except ParseError as e:
    print(f"Parse error: {e}")
\`\`\`

## Class Reference

::: holoconf.ParseError
    options:
      show_root_heading: false
```

This pattern provides richer documentation while still auto-generating the class reference.

The navigation in `mkdocs.yml` defines the categorized structure, using package names as top-level identifiers:

```yaml
- API Reference:
    - holoconf (Python):
        - Overview: api/python/index.md
        - Classes:
            - Config: api/python/classes/config.md
            - Schema: api/python/classes/schema.md
        - Exceptions:
            - HoloconfError: api/python/exceptions/holoconf-error.md
            # ... other exceptions
    - holoconf-core (Rust):
        - Overview: api/rust/index.md
        - Structs:
            - Config: api/rust/structs/config.md
            # ... other structs
        - Enums:
            - Value: api/rust/enums/value.md
            # ... other enums
    - holoconf-cli:
        - Overview: api/cli/index.md
```

This pattern is applied consistently across all language bindings.

### Keeping Stubs in Sync

The stub files must be kept in sync with the Rust implementation:

1. **Docstrings** - Copy from PyO3 doc comments in `crates/holoconf-python/src/lib.rs`
2. **Signatures** - Match `#[pyo3(signature = ...)]` annotations
3. **Types** - Use Python equivalents of Rust types

When updating the Rust implementation:
1. Update `lib.rs` with new methods/changes
2. Update `_holoconf.pyi` with corresponding changes
3. Run `make docs` to verify

## Rationale

1. **Standard Python Pattern** - `.pyi` files are the standard way to add type information to native extensions

2. **Works with Existing Tools** - mkdocstrings/griffe can parse `.pyi` files without special handling

3. **Type Checker Support** - The same stub files work with mypy, pyright, and IDE autocompletion

4. **Single Source of Truth** - While stubs duplicate Rust docs, they serve multiple purposes (docs + type checking)

5. **Minimal Tooling** - No custom scripts or build steps; stubs are just Python files

## Trade-offs Accepted

- **Manual Sync Required** - Must update stubs when Rust code changes
- **Potential Drift** - Stubs could become out of sync (mitigated by code review)
- **Duplicate Documentation** - Docstrings exist in both Rust and Python stubs

## Consequences

- **Positive:**
  - Professional API documentation with auto-generated method reference
  - Type hints available for IDE autocompletion and type checkers
  - Standard documentation structure (classes, methods, parameters, returns)
  - Works with existing MkDocs infrastructure

- **Negative:**
  - Additional file to maintain (`_holoconf.pyi`)
  - Must remember to update stubs when changing Rust API

- **Neutral:**
  - Documentation build process unchanged (still `make docs`)
  - Same tooling (MkDocs + Material theme)

## References

- [PEP 484 - Type Hints](https://peps.python.org/pep-0484/)
- [PEP 561 - Distributing and Packaging Type Information](https://peps.python.org/pep-0561/)
- [mkdocstrings documentation](https://mkdocstrings.github.io/)
- [PyO3 documentation](https://pyo3.rs/)

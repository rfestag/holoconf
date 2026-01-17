# Python Patterns

> **Agent**: For Python and PyO3 analysis, use the `python-expert` agent.

## Development Setup
```bash
cd packages/python/holoconf
source .venv/bin/activate && maturin develop
```

## Type Stubs
After changing Python API, update: `packages/python/holoconf/src/holoconf/_holoconf.pyi`

## Linting & Formatting (ruff)

This project uses `ruff` which replaces black, isort, flake8, and pyupgrade.

```bash
.venv/bin/ruff check src/ tests/       # Lint
.venv/bin/ruff check src/ tests/ --fix # Auto-fix
.venv/bin/ruff format src/ tests/      # Format
```

## Code Style (PEP 8 + Google Docstrings)

### Naming
- `snake_case` for functions, methods, variables
- `PascalCase` for classes
- `SCREAMING_SNAKE_CASE` for constants
- `_private` prefix for internal APIs

### Docstrings (Google style)
```python
def resolve(self, key: str, default: str | None = None) -> str:
    """Resolve a configuration value by key.

    Args:
        key: The configuration path (e.g., "database.host").
        default: Fallback value if key not found.

    Returns:
        The resolved configuration value.

    Raises:
        KeyError: If key not found and no default provided.
    """
```

### Type Hints
- Required for public APIs (matches `.pyi` stubs)
- Use `str | None` (Python 3.10+) not `Optional[str]`
- Use `list[str]` not `List[str]`

## PyO3 Specifics
- Rust functions exposed via `#[pyfunction]` must handle Python exceptions
- Use `PyResult<T>` return types in Rust bindings
- Test Python bindings with actual Python calls, not just Rust tests

## Access Patterns
```python
config = Config.load("config.yaml")
config.database.port          # Dot notation
config["database"]["port"]    # Bracket notation
config.get("database.port")   # Path string
```

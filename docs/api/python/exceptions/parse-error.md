# ParseError

Raised when YAML or JSON content cannot be parsed due to syntax errors.

## When It's Raised

- Invalid YAML syntax (missing colons, bad indentation, etc.)
- Invalid JSON syntax (missing quotes, trailing commas, etc.)
- Encoding errors in the configuration file
- Malformed interpolation syntax

## Example

```python
from holoconf import Config, ParseError

try:
    # Invalid YAML - missing colon
    config = Config.loads("""
    database
      host: localhost
    """)
except ParseError as e:
    print(f"Parse error: {e}")
    # Parse error: expected ':', but found '-' at line 2 column 3
```

## Handling

```python
from holoconf import Config, ParseError

def load_config(path: str) -> Config:
    try:
        return Config.load(path)
    except ParseError as e:
        print(f"Failed to parse {path}: {e}")
        raise SystemExit(1)
```

## Class Reference

::: holoconf.ParseError
    options:
      show_root_heading: false

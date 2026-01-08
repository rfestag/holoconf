# CircularReferenceError

Raised when self-references create a circular dependency.

## When It's Raised

- Direct circular reference: `a: ${a}`
- Indirect circular reference: `a: ${b}` and `b: ${a}`
- Longer cycles: `a: ${b}`, `b: ${c}`, `c: ${a}`

## Example

```python
from holoconf import Config, CircularReferenceError

config = Config.loads("""
a: ${b}
b: ${c}
c: ${a}
""")

try:
    value = config.get("a")
except CircularReferenceError as e:
    print(f"Circular reference: {e}")
    # Circular reference: Circular reference detected at path 'a'
```

## Prevention

Ensure your self-references form a directed acyclic graph (DAG):

```yaml
# Good - no cycles
defaults:
  timeout: 30

database:
  timeout: ${defaults.timeout}

cache:
  timeout: ${defaults.timeout}
```

```yaml
# Bad - cycle between a and b
a: ${b}
b: ${a}
```

## Class Reference

::: holoconf.CircularReferenceError
    options:
      show_root_heading: false

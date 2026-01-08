# ValidationError

Raised when configuration fails JSON Schema validation.

## When It's Raised

- Required property is missing
- Value doesn't match expected type
- Value fails pattern or format validation
- Value outside min/max bounds
- Additional properties when not allowed

## Example

```python
from holoconf import Config, Schema, ValidationError

config = Config.loads("""
database:
  port: -1
""")

schema = Schema.from_json("""
{
  "type": "object",
  "properties": {
    "database": {
      "type": "object",
      "properties": {
        "port": {"type": "integer", "minimum": 1, "maximum": 65535}
      }
    }
  }
}
""")

try:
    config.validate(schema)
except ValidationError as e:
    print(f"Validation failed: {e}")
    # Validation failed: -1 is less than the minimum of 1 at path 'database.port'
```

## Collecting All Errors

To get all validation errors instead of failing on the first:

```python
errors = config.validate_collect(schema)
if errors:
    print("Validation errors:")
    for error in errors:
        print(f"  - {error}")
```

## Class Reference

::: holoconf.ValidationError
    options:
      show_root_heading: false

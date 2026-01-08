# FEAT-005: Serialization and Export

## Overview

Export configuration to YAML, JSON, or dict formats for debugging, auditing, and integration with other tools. Includes resolver-aware redaction to prevent secrets from being exposed.

## User Stories

- As a developer, I want to see the fully resolved config for debugging
- As an operator, I want to audit what config was used for a deployment
- As a developer, I want to export config without exposing secrets
- As a developer, I want to see the raw config with placeholders for documentation

## Dependencies

- [ADR-009: Serialization and Export](../../adr/ADR-009-serialization-export.md)
- [FEAT-001: Configuration File Loading](FEAT-001-config-loading.md)
- [FEAT-002: Core Resolvers](FEAT-002-core-resolvers.md)

## API Surface

### Python

```python
from holoconf import Config

config = Config.load("config.yaml")

# Export as YAML (default: unresolved, shows ${...} placeholders)
yaml_str = config.to_yaml()

# Export resolved values with automatic secret redaction
yaml_str = config.to_yaml(resolve=True, redact=True)

# Export resolved values WITHOUT redaction (use with caution!)
yaml_str = config.to_yaml(resolve=True, redact=False)

# Export as JSON
json_str = config.to_json()
json_str = config.to_json(resolve=True, redact=True)

# Export as dict (for programmatic use)
data = config.to_dict()
data = config.to_dict(resolve=True, redact=True)
```

### JavaScript

```javascript
const config = await Config.load("config.yaml");

// Export as YAML
const yaml = config.toYaml();
const yaml = config.toYaml({ resolve: true, redact: true });

// Export as JSON
const json = config.toJson();
const json = config.toJson({ resolve: true, redact: true });

// Export as object
const obj = config.toObject();
const obj = config.toObject({ resolve: true, redact: true });
```

## Behavior

### Export Modes

| Mode | `resolve` | `redact` | Output |
|------|-----------|----------|--------|
| Default (safe) | `false` | N/A | Shows `${env:VAR}` placeholders |
| Debug (resolved) | `true` | `true` | Resolved values, secrets redacted |
| Full export | `true` | `false` | Everything including secrets |

### Unresolved Export (Default)

Shows the configuration as-is with interpolation placeholders:

```yaml
# Input config
database:
  host: ${env:DB_HOST}
  password: ${ssm:/prod/db/password}
  port: 5432
```

```python
config.to_yaml()
```

```yaml
# Output
database:
  host: ${env:DB_HOST}
  password: ${ssm:/prod/db/password}
  port: 5432
```

### Resolved Export with Redaction

Resolves values and redacts sensitive ones:

```python
config.to_yaml(resolve=True, redact=True)
```

```yaml
# Output
database:
  host: db.prod.example.com       # Resolved from env
  password: "[REDACTED]"          # SSM SecureString - redacted
  port: 5432
```

### Resolver-Aware Redaction (from [ADR-009](../../adr/ADR-009-serialization-export.md))

Redaction is determined by the resolver, not just the resolver name:

| Resolver | Redaction Rule |
|----------|----------------|
| `env` | Not redacted (by default) |
| `ssm` | SecureString → redacted, String → not redacted |
| `vault` | Always redacted |
| `secretsmanager` | Always redacted |
| `file` | Not redacted |
| Self-reference | Inherits from source value |

```yaml
# config.yaml
database:
  host: ${ssm:/prod/db/host}          # SSM String → not redacted
  password: ${ssm:/prod/db/password}  # SSM SecureString → redacted
  api_key: ${vault:secret/api/key}    # Vault → redacted
  port: ${env:DB_PORT}                # Env → not redacted
```

```yaml
# Output with resolve=True, redact=True
database:
  host: "db.prod.example.com"
  password: "[REDACTED]"
  api_key: "[REDACTED]"
  port: "5432"
```

### Redaction Format

Redacted values are replaced with the string `"[REDACTED]"`:

```python
REDACTED_VALUE = "[REDACTED]"
```

### Full Export (No Redaction)

For cases where you need the actual values (use with extreme caution):

```python
# WARNING: This exposes secrets!
config.to_yaml(resolve=True, redact=False)
```

```yaml
database:
  host: "db.prod.example.com"
  password: "actual-secret-password"  # Exposed!
  api_key: "sk-1234567890"            # Exposed!
  port: "5432"
```

### Dict/Object Export

For programmatic use, export to native dict/object:

```python
data = config.to_dict(resolve=True, redact=True)

# Returns:
{
    "database": {
        "host": "db.prod.example.com",
        "password": "[REDACTED]",
        "port": 5432
    }
}
```

### Type Preservation

Resolved values maintain their types:

```python
config.to_dict(resolve=True)
# {
#     "port": 5432,          # integer (if coerced via schema)
#     "debug": True,         # boolean
#     "timeout": 30.5,       # float
#     "name": "myapp"        # string
# }
```

## Error Cases

### Resolution Error During Export

If resolution fails during export, the error is raised:

```python
try:
    yaml_str = config.to_yaml(resolve=True)
except ResolverError as e:
    print(f"Failed to resolve {e.path}: {e.message}")
```

### Circular Reference

```
CircularReferenceError: Circular reference detected during export
  Path: a
  Chain: a → b → c → a
```

## Examples

### Debugging Configuration

```python
config = Config.load("config.yaml")

# See what the config looks like before resolution
print("=== Raw Config ===")
print(config.to_yaml())

# See resolved values (safe for logs)
await config.resolve_all()
print("=== Resolved Config (redacted) ===")
print(config.to_yaml(resolve=True, redact=True))
```

### Audit Logging

```python
import logging

logger = logging.getLogger(__name__)

config = Config.load("config.yaml", "production.yaml")

# Log the config used for deployment (safe, no secrets)
logger.info(
    "Deployment configuration",
    extra={"config": config.to_dict(resolve=True, redact=True)}
)
```

### Configuration Diff

```python
# Compare configs across environments
dev_config = Config.load("base.yaml", "development.yaml")
prod_config = Config.load("base.yaml", "production.yaml")

dev_dict = dev_config.to_dict()
prod_dict = prod_config.to_dict()

# Use your preferred diff tool
from deepdiff import DeepDiff
diff = DeepDiff(dev_dict, prod_dict)
print(diff)
```

### Export for Other Tools

```python
# Generate config for another tool that needs JSON
config = Config.load("holoconf.yaml")

with open("output.json", "w") as f:
    f.write(config.to_json(resolve=True, redact=False))
```

### Template Generation

```python
# Export unresolved config as documentation template
config = Config.load("config.yaml")

with open("config.template.yaml", "w") as f:
    f.write("# Configuration Template\n")
    f.write("# Replace ${...} placeholders with your values\n\n")
    f.write(config.to_yaml())
```

## Implementation Notes

### Serialization Libraries

- YAML: Use `serde_yaml` in Rust
- JSON: Use `serde_json` in Rust
- Both support pretty-printing options

### Redaction Implementation

1. During serialization, check each value's metadata
2. If `sensitive=True` and `redact=True`, replace with `"[REDACTED]"`
3. Sensitivity is tracked per-value from resolver results

### Resolution During Export

When `resolve=True`:
1. Walk the config tree
2. For each interpolation, resolve it (may be cached)
3. Check sensitivity metadata
4. Apply redaction if needed
5. Serialize the result

### Performance Considerations

- Unresolved export is fast (just serialize stored values)
- Resolved export may trigger resolver calls (unless already cached)
- Consider `resolve_all()` before export if doing multiple exports

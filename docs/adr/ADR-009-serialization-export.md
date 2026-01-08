# ADR-009: Serialization and Export

## Status

- **Proposed by:** Ryan on 2026-01-07
- **Accepted on:** 2026-01-07

## Context

Users may need to export configuration data for various purposes:

- Debugging: See the fully resolved config
- Auditing: Log what config was used for a deployment
- Templating: Generate config files for other tools
- Inspection: View merged config before resolution

We need to decide what export capabilities holoconf provides and how they handle sensitive data.

## Alternatives Considered

### Alternative 1: No Export (Read-Only)

Config objects are read-only; no serialization back to YAML/JSON.

- **Pros:** Simpler implementation, no secrets-in-logs risk
- **Cons:** Users can't debug or audit configs easily
- **Rejected:** Export is essential for debugging and operational visibility

### Alternative 2: Full Export (No Redaction)

Export everything as-is, including resolved secrets.

```python
config.to_yaml()  # Includes passwords, API keys, etc.
```

- **Pros:** Simple, complete
- **Cons:** Security risk - secrets in logs/files
- **Rejected:** Too dangerous as default behavior

### Alternative 3: Export with Redaction Options

Provide export with configurable handling of sensitive values.

```python
config.to_yaml(redact=True)      # Replaces secrets with "[REDACTED]"
config.to_yaml(resolve=False)    # Shows ${...} placeholders
```

- **Pros:** Flexible, safe defaults possible
- **Cons:** More complex API
- **Chosen:** Best balance of safety and usability

## Open Questions (Proposal Phase)

*All resolved - see Decision section.*

## Next Steps (Proposal Phase)

- [ ] Implement export methods in holoconf-core
- [ ] Implement resolver-aware redaction logic
- [ ] Add schema-based and pattern-based redaction (future enhancement)

## Decision

**Export with Resolver-Aware Redaction**

- Output formats: YAML and JSON
- Default behavior: `resolve=False` (shows `${...}` placeholders, safest and fastest)
- Redaction: Resolver-implementation-aware (e.g., SSM `SecureString` types redacted, regular `String` types not)
- Redaction scope: Only affects serialization methods, not value access
- Flattened export: Out of scope for v1; can be added later if needed
- Future enhancements: Key pattern redaction, schema-based redaction (`x-sensitive`)

## Design

### Export API

```python
# Export as YAML (default: unresolved, shows ${...} placeholders)
yaml_str = config.to_yaml()

# Export resolved values with automatic redaction of secrets
yaml_str = config.to_yaml(resolve=True, redact=True)

# Export resolved values WITHOUT redaction (use with caution)
yaml_str = config.to_yaml(resolve=True, redact=False)

# Export as JSON
json_str = config.to_json()
json_str = config.to_json(resolve=True, redact=True)

# Export as dict (for programmatic use)
data = config.to_dict()
data = config.to_dict(resolve=True, redact=True)
```

### Export Modes

| Mode | `resolve` | `redact` | Output |
|------|-----------|----------|--------|
| Default (safe) | `False` | N/A | Shows `${ssm:/path}` placeholders |
| Debug (resolved) | `True` | `True` | Shows resolved values, secrets redacted |
| Full export | `True` | `False` | Shows everything including secrets |

### Resolver-Aware Redaction

Redaction is determined by the resolver implementation, not just the resolver name:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Redaction Decision Tree                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  SSM Resolver:                                                  │
│  ├── Parameter type is SecureString? → REDACT                   │
│  ├── Value is Secrets Manager reference? → REDACT               │
│  └── Regular String parameter? → DO NOT REDACT                  │
│                                                                 │
│  Vault Resolver:                                                │
│  └── All values → REDACT (it's a secrets manager)               │
│                                                                 │
│  Secrets Manager Resolver:                                      │
│  └── All values → REDACT                                        │
│                                                                 │
│  Env Resolver:                                                  │
│  └── All values → DO NOT REDACT (by default)                    │
│                                                                 │
│  Self-Reference Resolver:                                       │
│  └── Inherits sensitivity from source value                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

Example:

```yaml
# config.yaml
database:
  host: ${ssm:/prod/db/host}          # SSM String → not redacted
  password: ${ssm:/prod/db/password}  # SSM SecureString → redacted
  api_key: ${vault:secret/api/key}    # Vault → redacted
  vpc_id: ${ssm:/prod/vpc-id}         # SSM String → not redacted

# Output with resolve=True, redact=True
database:
  host: "db.prod.example.com"
  password: "[REDACTED]"
  api_key: "[REDACTED]"
  vpc_id: "vpc-abc123"
```

### Redaction Scope

**Important:** Redaction only applies to serialization methods. Once a value is accessed programmatically, holoconf cannot prevent it from being logged or printed.

```python
# Redaction works here:
config.to_yaml(resolve=True, redact=True)  # "[REDACTED]"

# Redaction does NOT apply here - user has the raw value:
password = config.database.password  # Returns actual secret string
print(password)                      # User's responsibility
```

This is intentional - holoconf is a configuration library, not a secrets management SDK. Users who need value-level protection (e.g., types that refuse to print themselves) should use dedicated secrets libraries.

### Future Enhancements (Out of Scope for v1)

**Key pattern redaction:**
```python
config.to_yaml(resolve=True, redact=True, redact_patterns=["*password*", "*secret*"])
```

**Schema-based redaction:**
```yaml
# schema.yaml
properties:
  custom_secret:
    type: string
    x-sensitive: true  # JSON Schema extension
```

**Flattened export:**
```python
config.to_flat_dict()  # {"database.host": "...", "database.port": 5432}
```

## Rationale

- **`resolve=False` as default** is safest - no secrets exposed, no resolver calls needed
- **Resolver-aware redaction** is smarter than blanket resolver-name redaction (not all SSM values are secrets)
- **YAML + JSON** covers the input formats; TOML can be added if demand exists
- **No value-level protection** keeps the API simple and expectations clear

## Trade-offs Accepted

- **Redaction only at serialization** in exchange for **simple, predictable value access**
- **Resolver must report sensitivity** in exchange for **accurate redaction**
- **No flattened export initially** in exchange for **simpler initial implementation**

## Migration

N/A - This is a new feature.

## Consequences

- **Positive:** Safe debugging, audit trails, operational visibility without secret exposure
- **Negative:** Users must be careful with `redact=False`; resolver implementations must track sensitivity
- **Neutral:** Future pattern/schema-based redaction can be added without breaking changes

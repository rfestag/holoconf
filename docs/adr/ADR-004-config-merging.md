# ADR-004: Config Merging Semantics

## Status

- **Proposed by:** Ryan on 2026-01-07
- **Accepted on:** 2026-01-07

## Context

holoconf supports loading and merging multiple configuration files. Users load configs in a specific order, where later files override earlier ones:

```python
config = Config.load("base.yaml", "environment.yaml", "local.yaml")
#                    ^ lowest priority              ^ highest priority
```

The merge order is determined by the user, not holoconf. Users may organize configs however suits their deployment model (by environment, region, team, feature flags, etc.).

We need to define how merging works when the same keys appear in multiple files.

## Alternatives Considered

### Alternative 1: Shallow Merge (top-level only)

Only merge top-level keys; nested objects replace entirely.

- **Pros:** Simple, predictable
- **Cons:** Can't override nested values without replacing entire subtree
- **Rejected:** Too limiting for real-world hierarchical configs

### Alternative 2: List Concatenation by Default

When merging lists, concatenate them.

- **Pros:** Additive, nothing lost
- **Cons:** Often not desired (e.g., overriding log handlers), harder to remove items
- **Rejected:** Replace is more commonly expected behavior

### Alternative 3: JSON Merge Patch (RFC 7396)

Use the standard JSON Merge Patch algorithm.

- **Pros:** Standard
- **Cons:** Limited (no way to append to arrays, null removes keys)
- **Rejected:** Too limited, but we adopt null-removes-key from it

## Decision

**Deep Merge with Last-Writer-Wins for Scalars**

Merging follows these rules:

1. **Dictionaries/Objects:** Deep merge recursively
2. **Scalars (strings, numbers, booleans):** Last value wins (later config overrides earlier)
3. **Lists/Arrays:** Replace (later list replaces earlier list entirely)
4. **Null values:** Explicit null removes key from merged result
5. **Type mismatches:** Last value wins (later value replaces entirely)

## Design

### Basic Merging

```yaml
# base.yaml (loaded first)
database:
  host: localhost
  port: 5432
  options:
    timeout: 30
    retries: 3
logging:
  level: info
  handlers:
    - console

# override.yaml (loaded second, higher priority)
database:
  host: prod-db.example.com    # overrides scalar
  options:
    timeout: 60                 # overrides nested scalar
    pool_size: 10               # adds new key
logging:
  level: debug                  # overrides scalar
  handlers:                     # replaces list (default behavior)
    - file
    - syslog

# Result after merge:
database:
  host: prod-db.example.com    # from override
  port: 5432                    # from base (preserved)
  options:
    timeout: 60                 # from override
    retries: 3                  # from base (preserved)
    pool_size: 10               # from override (added)
logging:
  level: debug
  handlers:
    - file
    - syslog
```

### Null Removes Key

```yaml
# base.yaml
feature:
  enabled: true
  config:
    setting: value

# override.yaml
feature:
  config: null   # removes 'config' entirely

# Result:
feature:
  enabled: true
  # config is removed
```

### Type Mismatch Handling

When types don't match, the later value wins entirely (replacement, not merge):

```yaml
# base.yaml
database:
  host: localhost
  port: 5432

# override.yaml
database: "postgresql://prod-db/app"   # scalar replaces dict

# Result:
database: "postgresql://prod-db/app"
```

```yaml
# base.yaml
database: "postgresql://localhost/app"

# override.yaml
database:                              # dict replaces scalar
  host: prod-db
  port: 5432

# Result:
database:
  host: prod-db
  port: 5432
```

This behavior is consistent with last-writer-wins and allows users to restructure configs at higher hierarchy levels.

## Rationale

- **Deep merge preserves base config** while allowing targeted overrides
- **Last-writer-wins is intuitive** for operators ("my override file wins")
- **List replacement avoids surprising concatenation** behavior
- **Type mismatch replacement is consistent** with last-writer-wins semantics
- **No strict mode for v1** - keeps the API simple; can be added if users request it
- **Future: append/prepend could be implemented as resolvers** if needed

## Trade-offs Accepted

- **List replacement may surprise users** expecting concatenation in exchange for **simpler mental model**
- **No automatic conflict detection** (last writer silently wins) in exchange for **predictable behavior**
- **Type mismatches silently replace** in exchange for **flexibility in restructuring configs**

## Migration

N/A - This is the initial architecture decision.

## Consequences

- **Positive:** Intuitive override behavior, nested configs work naturally, flexible restructuring at higher levels
- **Negative:** List operations require workarounds (future resolver-based append), type changes may be unintentional
- **Neutral:** Matches common YAML config tooling behavior (Helm, Kustomize)

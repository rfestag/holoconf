# ADR-005: Resolver Timing (Lazy Resolution)

## Status

- **Proposed by:** Ryan on 2026-01-07
- **Accepted on:** 2026-01-07

## Context

OmegaConf resolves interpolations at parse time, before configs are merged. This causes issues:

- Resolvers run even if the value is later overridden
- External resolver calls (SSM, S3) happen at import time, slowing startup
- Errors occur for values that would be overridden anyway
- No way to inspect unresolved config structure

We need to decide when resolution occurs.

## Alternatives Considered

### Alternative 1: Parse-Time Resolution (OmegaConf behavior)

Resolve all interpolations when the config is parsed.

- **Pros:** Simple mental model, all values ready immediately
- **Cons:** Slow startup, resolves overridden values, errors on unused invalid refs
- **Rejected:** This is the pain point we're solving

### Alternative 2: Explicit Resolution Step

Require users to call `config.resolve()` explicitly.

- **Pros:** Clear when resolution happens
- **Cons:** Extra boilerplate, easy to forget
- **Rejected:** Lazy is more ergonomic

### Alternative 3: No Memoization (resolve every access)

Resolve values fresh on every access.

- **Pros:** Always fresh values
- **Cons:** Performance disaster, inconsistent values during execution
- **Rejected:** Memoization is essential

## Decision

**Lazy Resolution with Memoization**

Resolvers execute lazily when values are accessed, not at parse/merge time. Resolved values are memoized so each resolver runs exactly once per key.

Key decisions:
- **No TTL/expiry** - Config values are stable for the lifetime of the Config object
- **Circular reference detection at access time** - Detected during lazy evaluation, not at parse
- **`resolve_all()` is eager** - Resolves all pending interpolations in parallel

## Design

### Timeline

```
Parse time:     config = Config.load("base.yaml", "override.yaml")
                -> Config object created with unresolved interpolations
                -> No resolver calls yet
                -> Merge happens on raw structures

Access time:    value = config.database.password
                -> Resolver executes: ${ssm:/prod/db/password}
                -> Result memoized in config object

Second access:  value2 = config.database.password
                -> Returns memoized value (no resolver call)
```

### Benefits

```python
# With lazy resolution:

# 1. Fast startup - no resolver calls at parse time
config = Config.load("base.yaml", "prod.yaml")  # Instant

# 2. Override works correctly
# base.yaml:   password: ${ssm:/base/password}
# prod.yaml:   password: ${ssm:/prod/password}
# Only /prod/password is ever resolved (base was overridden)

# 3. Can inspect unresolved structure
config.to_yaml(resolve=False)  # Shows ${...} placeholders

# 4. Unused values never resolved
# If config.unused_feature.api_key is never accessed,
# that SSM call never happens
```

### API Surface

```python
# Python
config = Config.load("config.yaml")

# Lazy access (resolves on demand)
password = config.database.password

# Force resolution of entire config (parallel, see ADR-003)
await config.resolve_all()

# Get unresolved value (for debugging/inspection)
raw = config.get_raw("database.password")  # Returns "${ssm:/path}"

# Check if resolved
config.is_resolved("database.password")  # True/False
```

### Memoization Semantics

- Values are memoized per-Config-instance
- No TTL/expiry - values remain stable for the Config object's lifetime
- To get fresh values, create a new Config object (reload the config)
- This ensures consistent behavior during application execution

```python
# Values are stable within a Config instance
config = Config.load("config.yaml")
v1 = config.api.key  # resolves ${ssm:/api/key}
# ... SSM value changes externally ...
v2 = config.api.key  # returns same memoized value as v1

# To get fresh values, reload
config = Config.load("config.yaml")  # new instance
v3 = config.api.key  # re-resolves, gets new value
```

### Circular Reference Detection

Circular references are detected at access time during lazy evaluation:

```yaml
# config.yaml
a: ${b}
b: ${a}
```

```python
config = Config.load("config.yaml")  # OK - no resolution yet
config.a  # raises CircularReferenceError: a -> b -> a
```

Detection works by tracking the resolution stack. If a key is encountered that's already being resolved, it's a circular reference.

### Interaction with Async (ADR-003)

- Lazy resolution integrates naturally with async
- Individual access resolves sequentially (lazy behavior)
- `resolve_all()` parallelizes all pending resolutions
- See ADR-003 for parallelism semantics

## Rationale

- **Lazy resolution avoids wasted work** on overridden/unused values
- **Memoization ensures consistency** and performance
- **No TTL keeps the model simple** - reload for fresh values
- **Access-time circular detection** is consistent with lazy philosophy
- **Matches how most developers expect** config access to work
- **Enables fast startup** (critical for Lambda, CLI tools)

## Trade-offs Accepted

- **First access to a value may be slow** (resolver runs) in exchange for **fast startup**
- **Memoization means value won't update** if external source changes in exchange for **consistency during execution**
- **Debugging requires understanding lazy evaluation** in exchange for **performance benefits**
- **Circular reference detection happens at access time**, not parse time in exchange for **avoiding unnecessary resolution**

## Migration

N/A - This is the initial architecture decision.

## Consequences

- **Positive:** Fast startup, no wasted resolver calls, clean override semantics, stable values during execution
- **Negative:** First access latency, debugging lazy values can be confusing, must reload for fresh values
- **Neutral:** Different mental model than OmegaConf (but better for our use case)

# ADR-003: Async Execution Model

## Status

- **Proposed by:** Ryan on 2026-01-07
- **Accepted on:** 2026-01-07

## Context

Resolvers may need to fetch values from external services (SSM, S3, HTTP endpoints). A configuration file might have many such references. We need to decide how resolver execution works.

Key considerations:

- Configs with multiple external references should resolve quickly
- AWS Lambda cold starts are performance-critical
- Some contexts prefer sync APIs (scripts, CLI tools)

## Alternatives Considered

### Alternative 1: Sync-Only

All resolver execution is synchronous and sequential.

- **Pros:** Simpler FFI, works everywhere
- **Cons:** Sequential resolver execution, slow for configs with many external refs
- **Rejected:** Performance critical for configs with multiple SSM/S3 lookups

### Alternative 2: Async-Only

Only expose async APIs.

- **Pros:** Maximum parallelism
- **Cons:** Some languages/contexts prefer sync (scripts, CLI tools)
- **Rejected:** Too restrictive

## Decision

**Async-First with Parallel Resolution**

- Rust core uses async (tokio) internally
- Resolvers execute in parallel when explicitly requested via `resolve_all()`
- Language bindings expose both sync and async APIs
- Lazy resolution by default (sequential on access), parallel on demand

## Design

### Execution Model

```
Config with 5 SSM references:

Sequential access:       resolve_all():
config.a  # SSM1 ───►    SSM1 ──┐
config.b  # SSM2 ───►    SSM2 ──┼──► All parallel
config.c  # SSM3 ───►    SSM3 ──┤
config.d  # SSM4 ───►    SSM4 ──┤
config.e  # SSM5 ───►    SSM5 ──┘
Total: 5 × latency       Total: 1 × latency
```

### API Surface

```python
# Python - lazy access (sequential resolution)
config = Config.load("config.yaml")
host = config.database.host  # resolves on access

# Python - explicit parallel resolution
config = Config.load("config.yaml")
await config.resolve_all()  # resolve all refs in parallel
host = config.database.host  # already resolved, instant

# Python - sync parallel resolution
config = Config.load("config.yaml")
config.resolve_all_sync()  # blocks, but parallel internally
```

```javascript
// JavaScript - lazy access
const config = Config.load("config.yaml");
const host = config.database.host;  // resolves on access

// JavaScript - explicit parallel resolution
const config = Config.load("config.yaml");
await config.resolveAll();  // resolve all refs in parallel
const host = config.database.host;  // already resolved
```

### Sync Callback Integration

Sync custom resolver callbacks are wrapped in `spawn_blocking` to integrate with the async core:

```python
# User's sync resolver
def my_resolver(key: str) -> str:
    return lookup_value(key)  # sync code

# Internally wrapped as:
# spawn_blocking(|| callback(key))
```

This allows sync callbacks to work without blocking the tokio runtime, while async callbacks run natively.

### Timeout Handling

Two levels of timeout control:

1. **Per-invocation timeout** (default: 30s) - Individual resolver calls timeout independently
2. **Batch timeout** (optional) - `resolve_all(timeout=5.0)` sets a deadline for all resolutions

```python
# Per-invocation timeout (configured at resolver registration)
holoconf.register("slow_resolver", my_resolver, timeout=60.0)

# Batch timeout for resolve_all
await config.resolve_all(timeout=10.0)  # all must complete in 10s
```

### Concurrency Limits

Global concurrency limit with sensible default (e.g., 50 concurrent resolver calls):

```python
# Configure global limit
holoconf.configure(max_concurrent_resolvers=100)

# Or at resolve_all
await config.resolve_all(max_concurrency=20)
```

This prevents overwhelming external services when configs have many references.

## Rationale

- **AWS Lambda cold starts benefit significantly** from parallel SSM fetches
- **tokio is the standard async runtime** for Rust - mature and well-supported
- **Sync wrapper can use `block_on` internally** while still parallelizing resolver calls
- **Lazy by default follows intuitive semantics** - access triggers resolution
- **Explicit `resolve_all()` gives users control** over when parallelism happens
- **Per-invocation + batch timeouts** provide flexibility without complexity
- **Global concurrency limits** prevent accidental DoS of external services

## Trade-offs Accepted

- **tokio dependency adds to binary size** in exchange for **parallel execution**
- **Async FFI is more complex than sync** in exchange for **better performance**
- **Sync resolver callbacks require `spawn_blocking`** in exchange for **simple user API**
- **Lazy resolution is sequential by default** in exchange for **intuitive access semantics**

## Migration

N/A - This is the initial architecture decision.

## Consequences

- **Positive:** Fast config loading with many external references, good Lambda cold start performance, intuitive lazy access semantics
- **Negative:** Async complexity in FFI layer, larger binary size, users must call `resolve_all()` explicitly for parallelism
- **Neutral:** Users can choose sync or async API based on their needs

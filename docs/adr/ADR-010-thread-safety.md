# ADR-010: Thread Safety and Concurrency

## Status

- **Proposed by:** Ryan on 2026-01-07
- **Accepted on:** 2026-01-07

## Context

The Config object may be accessed from multiple threads/contexts:

- Python: Multiple threads (with GIL), async tasks
- JavaScript: Event loop, worker threads
- Go: Multiple goroutines
- Java: Multiple threads
- Rust: Multiple threads

Key questions:

- Can a Config object be safely shared across threads?
- What happens if two threads access the same unresolved value simultaneously?
- How does lazy resolution interact with concurrent access?
- How does memoization work under concurrent access?

## Alternatives Considered

### Alternative 1: Single-Threaded Only

Config objects are not thread-safe; users must not share across threads.

- **Pros:** Simpler implementation, no synchronization overhead
- **Cons:** Limits use cases, error-prone (silent data races)
- **Rejected:** Multi-threaded access is a common pattern

### Alternative 2: Full Thread Safety (Internal Locking)

Config objects are fully thread-safe with internal synchronization.

```python
# Safe to do from multiple threads
config = Config.load("config.yaml")
# Thread 1: config.database.host
# Thread 2: config.database.port
# Thread 3: config.api.timeout
```

- **Pros:** Users don't need to think about synchronization
- **Cons:** Synchronization overhead, potential for deadlocks
- **Chosen:** With per-value locking (not whole-config locking) to minimize contention

### Alternative 3: Clone for Thread Transfer

Config objects are not thread-safe, but cheap to clone.

```python
config = Config.load("config.yaml")
# Pass clone to each thread
thread1_config = config.clone()
thread2_config = config.clone()
```

- **Pros:** No synchronization overhead during access, clear ownership
- **Cons:** Memory duplication, resolved values not shared
- **Rejected:** Wastes memory and resolver calls; shared cache is more efficient

### Alternative 4: Read-Only After Load

Config becomes immutable and thread-safe after initial resolution.

```python
config = Config.load("config.yaml")
await config.resolve_all()  # After this, config is read-only and thread-safe
# Now safe to share across threads
```

- **Pros:** Clear lifecycle, no synchronization during reads
- **Cons:** Forces eager resolution, doesn't work with lazy access pattern
- **Rejected:** Conflicts with lazy resolution (ADR-005)

## Open Questions (Proposal Phase)

*All resolved - see Decision section.*

## Next Steps (Proposal Phase)

- [ ] Implement `Arc<RwLock<Cache>>` pattern in holoconf-core
- [ ] Add async variants to language bindings (Python awaitable, JS Promise)
- [ ] Benchmark locking overhead vs no-locking baseline
- [ ] Test with Python GIL, JS event loop, Go goroutines

## Decision

**Thread-Safe Config with Per-Value Locking and Shared Cache**

- Locking granularity: Per-value locking; first accessor resolves, others wait for cached result
- Clone semantics: Shared cache via `Arc` - clones share resolved values, memory efficient
- `resolve_all()` pattern: Supported - after completion, all access is cache reads with no blocking
- Language runtimes: Async handled in Rust (tokio); bindings expose both sync and async APIs
- Config implements `Send + Sync` in Rust

## Design

### Thread-Safe with Per-Value Locking

Config objects are thread-safe with fine-grained locking:

```
┌─────────────────────────────────────────────────────────────┐
│                     Config Object                            │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              Resolved Value Cache                    │    │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐             │    │
│  │  │ db.host │  │ db.port │  │ api.key │  ...        │    │
│  │  │ [Lock]  │  │ [Lock]  │  │ [Lock]  │             │    │
│  │  └─────────┘  └─────────┘  └─────────┘             │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

### Concurrent Resolution Behavior

When two threads access the same unresolved value simultaneously:

```
Thread 1: config.database.password
Thread 2: config.database.password

Timeline:
─────────────────────────────────────────────────────────────►
T1: Check cache (miss) → Acquire lock → Start resolution
T2: Check cache (miss) → Wait on lock...
T1: Resolution complete → Store in cache → Release lock
T2: Acquire lock → Check cache (hit!) → Release lock → Return cached value
```

Only one resolution occurs; the second thread waits and gets the cached result.

### Language-Specific Considerations

**Python (GIL)**
- GIL provides some protection, but async and thread pools need explicit safety
- PyO3 releases GIL during Rust operations; re-acquire for callbacks

**JavaScript (Event Loop)**
- Single-threaded event loop; concurrency via async
- Worker threads are separate isolates (would need separate Config instances)
- Main concern: async resolution interleaving

**Go (Goroutines)**
- True parallelism; needs proper synchronization
- cgo boundary considerations

**Rust (Native)**
- `Config` implements `Send + Sync`
- Internal `RwLock` or similar for cache access

### API Surface

```python
# Thread-safe by default
config = Config.load("config.yaml")

# Explicit clone if needed (shares resolved cache via Arc)
config_clone = config.clone()  # Cheap, shares underlying data

# Check if a value is already resolved (non-blocking)
if config.is_resolved("database.password"):
    # Will not trigger resolution or block
    password = config.database.password
```

### Resolution Lock Semantics

| Operation | Locking Behavior |
|-----------|-----------------|
| Access resolved value | Read lock (concurrent reads OK) |
| Access unresolved value | Write lock (blocks other accessors of same key) |
| `resolve_all()` | Acquires locks per-key as needed |
| `to_yaml()` / `to_dict()` | Read locks on accessed values |

## Rationale

- **Per-value locking avoids duplicate resolver calls** - if two threads access the same unresolved value, only one calls the resolver
- **Shared cache via Arc is idiomatic Rust** - cheap clones, memory efficient, resolved values available to all clones
- **`resolve_all()` enables contention-free sharing** - users who want zero blocking during access can resolve upfront
- **Async in Rust, sync+async bindings** - keeps complexity in one place, language bindings stay simple

## Trade-offs Accepted

- **Per-value locking overhead** in exchange for **preventing duplicate resolver calls**
- **Shared cache means clones aren't isolated** in exchange for **memory efficiency and shared resolution work**
- **Blocking on unresolved values** in exchange for **simple, predictable API** (use `resolve_all()` to avoid)

## Migration

N/A - This is the initial architecture decision.

## Consequences

- **Positive:** Safe concurrent access, no duplicate resolver calls, efficient memory usage
- **Negative:** Threads may block waiting for resolution; requires understanding of lazy resolution behavior
- **Neutral:** Users wanting full isolation can load separate Config instances instead of cloning

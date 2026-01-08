# ADR-001: Multi-Language Architecture

## Status

- **Proposed by:** Ryan on 2026-01-07
- **Accepted on:** 2026-01-07

## Context

holoconf needs to support multiple programming languages (Python, JavaScript initially; Go, Java, Rust, C later). We need an architecture that:

- Ensures consistent behavior across all languages
- Scales to 6+ languages without re-architecture
- Supports both x86_64 and aarch64 platforms

## Alternatives Considered

### Alternative 1: Schema-First (JSON Schema + codegen)

Generate language-specific code from a schema definition.

- **Pros:** Strong typing, schema serves as documentation
- **Cons:** Cannot express dynamic resolver behavior, limited runtime flexibility
- **Rejected:** Resolvers are inherently runtime/dynamic

### Alternative 2: Pure Specification (independent implementations)

Define a spec and implement it independently in each language.

- **Pros:** Language-idiomatic APIs, no FFI complexity
- **Cons:** N implementations to maintain, drift risk, duplicated effort scales with languages
- **Rejected:** Does not scale to 6+ languages

### Alternative 3: Hybrid Specification (spec + optional shared components)

Define a spec with optional shared components that languages can adopt.

- **Pros:** Flexibility, can start simple
- **Cons:** Still requires reimplementation when adding languages
- **Rejected:** Re-architecture cliff when adding language #3

## Decision

**Rust Core with Language Bindings**

Implement core configuration logic in Rust with FFI bindings for each target language:

- **Rust Core (holoconf-core):** YAML/JSON parsing, hierarchical merging, interpolation syntax parsing, resolver registry & dispatch
- **Language Bindings:** PyO3 (Python), NAPI-RS (JavaScript/Node), cgo (Go), JNI (Java), native (Rust), C headers (C)

## Design

```
┌─────────────────────────────────────────────────────┐
│              holoconf-core (Rust)                   │
│  - YAML/JSON parsing                                │
│  - Hierarchical merge algorithm                     │
│  - Interpolation syntax parsing ${...}              │
│  - Resolver registry & dispatch                     │
│  - Built-in resolvers: env, self-reference          │
└─────────────────────────────────────────────────────┘
                         │
              FFI Bindings (per language)
                         │
    ┌────────────────────┼────────────────────┐
    ▼                    ▼                    ▼
┌─────────┐        ┌─────────┐         ┌─────────┐
│ Python  │        │   JS    │         │  Future │
│ (PyO3)  │        │(NAPI-RS)│         │Languages│
└─────────┘        └─────────┘         └─────────┘
```

## Rationale

- **Single implementation guarantees behavioral consistency** - All languages use the same parsing, merging, and resolution logic
- **Adding new language = write bindings, not reimplement core** - Dramatically reduces effort to add new language support
- **Rust compiles to all target platforms** - x86_64, aarch64 supported out of the box
- **WASM compilation possible** - Browser support can be added later

### Error Handling

Rust `Result<T, E>` types are mapped to native exceptions/errors in each language:

```python
# Python
try:
    config = holoconf.load("config.yaml")
except holoconf.ParseError as e:
    print(f"Failed to parse: {e}")
except holoconf.ResolverError as e:
    print(f"Resolver failed: {e}")
```

```javascript
// JavaScript
try {
    const config = await holoconf.load("config.yaml");
} catch (e) {
    if (e instanceof holoconf.ParseError) { ... }
}
```

### Memory Management

Hybrid approach - Config wrapper with copy-on-access for scalars:

- Python/JS `Config` object holds an opaque reference to Rust-owned config data
- Accessing nested paths (e.g., `config.database`) returns another wrapper (no copy)
- Accessing scalar values (strings, numbers, booleans) copies the resolved value to native types
- Lazy resolution (ADR-005) happens in Rust; only resolved values cross the FFI boundary

This approach:
- Works naturally with lazy resolution (unresolved values stay in Rust)
- Is memory efficient (large configs aren't duplicated)
- Returns native types where it matters (`config.database.host` returns a native string)

### Crate Structure

See [ADR-006](ADR-006-repository-package-structure.md) for repository and package structure.

## Trade-offs Accepted

- **Rust expertise required** for core development in exchange for **single source of truth**
- **FFI boundaries add complexity** in exchange for **guaranteed consistency**
- **Language bindings must be maintained per-language** in exchange for **simpler than full reimplementation**

## Migration

N/A - This is the initial architecture decision.

## Consequences

- **Positive:** Consistent behavior across all languages, reduced maintenance burden, scales to many languages
- **Negative:** Requires Rust expertise, FFI debugging can be challenging
- **Neutral:** Language-specific idioms may need adaptation to fit the binding model

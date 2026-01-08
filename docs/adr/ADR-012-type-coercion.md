# ADR-012: Type Coercion

## Status

- **Proposed by:** Ryan on 2026-01-07
- **Accepted on:** 2026-01-07

## Context

Resolvers often return string values (e.g., `${env:PORT}` returns `"8080"`), but the schema or application may expect a different type (e.g., `integer`). We need to decide how holoconf handles type mismatches between resolved values and expected types.

This affects:
- Schema validation (ADR-007)
- Resolver return values (ADR-002)
- Serialization output (ADR-009)

## Alternatives Considered

### Alternative 1: No Coercion (Strict Types)

Resolved values must match the expected type exactly. Type mismatches raise errors.

```yaml
# Schema expects integer
port:
  type: integer

# Config
port: ${env:PORT}  # Returns "8080" (string) → ValidationError
```

- **Pros:** Explicit, no surprises, forces correct resolver implementation
- **Cons:** Env vars are always strings, would require wrapper resolvers or schema workarounds

### Alternative 2: Automatic Coercion (Implicit)

Automatically coerce values to match schema types when possible.

```yaml
# Schema expects integer
port:
  type: integer

# Config
port: ${env:PORT}  # Returns "8080" → coerced to 8080 (integer)
```

- **Pros:** Convenient, works naturally with env vars
- **Cons:** Can hide bugs, unexpected behavior if coercion fails silently

### Alternative 3: Schema-Driven Coercion (Explicit)

Coercion only happens when a schema is present and defines the expected type. Without a schema, values retain their resolved type.

```yaml
# With schema expecting integer: "8080" → 8080
# Without schema: "8080" stays "8080"
```

- **Pros:** Predictable, schema controls behavior, no coercion surprises without schema
- **Cons:** Behavior differs with/without schema

## Open Questions (Proposal Phase)

*All resolved - see Decision section.*

## Next Steps (Proposal Phase)

- [ ] Implement coercion in holoconf-core
- [ ] Test edge cases (empty strings, whitespace, locale-specific numbers)
- [ ] Document resolver-driven coercion option

## Decision

**Schema-Driven Coercion with Resolver Override**

- Coercion is primarily schema-driven: when a schema specifies an expected type, holoconf attempts to coerce the resolved value
- Resolvers may also perform their own coercion before returning values (resolver-driven)
- Schema coercion applies after resolver returns, so resolvers returning already-typed values skip coercion
- Boolean coercion is strict: only `"true"` and `"false"` (case-insensitive), not `"1"/"0"` or `"yes"/"no"`
- No string-to-array coercion; use a built-in resolver for splitting if needed

## Design

### Schema-Driven Coercion

Coercion happens automatically when:
1. A schema is provided
2. The schema specifies an expected type
3. The resolved value doesn't match the expected type
4. A valid coercion rule exists

### Resolver-Driven Coercion

Resolvers may return already-typed values:

```python
class SmartEnvResolver(Resolver):
    def resolve(self, key: str) -> ResolvedValue:
        value = os.environ.get(key)
        # Resolver does its own coercion
        if value.isdigit():
            return ResolvedValue(value=int(value), sensitive=False)
        return ResolvedValue(value=value, sensitive=False)
```

When a resolver returns a typed value:
- If it matches the schema type, no coercion needed
- If it doesn't match, schema coercion is attempted
- This allows resolvers to optimize for common cases

### Coercion Rules

| From | To | Rule | Example |
|------|-----|------|---------|
| string | integer | Parse as integer, fail if not valid | `"8080"` → `8080`, `"abc"` → error |
| string | number | Parse as float | `"3.14"` → `3.14` |
| string | boolean | `"true"/"false"` only (case-insensitive) | `"true"` → `true`, `"1"` → error |
| string | array | No coercion (use split resolver) | error |
| string | object | No coercion | error |
| integer | number | Widen to float | `8080` → `8080.0` |
| any | string | No coercion needed (strings accept any) | value unchanged |

### Coercion Failure Behavior

When coercion fails, raise `TypeValidationError` with context:

```
TypeValidationError: Cannot coerce value to expected type
  Path: database.port
  Expected: integer
  Got: string ("not-a-number")
  Resolved from: ${env:DB_PORT}
  Help: Ensure DB_PORT environment variable contains a valid integer
```

### No-Schema Behavior

Without a schema, no coercion occurs. Values retain their resolved type:
- `${env:PORT}` → `"8080"` (string)
- Accessing `config.port` returns `"8080"` (string)

### Opt-Out

If schema specifies `type: string`, no coercion occurs even if the value looks like a number.

## Rationale

- **Schema-driven coercion is predictable** - users know when coercion happens (only with schema)
- **Resolver override enables optimization** - resolvers can return typed values to skip coercion
- **Strict boolean parsing prevents bugs** - `"1"` or `"yes"` silently becoming `true` can hide errors
- **No string-to-array keeps things simple** - complex parsing should be explicit via resolvers

## Trade-offs Accepted

- **Stricter boolean coercion** may require users to update configs using `"1"/"0"`, in exchange for **predictable behavior**
- **No implicit array splitting** requires explicit resolver usage, in exchange for **avoiding ambiguous parsing**
- **Schema required for coercion** means unvalidated configs keep string types, in exchange for **explicit behavior**

## Migration

N/A - This is the initial architecture decision.

## Consequences

- **Positive:** Predictable coercion, resolvers can optimize, clear separation of concerns
- **Negative:** Users expecting `"1"` → `true` will need to adjust
- **Neutral:** Resolvers have flexibility to do their own coercion if desired

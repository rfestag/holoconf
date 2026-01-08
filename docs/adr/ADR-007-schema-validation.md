# ADR-007: Schema and Validation

## Status

- **Proposed by:** Ryan on 2026-01-07
- **Accepted on:** 2026-01-07

## Context

Users need a way to define and enforce the expected shape of their configuration. This includes:

- Documenting what keys exist and what they mean
- Specifying types, allowed values, ranges, and patterns
- Providing example values and descriptions
- Validating configs at runtime with helpful error messages

Key constraints:

- The schema format must be language-agnostic (not Python-specific, not JS-specific)
- Must work with multiple input formats (YAML, JSON)
- Should be simple and intuitive to write
- Must interact sensibly with lazy resolution (ADR-005)

## Alternatives Considered

### Alternative 1: JSON Schema

Use the industry-standard JSON Schema format.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "database": {
      "type": "object",
      "properties": {
        "host": { "type": "string", "description": "Database hostname" },
        "port": { "type": "integer", "minimum": 1, "maximum": 65535 }
      },
      "required": ["host"]
    }
  }
}
```

- **Pros:** Industry standard, extensive tooling, editor support, language-agnostic
- **Cons:** Verbose, JSON syntax is tedious to write by hand, advanced features are complex
- **Status:** Under consideration

### Alternative 2: YAML Schema (JSON Schema in YAML)

JSON Schema but written in YAML for readability.

```yaml
type: object
properties:
  database:
    type: object
    description: Database connection settings
    properties:
      host:
        type: string
        description: Database hostname
        examples: ["localhost", "db.example.com"]
      port:
        type: integer
        minimum: 1
        maximum: 65535
        default: 5432
    required: [host]
```

- **Pros:** Same power as JSON Schema, more readable, language-agnostic
- **Cons:** Still verbose for simple cases, learning curve for JSON Schema concepts
- **Status:** Under consideration

### Alternative 3: Inline Schema (schema in config file)

Define schema inline within the config file itself using special syntax.

```yaml
# @schema: { type: string, description: "Database hostname" }
host: localhost

# @schema: { type: integer, min: 1, max: 65535 }
port: 5432
```

- **Pros:** Schema lives with the config, no separate file
- **Cons:** Clutters config files, awkward for nested structures, non-standard
- **Rejected:** Mixes concerns, makes configs harder to read

### Alternative 4: Schema-as-Config (holoconf-native format)

A custom schema format designed specifically for holoconf, optimized for the common case.

```yaml
# schema.holoconf.yaml
database:
  _description: Database connection settings
  host:
    _type: string
    _required: true
    _description: Database hostname
    _examples: [localhost, db.example.com]
  port:
    _type: integer
    _range: [1, 65535]
    _default: 5432
  password:
    _type: string
    _description: Database password (typically from SSM)
```

- **Pros:** Mirrors config structure exactly, easy to understand, minimal syntax
- **Cons:** Yet another schema format, no existing tooling
- **Status:** Under consideration

### Alternative 5: Language-Native Types (codegen)

Generate schemas from language-native type definitions.

```python
# Python
@holoconf.schema
class DatabaseConfig:
    host: str
    port: int = 5432
    password: str | None = None
```

```typescript
// TypeScript
interface DatabaseConfig {
  host: string;
  port?: number;
  password?: string;
}
```

- **Pros:** Type-safe access in each language, IDE autocomplete
- **Cons:** Language-specific, requires codegen step, schema not portable
- **Rejected:** Violates language-agnostic constraint

## Open Questions (Proposal Phase)

*All resolved - see Decision section.*

## Next Steps (Proposal Phase)

- [ ] Prototype JSON Schema validation in holoconf-core (using a Rust JSON Schema library)
- [ ] Implement `$ref` resolution for schema composition
- [ ] Test two-phase validation with real-world configs
- [ ] Design error message format for validation failures

## Decision

**JSON Schema (YAML-serialized) with Two-Phase Validation**

- Schema format: JSON Schema (Draft 2020-12), written in YAML or JSON
- Schema composition: Support `$ref` for splitting schemas across files
- Validation timing: Two-phase (structural after merge, type/value after resolution)
- `additionalProperties`: Use JSON Schema default behavior (permissive unless explicitly set to `false`)
- Interpolation in schemas: Not explicitly supported or documented, but not disabled (schemas are parsed like any YAML file)
- Language-native type generation: Out of scope for core; may be added as language-specific tooling later

## Design

### Schema Format

Schemas use standard JSON Schema, but can be written in any format holoconf supports (YAML, JSON):

```yaml
# schema.yaml
type: object
required: [database, api]
properties:
  database:
    type: object
    description: Database connection settings
    required: [host]
    properties:
      host:
        type: string
        description: Database hostname
        examples: ["localhost", "db.example.com"]
      port:
        type: integer
        minimum: 1
        maximum: 65535
        default: 5432
      password:
        type: string
        description: Database password (typically from SSM resolver)
  api:
    type: object
    properties:
      timeout:
        type: number
        minimum: 0
        description: Request timeout in seconds
```

### Schema Composition with $ref

Large schemas can be split across files using JSON Schema's `$ref`:

```yaml
# schema.yaml (main schema)
type: object
properties:
  networking:
    $ref: "./schemas/networking.yaml"
  application:
    $ref: "./schemas/application.yaml"
  database:
    $ref: "./schemas/database.yaml"
```

```yaml
# schemas/networking.yaml
type: object
required: [vpc_id]
properties:
  vpc_id:
    type: string
    pattern: "^vpc-[a-f0-9]+$"
  subnets:
    type: array
    items:
      type: string
      pattern: "^subnet-[a-f0-9]+$"
```

```yaml
# schemas/database.yaml
type: object
required: [host]
properties:
  host:
    type: string
  port:
    type: integer
    default: 5432
```

Refs are resolved relative to the schema file's location.

### Two-Phase Validation

Validation happens in two phases to accommodate lazy resolution:

```
┌─────────────────────────────────────────────────────────────────┐
│  Config.load("base.yaml", "env.yaml", schema="schema.yaml")     │
│                                                                 │
│  1. Parse all config files                                      │
│  2. Merge configs (ADR-004)                                     │
│  3. PHASE 1: Structural validation                              │
│     - Required keys present (after merge)                       │
│     - Nesting structure correct                                 │
│     - No unknown keys (if additionalProperties: false)          │
│     - Interpolations like ${...} are allowed (not yet resolved) │
│                                                                 │
│  Returns Config object (or raises StructuralValidationError)    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  config.database.port  (access triggers resolution)             │
│                                                                 │
│  1. Resolve interpolation: ${env:DB_PORT} -> "5432"             │
│  2. PHASE 2: Type/value validation                              │
│     - Type check (is it an integer?)                            │
│     - Constraints (minimum: 1, maximum: 65535)                  │
│     - Pattern matching (if specified)                           │
│                                                                 │
│  Returns resolved value (or raises TypeValidationError)         │
└─────────────────────────────────────────────────────────────────┘
```

**Phase 1 (Structural)** - Runs after merge, before any resolution:
- Validates required keys are present
- Validates object/array nesting matches schema
- Validates `additionalProperties` constraints
- Treats `${...}` interpolations as valid placeholders (any type)

**Phase 2 (Type/Value)** - Runs after each value is resolved:
- Validates resolved value matches declared type
- Validates constraints (minimum, maximum, pattern, enum, etc.)
- Runs automatically when accessing values with lazy resolution

### API Surface

```python
# Load with schema (structural validation on load)
config = Config.load(
    "base.yaml", "environment.yaml",
    schema="schema.yaml"
)

# Access triggers resolution + type validation
port = config.database.port  # Validates integer, range

# Validate entire config explicitly (resolves all + validates all)
await config.resolve_all()
config.validate()  # Re-runs full validation on resolved values

# Load without schema, validate later
config = Config.load("config.yaml")
config.validate(schema="schema.yaml")  # Both phases at once
```

### Error Messages

Validation errors include path and context:

```
StructuralValidationError: Missing required key
  Path: database.host
  Schema: schema.yaml#/properties/database/required
  Help: Add 'host' key to database section

TypeValidationError: Invalid type after resolution
  Path: database.port
  Expected: integer
  Got: string ("not-a-number")
  Resolved from: ${env:DB_PORT}
  Schema: schema.yaml#/properties/database/properties/port
```

## Rationale

- **JSON Schema is an industry standard** with existing tooling, documentation, and developer familiarity
- **YAML serialization** makes schemas readable and consistent with config file format
- **Two-phase validation** accommodates lazy resolution while still catching structural errors early
- **Schema composition via `$ref`** enables teams to share and reuse schema definitions
- **Permissive by default** avoids surprising users with strict validation they didn't opt into

## Trade-offs Accepted

- **JSON Schema verbosity** in exchange for **standard format with existing tooling**
- **Two-phase validation adds complexity** in exchange for **correct handling of lazy resolution**
- **No language-native type generation in core** in exchange for **keeping core language-agnostic**

## Migration

N/A - This is a new feature.

## Consequences

- **Positive:** Clear config documentation, early error detection, consistent validation across languages
- **Negative:** Additional file to maintain (schema), learning curve for JSON Schema
- **Neutral:** Schema validation is optional - configs work without schemas

# TypeScript/JavaScript Patterns

> **Agent**: For TypeScript/JavaScript and NAPI-RS analysis, use the `typescript-expert` agent.

## Project Structure (Planned)

```
packages/node/holoconf/
├── src/
│   └── index.ts          # Re-exports from native bindings
├── native/               # NAPI-RS Rust bindings
├── package.json
└── tsconfig.json

crates/holoconf-node/     # Rust NAPI-RS crate
├── src/
│   └── lib.rs
└── Cargo.toml
```

## TypeScript Style

### Type Annotations
- Strict mode required (`"strict": true`)
- Explicit return types on public functions
- Use `unknown` over `any`
- Prefer `interface` for objects, `type` for unions

### Naming
- `camelCase` for functions, methods, variables
- `PascalCase` for classes and interfaces
- `SCREAMING_SNAKE_CASE` for constants

### JSDoc (for public APIs)
```typescript
/**
 * Resolve a configuration value by key.
 * @param key - The configuration path (e.g., "database.host")
 * @param defaultValue - Fallback value if key not found
 * @returns The resolved configuration value
 * @throws {PathNotFoundError} If key not found and no default provided
 */
get(key: string, defaultValue?: unknown): unknown
```

## NAPI-RS Specifics

- TypeScript definitions auto-generated from Rust `#[napi]` macros
- Keep Rust bindings in sync with Python bindings for API parity
- Test with actual JavaScript, not just Rust tests

## API Parity

The JavaScript API should match Python API semantics:

```typescript
// Loading
const config = Config.load("config.yaml");
const config = Config.loads(yamlString);

// Access
config.get("database.port");
config.getString("database.host");
config.getInt("database.port");
config.getBool("feature.enabled");

// Operations
config.merge(otherConfig);
config.resolveAll();
config.validate(schema);
```

## Linting & Formatting

```bash
# ESLint + Prettier (or Biome)
npm run lint
npm run format
```

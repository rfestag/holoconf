# ADR-011: Interpolation Syntax

## Status

- **Proposed by:** Ryan on 2026-01-07
- **Accepted on:** 2026-01-07

## Context

ADR-002 established the basic interpolation syntax (`${resolver:key}` for external, `${path}` for self-reference). However, several edge cases and advanced features need to be defined:

- How to escape interpolation syntax (literal `${...}`)
- Whether nested interpolations are supported
- Default/fallback values when resolution fails
- String concatenation with interpolations

These decisions affect the parser implementation in holoconf-core and user experience.

## Alternatives Considered

### Escaping

**Option A: Double dollar sign**
```yaml
literal: $${not_interpolated}  # Outputs: ${not_interpolated}
```
- **Rejected:** Not OmegaConf-compatible

**Option B: Backslash escape**
```yaml
literal: \${not_interpolated}  # Outputs: ${not_interpolated}
```
- **Chosen:** Matches OmegaConf

**Option C: No escaping (use quotes or different key)**
```yaml
literal: "${not_interpolated}"  # Still interpolated in most systems
```
- **Rejected:** Not practical

### Nested Interpolations

**Option A: Not supported**
```yaml
# Invalid - parse error
path: ${ssm:${env:SSM_PREFIX}/password}
```
- **Rejected:** Limits flexibility

**Option B: Supported (inside-out resolution)**
```yaml
# Valid - resolves env:SSM_PREFIX first, then ssm:
path: ${ssm:${env:SSM_PREFIX}/password}
# If SSM_PREFIX=/prod → resolves ${ssm:/prod/password}
```
- **Chosen:** Matches OmegaConf, enables dynamic key construction

### Default Values

**Option A: Colon syntax (shell-style)**
```yaml
port: ${env:PORT:8080}           # Use 8080 if PORT not set
host: ${env:HOST:localhost}      # Use localhost if HOST not set
```
- **Rejected:** Conflicts with resolver:key syntax

**Option B: Pipe syntax**
```yaml
port: ${env:PORT | 8080}
host: ${env:HOST | localhost}
```
- **Rejected:** Non-standard syntax

**Option C: Positional comma syntax (OmegaConf-style)**
```yaml
port: ${env:PORT,8080}           # Use 8080 if PORT not set
host: ${env:HOST,localhost}      # Use localhost if HOST not set
```
- **Rejected:** Ambiguous when default contains commas, requires each resolver to implement default handling

**Option D: Keyword argument syntax**
```yaml
port: ${env:PORT,default=8080}           # Use 8080 if PORT not set
host: ${env:HOST,default=localhost}      # Use localhost if HOST not set
```
- **Chosen:** Explicit, unambiguous, handled by framework (not individual resolvers)

**Option E: Not supported (fail if missing)**
```yaml
port: ${env:PORT}  # Error if PORT not set
```
- **Rejected:** Too inflexible

### String Concatenation

**Option A: Inline interpolation**
```yaml
url: "https://${host}:${port}/api"
connection: "postgresql://${db.user}:${db.password}@${db.host}/${db.name}"
```
- **Chosen:** Matches OmegaConf, intuitive syntax

**Option B: Explicit concat resolver**
```yaml
url: ${concat:https://,${host},:,${port},/api}
```
- **Rejected:** Verbose, not needed when inline interpolation works

## Open Questions (Proposal Phase)

*All resolved - see Decision section.*

## Next Steps (Proposal Phase)

- [ ] Implement parser in holoconf-core matching OmegaConf grammar
- [ ] Test edge cases (nested quotes, special characters, escaping)
- [ ] Validate compatibility with existing OmegaConf configs

## Decision

**OmegaConf-Inspired Interpolation Syntax with Framework-Level Keywords**

Align with OmegaConf syntax where practical, with improvements for clarity and consistency:

- Escaping: `\${...}` (backslash) to output literal `${...}`
- Nested interpolations: Supported, resolve inside-out
- Self-references: `${path.to.value}` (absolute) and `${..relative}` (relative)
- Resolver syntax: `${resolver:arg,key=value,...}` with positional args and keyword arguments
- Framework keywords: `default` and `sensitive` are handled by the resolver framework, not individual resolvers
- Limits: 10 nesting levels, 100 interpolations per value, 10,000 characters (safety limits)

## Design

### Syntax (OmegaConf-Compatible)

#### Basic Interpolation

```yaml
# Self-reference (absolute path from root)
timeout: ${defaults.timeout}

# Self-reference (relative path)
timeout: ${.sibling_key}
timeout: ${..parent.sibling}

# External resolver
password: ${ssm:/prod/db/password}
api_key: ${env:API_KEY}
```

#### Escaping

```yaml
# Backslash to escape (OmegaConf-compatible)
literal: \${this_is_not_interpolated}
# Resolves to: ${this_is_not_interpolated}

# Backslash before interpolation that should resolve
path: "C:\\${dir}"
# Resolves to: C:\<resolved value of dir>

# In strings
message: "Cost is \${price} dollars"
# Resolves to: Cost is ${price} dollars
```

#### Default Values

```yaml
# Keyword syntax for defaults (framework-handled)
port: ${env:PORT,default=8080}
host: ${env:HOST,default=localhost}
log_level: ${env:LOG_LEVEL,default=info}

# Default can be another interpolation
password: ${env:DB_PASSWORD,default=${ssm:/default/db/password}}

# Self-references also support default
timeout: ${settings.timeout,default=30}

# Cascading defaults with nested interpolations
port: ${env:PORT,default=${env:DEFAULT_PORT,default=8080}}
```

#### String Concatenation

```yaml
# Interpolations within strings
database_url: "postgresql://${db.host}:${db.port}/${db.name}"
api_endpoint: "https://${api.host}/v${api.version}/users"
log_prefix: "[${app.name}] "
```

#### Nested Interpolations

```yaml
# Nested interpolations resolve inside-out
# Step 1: ${env:ENV} → "prod"
# Step 2: ${ssm:/prod/db/password} → "secret123"
password: ${ssm:/${env:ENV}/db/password}

# More complex nesting
# Step 1: ${env:REGION} → "us-east-1"
# Step 2: ${env:ACCOUNT} → "123456"
# Step 3: ${ssm:/us-east-1/123456/api-key} → "key123"
api_key: ${ssm:/${env:REGION}/${env:ACCOUNT}/api-key}
```

### Framework-Level Keyword Arguments

The resolver framework handles two special keyword arguments that apply to all resolvers uniformly. Individual resolvers do not implement these—they are extracted and processed by the framework before and after calling the resolver.

#### `default` Keyword

Provides a fallback value when the resolver cannot find the requested resource (e.g., missing env var, file not found, SSM parameter doesn't exist).

```yaml
# Framework extracts default=, calls resolver, uses default if resolver returns NotFound
port: ${env:PORT,default=8080}
config: ${file:./optional.yaml,default={}}
timeout: ${ssm:/app/timeout,default=30}
```

**Behavior:**
1. Framework parses the interpolation and extracts `default=` if present
2. Framework calls the resolver with remaining arguments
3. If resolver returns `NotFound` error and default is set, framework returns default value
4. If resolver returns other errors (e.g., permission denied, network error), error propagates (default not used)
5. If resolver succeeds, default is ignored

#### `sensitive` Keyword

Marks the resolved value as sensitive for redaction during serialization (see ADR-009).

```yaml
# Mark value as sensitive (affects serialization only)
api_key: ${env:API_KEY,sensitive=true}
password: ${file:./secret.key,sensitive=true}

# Override resolver's default sensitivity
non_secret_param: ${ssm:/app/public-config,sensitive=false}
```

**Behavior:**
1. Framework parses the interpolation and extracts `sensitive=` if present
2. Framework calls the resolver, which may return a sensitivity hint (e.g., SSM SecureString → sensitive)
3. If user specified `sensitive=`, that value overrides the resolver's hint
4. If user did not specify, resolver's hint is used (or `false` if resolver provides no hint)
5. Sensitivity only affects serialization (`to_yaml(redact=True)`), not value access

#### Sensitivity Inheritance

For self-references, sensitivity is inherited from the referenced value by default:

```yaml
secrets:
  api_key: ${env:API_KEY,sensitive=true}

# Inherits sensitive=true from the referenced value
derived: ${secrets.api_key}

# Can override if needed (rare)
public_copy: ${secrets.api_key,sensitive=false}
```

#### Framework Resolution Flow

```
┌─────────────────────────────────────────────────────────────┐
│  Interpolation: ${resolver:arg,default=X,sensitive=Y,opt=Z} │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Parse and extract framework keywords                    │
│     - default = X                                           │
│     - sensitive = Y                                         │
│     - remaining kwargs = {opt: Z}                           │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  2. Call resolver.resolve(arg, opt=Z)                       │
│     - Resolver only sees its own arguments                  │
│     - Returns ResolvedValue(value, sensitive_hint)          │
│       OR NotFound error                                     │
└─────────────────────────────────────────────────────────────┘
                            │
              ┌─────────────┴─────────────┐
              ▼                           ▼
┌──────────────────────┐    ┌──────────────────────────────┐
│  NotFound + default  │    │  Success or other error      │
│  → Use default value │    │  → Use resolver's value/error│
└──────────────────────┘    └──────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  3. Apply sensitivity override                              │
│     - If user specified sensitive=Y, use Y                  │
│     - Else use resolver's hint (or false)                   │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  4. Return final ResolvedValue(value, is_sensitive)         │
└─────────────────────────────────────────────────────────────┘
```

### Grammar

```
interpolation  = "${" expression "}"
expression     = resolver_ref | self_ref
resolver_ref   = resolver_name ":" args
self_ref       = path ("," kwargs)?
path           = relative_path | absolute_path
relative_path  = "."+ path_segment ("." path_segment)*
absolute_path  = path_segment ("." path_segment)*
path_segment   = identifier | "[" index "]"
args           = positional_arg* ("," kwarg)*
positional_arg = value
kwarg          = identifier "=" value
value          = quoted_string | unquoted_value | interpolation
quoted_string  = "'" [^']* "'" | '"' [^"]* '"'
unquoted_value = [^,=}]+                    # Until comma, equals, or closing brace
escape         = "\${"                      # Produces literal "${"
kwargs         = kwarg ("," kwarg)*         # For self-references with options
```

**Reserved Keywords (handled by framework):**
- `default` - Fallback value if resolver returns NotFound
- `sensitive` - Override sensitivity for redaction

### Edge Cases

| Input | Output | Notes |
|-------|--------|-------|
| `${env:PORT}` | `"8080"` | Basic resolver |
| `${env:PORT,default=3000}` | `"8080"` or `"3000"` | With default keyword |
| `${env:API_KEY,sensitive=true}` | Value, marked sensitive | Sensitivity override |
| `\${env:PORT}` | `"${env:PORT}"` | Escaped (backslash) |
| `"http://${host}"` | `"http://localhost"` | String interpolation |
| `${ssm:/${env:E}/k}` | Resolved value | Nested interpolation |
| `${a.b.c}` | Value at path | Self-reference |
| `${.sibling}` | Value at sibling | Relative reference |
| `${a.b,default=30}` | Value or `30` | Self-reference with default |
| `${env:X,default=${env:Y,default=z}}` | Cascading defaults | Nested defaults |

### Resolution Order

For nested interpolations, resolution happens inside-out:

```yaml
value: ${ssm:/${env:PREFIX}/key}

# Resolution steps:
# 1. Parse outer interpolation: ssm:/${env:PREFIX}/key
# 2. Find nested interpolation: ${env:PREFIX}
# 3. Resolve inner first: env:PREFIX → "prod"
# 4. Substitute: ssm:/prod/key
# 5. Resolve outer: ssm:/prod/key → "secret_value"
# 6. Final value: "secret_value"
```

### Limits

To prevent abuse and ensure predictable behavior:

- **Max nesting depth:** 10 levels
- **Max interpolations per value:** 100
- **Max interpolation length:** 10,000 characters

Exceeding limits raises `InterpolationError`.

## Rationale

- **OmegaConf-inspired syntax** provides familiarity for users of similar tools
- **Backslash escaping** matches OmegaConf and is familiar from other languages
- **Keyword syntax for `default`** is explicit and avoids ambiguity with comma-containing values
- **Framework-level keywords** ensure consistent behavior across all resolvers without code duplication
- **Nested interpolations** enable dynamic key construction, a powerful feature
- **Safety limits** prevent runaway parsing without breaking normal use cases
- **Sensitivity as framework concern** keeps resolver implementations simple (they just return values with hints)

## Trade-offs Accepted

- **Keyword syntax differs from OmegaConf's positional defaults** in exchange for **clarity and framework-level handling**
- **Backslash escaping in YAML** can be tricky (YAML also uses backslash), in exchange for **OmegaConf compatibility**
- **Resolvers cannot override framework keywords** in exchange for **consistent, predictable behavior**

## Migration

**From OmegaConf:** Most configs require minor changes for defaults:
- holoconf uses `${env:VAR,default=value}` while OmegaConf uses `${oc.env:VAR,value}` (positional)
- holoconf uses `${env:VAR}` while OmegaConf uses `${oc.env:VAR}` - we provide `env` directly
- holoconf uses `${ssm:...}` for AWS SSM - new resolver, not in OmegaConf

**Migration script:** A simple regex can convert OmegaConf positional defaults to keyword syntax:
```
${env:(\w+),([^}]+)} → ${env:$1,default=$2}
```

## Consequences

- **Positive:** Consistent `default` and `sensitive` behavior across all resolvers without code duplication
- **Positive:** Clear, explicit syntax that avoids ambiguity
- **Positive:** Resolvers are simpler (don't need to implement default/sensitive handling)
- **Negative:** Not 100% compatible with OmegaConf positional default syntax
- **Neutral:** Safety limits may need tuning based on real-world usage

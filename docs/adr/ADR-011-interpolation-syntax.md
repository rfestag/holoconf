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
- **Rejected:** Not OmegaConf-compatible, conflicts with resolver:key syntax

**Option B: Pipe syntax**
```yaml
port: ${env:PORT | 8080}
host: ${env:HOST | localhost}
```
- **Rejected:** Not OmegaConf-compatible

**Option C: Comma syntax (OmegaConf-style)**
```yaml
port: ${env:PORT,8080}           # Use 8080 if PORT not set
host: ${env:HOST,localhost}      # Use localhost if HOST not set
```
- **Chosen:** Matches OmegaConf's `oc.env` and `oc.select` resolvers

**Option D: Not supported (fail if missing)**
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

**OmegaConf-Compatible Interpolation Syntax**

Align with OmegaConf syntax for maximum compatibility with existing configs:

- Escaping: `\${...}` (backslash) to output literal `${...}`
- Nested interpolations: Supported, resolve inside-out
- Default values: Resolver-specific via comma syntax (e.g., `${env:VAR,default}`)
- Self-references: `${path.to.value}` (absolute) and `${..relative}` (relative)
- Resolver syntax: `${resolver:arg1,arg2,...}` with comma-separated arguments
- Limits: 10 nesting levels, 100 interpolations per value, 10,000 characters (safety limits, not in OmegaConf)

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
# Comma syntax for defaults (OmegaConf-compatible)
# Defaults are resolver arguments, handled by each resolver
port: ${env:PORT,8080}
host: ${env:HOST,localhost}
log_level: ${env:LOG_LEVEL,info}

# Default can be another interpolation
password: ${env:DB_PASSWORD,${ssm:/default/db/password}}

# For self-references, use oc.select-style resolver (or holoconf equivalent)
timeout: ${select:settings.timeout,30}
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

### Grammar (OmegaConf-Compatible)

```
interpolation  = "${" expression "}"
expression     = resolver_ref | self_ref
resolver_ref   = resolver_name ":" args
self_ref       = relative_path | absolute_path
relative_path  = "."+ path_segment ("." path_segment)*
absolute_path  = path_segment ("." path_segment)*
path_segment   = identifier | "[" index "]"
args           = arg ("," arg)*             # Comma-separated arguments
arg            = quoted_string | unquoted_value | interpolation
quoted_string  = "'" [^']* "'" | '"' [^"]* '"'
unquoted_value = [^,}]+                     # Until comma or closing brace
escape         = "\${"                      # Produces literal "${"
```

### Edge Cases

| Input | Output | Notes |
|-------|--------|-------|
| `${env:PORT}` | `"8080"` | Basic resolver |
| `${env:PORT,3000}` | `"8080"` or `"3000"` | With default (comma syntax) |
| `\${env:PORT}` | `"${env:PORT}"` | Escaped (backslash) |
| `"http://${host}"` | `"http://localhost"` | String interpolation |
| `${ssm:/${env:E}/k}` | Resolved value | Nested interpolation |
| `${a.b.c}` | Value at path | Self-reference |
| `${.sibling}` | Value at sibling | Relative reference |
| `${env:X,${env:Y,default}}` | Cascading defaults | Nested defaults |

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

- **OmegaConf compatibility** enables migration of existing configs without changes
- **Backslash escaping** matches OmegaConf and is familiar from other languages
- **Comma syntax for defaults** keeps defaults as resolver arguments, matching OmegaConf's `oc.env` and `oc.select`
- **Nested interpolations** enable dynamic key construction, a powerful OmegaConf feature
- **Safety limits** prevent runaway parsing without breaking normal use cases

## Trade-offs Accepted

- **Comma syntax for defaults** means resolvers must handle defaults individually, in exchange for **OmegaConf compatibility**
- **Backslash escaping in YAML** can be tricky (YAML also uses backslash), in exchange for **OmegaConf compatibility**
- **No universal default syntax** in exchange for **resolver flexibility** (each resolver decides how to handle its arguments)

## Migration

**From OmegaConf:** Most configs should work without changes. Key differences:
- holoconf uses `${env:VAR}` while OmegaConf uses `${oc.env:VAR}` - we'll provide `env` as an alias
- holoconf uses `${ssm:...}` for AWS SSM - new resolver, not in OmegaConf

## Consequences

- **Positive:** Existing OmegaConf configs work with minimal changes, familiar syntax for OmegaConf users
- **Negative:** Must maintain OmegaConf grammar compatibility as OmegaConf evolves
- **Neutral:** Safety limits may need tuning based on real-world usage

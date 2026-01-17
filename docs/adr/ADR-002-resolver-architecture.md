# ADR-002: Resolver Architecture

## Status

- **Proposed by:** Ryan on 2026-01-07
- **Accepted on:** 2026-01-07

## Context

holoconf needs to resolve configuration values from various sources:

- Environment variables
- Self-references (other config values)
- External services: AWS SSM, S3, CloudFormation outputs
- Future: Vault, GCP Secret Manager, Azure Key Vault, etc.

We need an architecture that:

- Provides consistent resolver behavior across languages
- Allows users to write custom resolvers without learning Rust
- Keeps binary size manageable (don't bundle unused resolvers)

## Alternatives Considered

### Alternative 1: All Native Resolvers (no Rust resolver packages)

Implement all resolvers in each language's native SDK.

- **Pros:** Full native SDK access, no FFI for resolvers
- **Cons:** Must reimplement SSM/S3/etc. for each language, inconsistent behavior risk
- **Rejected:** Defeats purpose of Rust core

### Alternative 2: Rust-Only Resolvers (no native callbacks)

All resolvers must be written in Rust.

- **Pros:** Maximum consistency, simpler FFI
- **Cons:** Users must write Rust to add custom resolvers, high barrier
- **Rejected:** Too restrictive for users

### Alternative 3: Binary Plugin System (dynamic library loading)

Resolvers distributed as platform-specific binary plugins (`.so`, `.dll`, `.dylib`) that holoconf-core discovers and loads at runtime from a plugin directory.

- **Pros:** Language-agnostic plugins, maximum isolation
- **Cons:** Complex ABI stability requirements, platform-specific binaries, security concerns with loading arbitrary shared libraries, distribution complexity
- **Rejected:** Over-engineering; the native callback mechanism provides plugin-like extensibility without binary distribution challenges

## Decision

**Hybrid - Rust Resolver Packages + Native Custom Resolver Callbacks**

**Built-in Resolvers (Rust packages):**

- `holoconf-core`: env, self-reference resolvers
- `holoconf-aws`: SSM, S3, CloudFormation (uses aws-sdk-rust)
- `holoconf-gcp`: Secret Manager, GCS (future)
- `holoconf-vault`: HashiCorp Vault (future)

**Custom Resolvers (native language callbacks):**

- Users can register custom resolvers in their native language
- Rust core calls back to native code for custom resolver execution

## Design

```
┌─────────────────────────────────────────────────────┐
│                 holoconf-core (Rust)                │
│  ┌─────────────────────────────────────────────┐   │
│  │          Resolver Registry                   │   │
│  │  ┌─────────┐ ┌─────────┐ ┌───────────────┐  │   │
│  │  │  env    │ │  self   │ │ custom (FFI)  │  │   │
│  │  └─────────┘ └─────────┘ └───────────────┘  │   │
│  └─────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
         │                              ▲
         │ links to                     │ callback
         ▼                              │
┌─────────────────┐            ┌────────────────────┐
│  holoconf-aws   │            │  Native Custom     │
│  (Rust crate)   │            │  Resolver          │
│  - SSM          │            │  (Python/JS/etc)   │
│  - S3           │            │                    │
│  - CloudForm.   │            │  class MyResolver: │
│  (aws-sdk-rust) │            │    def resolve()   │
└─────────────────┘            └────────────────────┘
```

### Resolver Syntax (OmegaConf-style)

Two reference types:

1. **Absolute references** - from document root: `${path.to.value}`
2. **Relative references** - from current node: `${.sibling}` or `${..parent.sibling}`

Resolver prefix is optional for self-references, required for external resolvers.

```yaml
database:
  host: ${env:DB_HOST}           # env resolver (external)
  password: ${ssm:/prod/db/password}  # ssm resolver (external)
  port: ${defaults.port}         # absolute reference (from root)
  timeout: ${.defaults.timeout}  # relative reference (sibling)

defaults:
  port: 5432
  timeout: 30

nested:
  level1:
    level2:
      value: ${..level1_sibling}     # relative: go up one level
      root_ref: ${defaults.port}      # absolute: from root
    level1_sibling: "hello"
```

### Custom Resolver Interface

Resolvers support two forms:

#### Simple Function Form

For resolvers where all values have the same sensitivity (or none). Returns the value directly.

```python
# Sync function resolver - values not marked sensitive
def my_resolver(key: str) -> str:
    return lookup_value(key)

holoconf.register("myresolver", my_resolver)

# Async function resolver
async def secret_resolver(key: str) -> str:
    return await fetch_secret(key)

holoconf.register("secret", secret_resolver)
```

```javascript
// JavaScript - async function resolver
async function secretResolver(key) {
    const response = await fetch(`https://secrets.internal/${key}`);
    return await response.text();
}

holoconf.register("secret", secretResolver);
```

#### Class Form (with Sensitivity Metadata)

For resolvers that need to report per-value sensitivity for redaction (see ADR-009). Returns a `ResolvedValue` with metadata.

```python
from holoconf import Resolver, ResolvedValue

class SSMResolver(Resolver):
    def resolve(self, key: str) -> ResolvedValue:
        param = self.client.get_parameter(Name=key, WithDecryption=True)
        return ResolvedValue(
            value=param["Parameter"]["Value"],
            sensitive=(param["Parameter"]["Type"] == "SecureString")
        )

holoconf.register("ssm", SSMResolver())
```

```javascript
// JavaScript class form
class VaultResolver extends Resolver {
    async resolve(key) {
        const secret = await this.client.read(key);
        return new ResolvedValue({
            value: secret.data,
            sensitive: true  // All Vault values are secrets
        });
    }
}

holoconf.register("vault", new VaultResolver());
```

The core wraps simple functions internally, defaulting `sensitive=False`. The class form enables resolver-aware redaction during serialization.

Async resolvers enable parallel resolution internally, but access appears synchronous - the value blocks until resolved. For explicit async access, use `resolve_all()` to resolve all values in parallel.

> **Note:** `load_async()` was considered but not implemented for file loading. Async
> file I/O provides minimal benefit for small local config files. The async value is
> in resolver execution (SSM, HTTP, etc.), not file loading.

### Resolver Return Types

Resolvers can return scalars or complex types:

| Resolver returns | You get back |
|------------------|--------------|
| `str`, `int`, `float`, `bool` | Native scalar (copied) |
| `dict` | Config wrapper (dot notation preserved) |
| `list` | Native list (copied) |

```python
# Resolver returning complex type
def json_resolver(key: str) -> dict:
    return {"nested": {"value": 123}, "items": [1, 2, 3]}

# Access with dot notation
config.some_data.nested.value  # 123
config.some_data.items[0]      # 1
```

### Resolver Package Registration

```python
# Register all resolvers from a package (uses defaults / env vars)
holoconf.register(holoconf_aws)

# Register with options
holoconf.register(holoconf_aws, {"region": "us-east-1", "endpoint": "http://localhost:4566"})

# Register specific resolvers
holoconf.register(holoconf_aws.ssm, holoconf_aws.s3)
```

Resolver packages expose a standard interface (e.g., `__holoconf_resolvers__` dict) for discovery. Built-in resolvers use standard credential/config chains by default (env vars, ~/.aws/config) but accept explicit overrides via options.

### Caching

Resolved values are memoized per-config-instance (see ADR-005). Resolvers themselves don't cache - that's the Config object's responsibility.

## Rationale

- **Rust resolver packages ensure SSM/S3/etc. behave identically** across all languages
- **Native callbacks allow users to write custom resolvers** without Rust knowledge
- **Separate packages (holoconf-aws) keep core binary small** - only pay for what you use
- **aws-sdk-rust is official and well-maintained** - reliable foundation

## Trade-offs Accepted

- **Rust AWS SDK may have different defaults** than boto3/@aws-sdk (credential chain, retries) in exchange for **cross-language consistency**
- **Custom resolver callbacks cross FFI boundary** (slight overhead) in exchange for **native language ergonomics**
- **Users wanting Rust-level performance for custom resolvers must write Rust** in exchange for **lower barrier for simple resolvers**

## Migration

N/A - This is the initial architecture decision.

## Consequences

- **Positive:** Consistent AWS/GCP/Vault behavior across languages, users can write custom resolvers in their preferred language
- **Negative:** Built-in resolvers may not expose all native SDK features
- **Neutral:** Two paths for resolver implementation (Rust for built-ins, native for custom)

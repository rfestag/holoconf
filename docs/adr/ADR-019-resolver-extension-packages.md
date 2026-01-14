# ADR-019: Resolver Extension Packages

## Status

Accepted

## Context

As holoconf grows, users need ways to add custom resolvers for their specific infrastructure (AWS SSM, HashiCorp Vault, Azure Key Vault, etc.). We need a pattern for creating extension packages that:

1. Can be published and installed independently
2. Work seamlessly with the core holoconf library
3. Support both Rust and Python implementations
4. Allow automatic discovery and registration

## Decision

### Global Resolver Registry

We use a global resolver registry pattern where resolvers can be registered once and used by all Config instances:

**Rust:**
```rust
use holoconf_core::resolver::{register_global, Resolver};
use std::sync::Arc;

let resolver = Arc::new(MyResolver::new());
register_global(resolver, force: bool)?;
```

**Python:**
```python
import holoconf

def my_resolver(arg, **kwargs):
    return value

holoconf.register_resolver("my", my_resolver)
```

### Extension Package Pattern

#### Rust Extension Crates

Rust extension crates (e.g., `holoconf-aws`) should:

1. Depend on `holoconf-core` for resolver traits
2. Implement `Resolver` trait for each resolver
3. Provide a `register()` function to register all resolvers
4. Use `register_global(resolver, force=true)` for registration

Example structure:
```
holoconf-aws/
├── Cargo.toml
├── src/
│   ├── lib.rs          # register_all(), re-exports
│   └── ssm.rs          # SsmResolver implementation
```

#### Python Extension Packages

Python extension packages (e.g., `holoconf-aws`) should:

1. Depend on `holoconf` for the `register_resolver` function
2. Implement resolver callables (functions or classes with `__call__`)
3. Auto-register on import for convenience
4. Define entry points for plugin discovery

Example structure:
```
holoconf-aws/
├── pyproject.toml
├── src/holoconf_aws/
│   ├── __init__.py     # Auto-registration on import
│   └── ssm.py          # SsmResolver class
```

### Plugin Discovery

Python packages can declare entry points in `pyproject.toml`:

```toml
[project.entry-points."holoconf.resolvers"]
ssm = "holoconf_aws:register_ssm"
```

Users can discover and load all plugins with:

```python
import holoconf

# Discover and load all installed resolver plugins
loaded = holoconf.discover_plugins()
print(f"Loaded plugins: {loaded}")  # ['ssm']
```

### Error Handling for Custom Resolvers

Custom resolvers should follow these patterns for proper framework integration:

1. **Raise `KeyError` for "not found" conditions** - This triggers framework-level default handling
2. **Raise `ValueError` for validation errors** - This becomes a resolver error
3. **Return `ResolvedValue(value, sensitive=True)` for secrets** - This enables redaction

### Framework-Level Kwargs

The framework handles these kwargs uniformly for all resolvers:

- `default`: Value to use if resolver raises KeyError
- `sensitive`: Override automatic sensitivity detection

Resolvers should NOT handle these kwargs themselves.

## Consequences

### Positive

- Clean separation between core library and extensions
- Extensions can be published independently
- Plugin discovery enables zero-config usage
- Consistent patterns for both Rust and Python
- Framework handles cross-cutting concerns (defaults, sensitivity)

### Negative

- Global state (registry) complicates testing
- Need to document KeyError convention for default handling
- Two implementations (Rust and Python) for each resolver

### Mitigations

- Tests can use `register_global(resolver, force=true)` to override
- Clear documentation of error handling conventions
- Python packages can use boto3 directly (simpler than Rust async)

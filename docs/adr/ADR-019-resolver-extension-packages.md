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

Python extension packages (e.g., `holoconf-aws`) can be implemented in two ways:

**Option A: PyO3 bindings to Rust (recommended)**

Use PyO3 to expose Rust resolvers to Python. This ensures consistent behavior across both languages and avoids code duplication.

```
holoconf-aws/
├── pyproject.toml          # maturin build, entry points
├── src/holoconf_aws/
│   ├── __init__.py         # Re-exports from Rust bindings
│   └── _holoconf_aws.pyi   # Type stubs
```

The Rust implementation lives in a separate PyO3 crate (e.g., `holoconf-aws-python`) that depends on the Rust resolver crate.

**Option B: Pure Python**

For simpler resolvers or when Rust isn't needed:

```
holoconf-aws/
├── pyproject.toml
├── src/holoconf_aws/
│   ├── __init__.py     # Registration functions
│   └── resolver.py     # Resolver implementation
```

### Plugin Discovery

Python packages declare entry points in `pyproject.toml`:

```toml
[project.entry-points."holoconf.resolvers"]
ssm = "holoconf_aws:register_ssm"
```

**Plugins are automatically discovered and registered when holoconf is imported.** No explicit import or registration is needed:

```python
import holoconf  # Auto-discovers all installed plugins

# SSM resolver is already available (if holoconf-aws is installed)
config = holoconf.Config.loads("secret: ${ssm:/app/password}")
```

The `discover_plugins()` function is called automatically at import time. It can also be called manually to re-discover plugins if new ones are installed at runtime:

```python
loaded = holoconf.discover_plugins()
print(f"Loaded plugins: {loaded}")  # ['ssm']
```

If a plugin fails to load, a warning is logged (but does not raise an exception).

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
- PyO3 bindings allow Python to use the same Rust implementation

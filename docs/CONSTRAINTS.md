# holoconf Constraints

This document captures constraints that must be respected when building holoconf.

## Design Constraints

### C-1: OmegaConf-Style Resolver Syntax

Must use OmegaConf-compatible interpolation syntax.

- Self-references: `${path.to.value}` (absolute) or `${.relative.path}` (relative)
- External resolvers: `${resolver:key}`
- Examples: `${env:DB_HOST}`, `${ssm:/prod/password}`, `${defaults.port}`

**Rationale:** Familiar to OmegaConf users, clear distinction between self-refs and external resolvers.

### C-2: Native Custom Resolvers

Custom resolvers must be implementable in the user's native language (Python, JavaScript, etc.) without requiring Rust knowledge.

**Rationale:** Low barrier for users to extend holoconf for their specific needs.

### C-3: Cross-Language Behavioral Parity

All language implementations must behave identically for the same input.

- Same config files must produce same merged output
- Same resolver syntax must resolve identically
- Conformance test suite validates parity

**Rationale:** Users should be able to switch languages without surprises.

### C-4: Language-Agnostic Configuration Format

Configuration files must be language-agnostic (YAML/JSON).

- No language-specific constructs in config files
- Same config files work across all supported languages

**Rationale:** Config files are often shared across services in different languages.

## Technical Constraints

### C-5: Rust Core Implementation

Core logic must be implemented in Rust with FFI bindings.

**Rationale:** ADR-001 decision - ensures consistency and scales to many languages.

### C-6: Async-First Internal Architecture

Internal architecture must support async execution for parallel resolver calls.

**Rationale:** ADR-003 decision - required for performance with multiple external resolvers.

### C-7: Lazy Resolution

Resolver execution must be lazy (on-access) not eager (on-parse).

**Rationale:** ADR-005 decision - avoids wasted work, enables fast startup.

## Operational Constraints

### C-8: No Secrets in Config Files

Config files should reference secrets via resolvers, not contain them directly.

- Use `${ssm:/path}` or `${env:SECRET}` for secrets
- Config files can be committed to version control

**Rationale:** Security best practice, enables GitOps workflows.

### C-9: Deterministic Merging

Config merging must be deterministic given the same inputs in the same order.

**Rationale:** Reproducible deployments require predictable configuration.

## Constraint Traceability

| Constraint | Related ADR |
|------------|-------------|
| C-1 | [ADR-002](adr/ADR-002-resolver-architecture.md) |
| C-2 | [ADR-002](adr/ADR-002-resolver-architecture.md) |
| C-3 | [ADR-001](adr/ADR-001-multi-language-architecture.md) |
| C-4 | - |
| C-5 | [ADR-001](adr/ADR-001-multi-language-architecture.md) |
| C-6 | [ADR-003](adr/ADR-003-async-execution-model.md) |
| C-7 | [ADR-005](adr/ADR-005-resolver-timing.md) |
| C-8 | - |
| C-9 | [ADR-004](adr/ADR-004-config-merging.md) |

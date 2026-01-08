# holoconf Requirements

This document captures the core requirements that drive holoconf development.

## Functional Requirements

### R-1: Cross-Language Compatibility

holoconf must support multiple programming languages with consistent behavior.

- **R-1.1:** Python support (initial)
- **R-1.2:** JavaScript/Node.js support (initial)
- **R-1.3:** Go support (future)
- **R-1.4:** Java support (future)
- **R-1.5:** Rust support (future)
- **R-1.6:** C support (future)

### R-2: Hierarchical Configuration Merging

holoconf must support loading and merging multiple configuration files in a defined hierarchy.

- **R-2.1:** Load multiple config files in priority order
- **R-2.2:** Deep merge dictionaries/objects
- **R-2.3:** Last-writer-wins for scalar values
- **R-2.4:** List replacement on merge
- **R-2.5:** Null values remove keys from merged result

### R-3: Config Interpolation/Resolution

holoconf must support referencing other configuration values within the config.

- **R-3.1:** Absolute references from document root: `${path.to.value}`
- **R-3.2:** Relative references from current node: `${.sibling}`, `${..parent}`
- **R-3.3:** Lazy resolution (resolve on access, not parse)
- **R-3.4:** Memoization (resolve each key exactly once)

### R-4: Custom Resolvers

holoconf must support resolving values from external sources.

- **R-4.1:** Built-in `env` resolver for environment variables
- **R-4.2:** Built-in self-reference resolver
- **R-4.3:** AWS resolvers: SSM, S3, CloudFormation outputs
- **R-4.4:** User-defined custom resolvers in native language
- **R-4.5:** Resolver syntax: `${resolver:key}`

### R-5: YAML Configuration File Support

holoconf must support YAML as the primary configuration format.

- **R-5.1:** Parse standard YAML files
- **R-5.2:** Support interpolation syntax within YAML values

### R-6: Hierarchical Deployment Configuration

holoconf must support partition/region/account/deployment level config hierarchies.

- **R-6.1:** Define config loading order/hierarchy
- **R-6.2:** Override lower-priority configs with higher-priority ones
- **R-6.3:** Support environment-specific configurations

## Non-Functional Requirements

### NFR-1: Performance

- **NFR-1.1:** Fast startup (no resolver calls at parse time)
- **NFR-1.2:** Parallel resolver execution for multiple external references
- **NFR-1.3:** Suitable for AWS Lambda cold start scenarios

### NFR-2: Platform Support

- **NFR-2.1:** x86_64 architecture support
- **NFR-2.2:** aarch64 (ARM64) architecture support

### NFR-3: API Ergonomics

- **NFR-3.1:** Both sync and async APIs
- **NFR-3.2:** Language-idiomatic bindings
- **NFR-3.3:** Clear error messages

## Requirement Traceability

| Requirement | ADR | Feature Spec |
|-------------|-----|--------------|
| R-1 | [ADR-001](adr/ADR-001-multi-language-architecture.md) | TBD |
| R-2 | [ADR-004](adr/ADR-004-config-merging.md) | TBD |
| R-3 | [ADR-002](adr/ADR-002-resolver-architecture.md), [ADR-005](adr/ADR-005-resolver-timing.md) | TBD |
| R-4 | [ADR-002](adr/ADR-002-resolver-architecture.md) | TBD |
| R-5 | TBD (ADR-006 future) | TBD |
| R-6 | [ADR-004](adr/ADR-004-config-merging.md) | TBD |
| NFR-1 | [ADR-003](adr/ADR-003-async-execution-model.md), [ADR-005](adr/ADR-005-resolver-timing.md) | TBD |
| NFR-2 | [ADR-001](adr/ADR-001-multi-language-architecture.md) | TBD |
| NFR-3 | [ADR-001](adr/ADR-001-multi-language-architecture.md), [ADR-003](adr/ADR-003-async-execution-model.md) | TBD |

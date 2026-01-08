# Feature Specifications

This directory contains feature specifications for holoconf. Each spec defines a user-facing capability built on top of the architectural decisions in `/docs/adr/`.

## Feature Spec Format

Each feature spec includes:
- **Overview**: What the feature does
- **User Stories**: Who needs this and why
- **API Surface**: How users interact with the feature
- **Behavior**: Detailed behavior specification
- **Error Cases**: What can go wrong and how it's handled
- **Examples**: Concrete usage examples
- **Dependencies**: Which ADRs and other features this depends on

## Index

| Feature | Description | Status |
|---------|-------------|--------|
| [FEAT-001](FEAT-001-config-loading.md) | Configuration File Loading | Draft |
| [FEAT-002](FEAT-002-core-resolvers.md) | Core Resolvers (env, self, file) | Draft |
| [FEAT-003](FEAT-003-config-merging.md) | Configuration Merging | Draft |
| [FEAT-004](FEAT-004-schema-validation.md) | Schema Validation | Draft |
| [FEAT-005](FEAT-005-serialization.md) | Serialization and Export | Draft |
| [FEAT-006](FEAT-006-cli.md) | Command Line Interface | Draft |

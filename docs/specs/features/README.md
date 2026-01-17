# Feature Specifications

This directory contains feature specifications for HoloConf. Each spec defines a user-facing capability built on top of the architectural decisions in [ADRs](../../adr/README.md).

## What is a Feature Spec?

A feature spec is a document that defines the expected behavior of a user-facing capability. Feature specs help us:

- Define clear acceptance criteria before implementation
- Ensure consistent behavior across language bindings
- Provide a reference for writing acceptance tests
- Document the public API surface

## Feature Status

- **Draft** - Initial specification, under development
- **Review** - Ready for review, may change
- **Accepted** - Approved and ready for implementation
- **Implemented** - Fully implemented and tested

## Features

<div class="searchable-table" data-page-size="10" markdown>

| Feature | Description | Status |
|---------|-------------|--------|
| [FEAT-001](FEAT-001-config-loading.md) | Configuration File Loading | Implemented |
| [FEAT-002](FEAT-002-core-resolvers.md) | Core Resolvers (env, self, file, http) | Implemented |
| [FEAT-003](FEAT-003-config-merging.md) | Configuration Merging | Implemented |
| [FEAT-004](FEAT-004-schema-validation.md) | Schema Validation | Implemented |
| [FEAT-005](FEAT-005-serialization.md) | Serialization and Export | Implemented |
| [FEAT-006](FEAT-006-cli.md) | Command Line Interface | Implemented |
| [FEAT-007](FEAT-007-aws-resolvers.md) | AWS Resolvers (SSM, CloudFormation, S3) | Implemented |

</div>

## Creating a New Feature Spec

1. Copy `template.md` to `FEAT-NNN-short-title.md`
2. Fill in all sections
3. Submit for review
4. Update this index

## Template

See [template.md](template.md) for the feature spec template.

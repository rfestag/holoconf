# Contributing to HoloConf

Thank you for your interest in contributing to HoloConf! This section provides guidelines for contributors.

## Quick Links

- [Development Setup](development.md) - Get your environment ready
- [Testing](testing.md) - Testing guidelines and acceptance tests
- [Architecture Decisions](../adr/README.md) - ADRs documenting design decisions
- [Feature Specs](../specs/features/README.md) - Detailed feature specifications

## Ways to Contribute

### Bug Reports

Found a bug? [Open an issue](https://github.com/rfestag/holoconf/issues/new) with:

- A clear description of the problem
- Steps to reproduce
- Expected vs actual behavior
- Your environment (OS, Python/Rust version, HoloConf version)

### Feature Requests

Have an idea? [Open an issue](https://github.com/rfestag/holoconf/issues/new) describing:

- The use case or problem you're trying to solve
- Your proposed solution
- Any alternatives you've considered

### Code Contributions

1. Fork the repository
2. Create a feature branch
3. Make your changes (with tests!)
4. Submit a pull request

See [Development Setup](development.md) for detailed instructions.

## Project Structure

```
holoconf/
├── crates/
│   ├── holoconf-core/      # Rust core library
│   ├── holoconf-python/    # PyO3 Python bindings
│   └── holoconf-cli/       # Rust CLI binary
├── packages/
│   └── python/holoconf/    # Python package
├── tests/
│   └── acceptance/         # Cross-language acceptance tests
├── docs/
│   ├── adr/                # Architecture Decision Records
│   └── specs/              # Feature specifications
└── tools/
    └── test_runner.py      # Acceptance test runner
```

## Decision Making

Significant decisions are documented as Architecture Decision Records (ADRs). Before proposing major changes, review existing ADRs and consider whether your change requires a new one.

See [Architecture Decisions](../adr/README.md) for the full list.

## Code of Conduct

- Be respectful and inclusive
- Provide constructive feedback
- Focus on the code, not the person
- Help others learn and grow

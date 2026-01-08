# Contributing to holoconf

Thank you for your interest in contributing to holoconf! This document provides guidelines and instructions for contributing to the project.

## Development Environment Setup

### Prerequisites

- **Rust**: Install the Rust toolchain (1.75 or later) from [rustup.rs](https://rustup.rs/)
- **Python**: Python 3.8 or later
- **maturin**: For building Python bindings

### Setting Up Your Environment

1. Clone the repository:
```bash
git clone https://github.com/holoconf/holoconf.git
cd holoconf
```

2. Set up a Python virtual environment:
```bash
python -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate
```

3. Install maturin and development dependencies:
```bash
pip install maturin
```

4. Build the project:
```bash
# Build Rust core
cargo build

# Build Python bindings in development mode
cd crates/holoconf-python
maturin develop
cd ../..
```

5. Verify the setup:
```bash
# Run Rust tests
cargo test

# Run Python acceptance tests
python tools/test_runner.py --binding python
```

## Architecture Decision Records (ADRs)

We use Architecture Decision Records to document significant architectural decisions and their rationale.

### When to Create an ADR

Create an ADR when making decisions about:
- Core architecture and design patterns
- Language binding interfaces
- Performance-critical implementations
- Breaking changes to public APIs
- Cross-cutting concerns (security, threading, etc.)

### ADR Process

1. Copy the ADR template:
```bash
cp docs/adr/template.md docs/adr/ADR-XXX-your-decision-name.md
```

2. Fill out the template with:
   - **Status**: Proposed, Accepted, Deprecated, or Superseded
   - **Context**: Background and problem being solved
   - **Decision**: The decision being made
   - **Consequences**: Trade-offs and implications

3. Submit the ADR as part of your pull request

4. ADRs should be discussed and reviewed before being accepted

### Existing ADRs

See [docs/adr/](docs/adr/) for existing Architecture Decision Records, including:
- ADR-001: Multi-Language Architecture
- ADR-002: Resolver Architecture
- ADR-011: Interpolation Syntax
- ADR-013: Testing Architecture

## Feature Specifications

Feature specifications provide detailed behavioral documentation for holoconf features.

### When to Create a Feature Spec

Create a feature specification when:
- Implementing a new user-facing feature
- Defining behavior that spans multiple language bindings
- Documenting complex feature interactions
- Specifying acceptance criteria

### Feature Spec Process

1. Copy the feature spec template:
```bash
cp docs/specs/features/template.md docs/specs/features/FEAT-XXX-feature-name.md
```

2. Fill out the template with:
   - **Status**: Draft, Accepted, Implemented, or Deprecated
   - **Overview**: Feature description and motivation
   - **Specification**: Detailed behavior and API design
   - **Examples**: Usage examples in each language
   - **Test Cases**: Acceptance criteria

3. Submit the spec as part of your pull request

4. Implement the feature with corresponding acceptance tests

### Existing Feature Specs

See [docs/specs/features/](docs/specs/features/) for existing specifications, including:
- FEAT-001: Config Loading
- FEAT-002: Core Resolvers
- FEAT-003: Config Merging
- FEAT-004: Schema Validation

## Testing Requirements

holoconf uses a multi-layered testing approach to ensure correctness across all language bindings.

### Unit Tests

Write unit tests for each component in the appropriate language:

**Rust (in crates/holoconf-core/):**
```bash
cargo test
```

Follow Rust testing conventions:
- Place tests in a `tests` module or `tests/` directory
- Use descriptive test names: `test_feature_behavior_condition`
- Test both success and error cases

**Python (in packages/python/):**
```bash
pytest
```

Follow Python testing conventions:
- Place tests in `tests/` directories
- Use pytest fixtures for common setup
- Test both success and error cases

### Acceptance Tests

holoconf uses YAML-based acceptance tests (per ADR-013) that run against all language bindings.

**Creating Acceptance Tests:**

1. Create a YAML test file in `tests/acceptance/`:
```yaml
name: "Feature Description"
tests:
  - name: "Test case description"
    config:
      key: "value"
      nested:
        path: "${env:HOME}"
    assertions:
      - path: "key"
        expected: "value"
      - path: "nested.path"
        expected_pattern: "/.*"
```

2. Run acceptance tests:
```bash
# Test Python bindings
python tools/test_runner.py --binding python

# Test all bindings
python tools/test_runner.py --binding all

# Test specific file
python tools/test_runner.py --binding python --file tests/acceptance/interpolation.yaml
```

**Acceptance Test Requirements:**
- All new features must have acceptance tests
- Tests must pass for all language bindings
- Use descriptive test names
- Cover both success and error cases
- Test edge cases and boundary conditions

## Code Style

### Rust

- Use `rustfmt` for code formatting:
```bash
cargo fmt
```

- Use `clippy` for linting:
```bash
cargo clippy -- -D warnings
```

- Follow Rust naming conventions:
  - `snake_case` for functions and variables
  - `CamelCase` for types
  - `SCREAMING_SNAKE_CASE` for constants

### Python

- Follow PEP 8 style guidelines
- Use type hints where appropriate
- Format code with standard Python formatting tools
- Keep line length to 88-100 characters

### Documentation

- Write clear, concise documentation
- Include code examples where appropriate
- Document public APIs thoroughly
- Keep ADRs and feature specs up to date

## Pull Request Process

1. **Create a feature branch:**
```bash
git checkout -b feature/your-feature-name
```

2. **Make your changes:**
   - Write code following style guidelines
   - Add or update tests
   - Update documentation as needed
   - Create/update ADRs for architectural changes
   - Create/update feature specs for new features

3. **Run tests locally:**
```bash
# Rust tests
cargo test
cargo clippy

# Python acceptance tests
python tools/test_runner.py --binding python

# Format code
cargo fmt
```

4. **Commit your changes:**
   - Write clear, descriptive commit messages
   - Reference related issues or ADRs
   - Keep commits focused and atomic

5. **Push and create a pull request:**
```bash
git push origin feature/your-feature-name
```

6. **PR Requirements:**
   - Provide a clear description of changes
   - Link to related issues or ADRs
   - Ensure all tests pass
   - Respond to review feedback
   - Keep PR scope focused

7. **Review Process:**
   - At least one maintainer approval required
   - All CI checks must pass
   - Code review for style and correctness
   - Architecture review for significant changes

## Getting Help

- Check existing [ADRs](docs/adr/) and [feature specs](docs/specs/features/)
- Review the [README](README.md) for project overview
- Open an issue for bugs or feature requests
- Ask questions in pull request discussions

## Code of Conduct

- Be respectful and inclusive
- Provide constructive feedback
- Focus on the code, not the person
- Help others learn and grow

Thank you for contributing to holoconf!

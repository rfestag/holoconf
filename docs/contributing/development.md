# Development Setup

This guide helps you set up a development environment for HoloConf.

## Prerequisites

- **Rust** 1.88 or later ([rustup.rs](https://rustup.rs/))
- **Python** 3.8 or later
- **maturin** for building Python bindings

## Quick Setup

```bash
# Clone the repository
git clone https://github.com/rfestag/holoconf.git
cd holoconf

# Set up Python virtual environment
python -m venv .venv
source .venv/bin/activate  # Windows: .venv\Scripts\activate

# Install development tools
make install-tools

# Build everything
make build

# Run all tests
make test
```

## Step-by-Step Setup

### 1. Clone and Enter the Repository

```bash
git clone https://github.com/rfestag/holoconf.git
cd holoconf
```

### 2. Set Up Python Environment

```bash
python -m venv .venv
source .venv/bin/activate
```

### 3. Install Development Dependencies

```bash
# Install Rust tooling
cargo install cargo-deny cargo-audit cargo-machete

# Install Python dev dependencies
cd packages/python/holoconf
pip install -e ".[dev]"
cd ../../..
```

### 4. Build the Project

```bash
# Build Rust crates
cargo build

# Build Python bindings (development mode)
cd packages/python/holoconf
maturin develop
cd ../../..
```

### 5. Verify Setup

```bash
# Run Rust tests
cargo test

# Run Python tests
cd packages/python/holoconf
pytest tests/ -v
cd ../../..

# Run acceptance tests
python tools/test_runner.py --driver python 'tests/acceptance/**/*.yaml' -v
```

## Available Make Targets

| Target | Description |
|--------|-------------|
| `make help` | Show all available targets |
| `make build` | Build Rust crates and Python bindings |
| `make test` | Run all tests (Rust, Python, acceptance) |
| `make lint` | Run all linters |
| `make format` | Format all code |
| `make check` | Run lint + security + test |
| `make clean` | Clean build artifacts |

## Code Style

### Rust

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings
```

Follow Rust conventions:
- `snake_case` for functions and variables
- `CamelCase` for types
- `SCREAMING_SNAKE_CASE` for constants

### Python

```bash
# Format and lint
cd packages/python/holoconf
ruff format src/ tests/
ruff check src/ tests/
```

Follow PEP 8 guidelines with type hints.

## Pull Request Workflow

1. **Create a feature branch:**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make changes** with tests and documentation

3. **Run checks locally:**
   ```bash
   make check
   ```

4. **Commit with clear messages:**
   ```bash
   git commit -m "feat: add support for X"
   ```

5. **Push and open PR:**
   ```bash
   git push origin feature/your-feature-name
   ```

## Creating an ADR

For architectural decisions, create an ADR:

```bash
cp docs/adr/template.md docs/adr/ADR-XXX-your-decision.md
```

Fill out the template and submit with your PR. See [Architecture Decisions](../adr/README.md) for examples.

## Creating a Feature Spec

For new features, create a specification:

```bash
cp docs/specs/features/template.md docs/specs/features/FEAT-XXX-feature-name.md
```

See [Feature Specs](../specs/features/README.md) for examples.

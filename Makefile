# Holoconf Development Makefile
# Run `make help` to see available targets

.PHONY: help install-tools lint lint-rust lint-python format format-rust format-python \
        security security-rust security-python test test-rust test-python \
        test-acceptance build clean check all audit-unsafe semver-check sbom \
        docs docs-serve docs-build

# Default target
help:
	@echo "Holoconf Development Commands"
	@echo ""
	@echo "Quality & Linting:"
	@echo "  make lint          - Run all linters (Rust + Python)"
	@echo "  make lint-rust     - Run Rust linters (clippy)"
	@echo "  make lint-python   - Run Python linter (ruff)"
	@echo "  make format        - Format all code"
	@echo "  make format-rust   - Format Rust code"
	@echo "  make format-python - Format Python code"
	@echo ""
	@echo "Security:"
	@echo "  make security        - Run all security checks"
	@echo "  make security-rust   - Run Rust security (cargo-deny, cargo-audit)"
	@echo "  make security-python - Run Python security (pip-audit)"
	@echo "  make audit-unsafe    - Report unsafe code usage (cargo-geiger)"
	@echo ""
	@echo "Testing:"
	@echo "  make test          - Run all tests"
	@echo "  make test-rust     - Run Rust unit tests"
	@echo "  make test-python   - Run Python unit tests"
	@echo "  make test-acceptance - Run acceptance tests (both drivers)"
	@echo ""
	@echo "Documentation:"
	@echo "  make docs          - Build documentation site"
	@echo "  make docs-serve    - Serve documentation locally (with live reload)"
	@echo ""
	@echo "Other:"
	@echo "  make install-tools - Install required dev tools (cargo-deny, cargo-audit, etc.)"
	@echo "  make build         - Build all packages"
	@echo "  make check         - Run all checks (lint + security + test)"
	@echo "  make semver-check  - Check for semver violations (run before releases)"
	@echo "  make sbom          - Generate SBOMs for all packages (CycloneDX)"
	@echo "  make clean         - Clean build artifacts"
	@echo ""

# =============================================================================
# Tool Installation
# =============================================================================

# Run this first when setting up a development environment:
#   make install-tools
#
# This installs all required Rust and Python tooling for development.

install-tools:
	@echo "══════════════════════════════════════════════════════════════════"
	@echo "Installing development tools..."
	@echo "══════════════════════════════════════════════════════════════════"
	@echo ""
	@echo "→ Installing Rust tools..."
	cargo install cargo-deny cargo-audit cargo-machete cargo-geiger cargo-semver-checks cargo-cyclonedx
	@echo ""
	@echo "→ Installing Python dev dependencies..."
	cd packages/python/holoconf && pip install -e ".[dev]"
	@echo ""
	@echo "→ Building Python bindings..."
	cd packages/python/holoconf && maturin develop
	@echo ""
	@echo "══════════════════════════════════════════════════════════════════"
	@echo "✓ All tools installed! You can now run 'make check'"
	@echo "══════════════════════════════════════════════════════════════════"

# =============================================================================
# Linting
# =============================================================================

lint: lint-rust lint-python
	@echo "✓ All linting passed"

lint-rust:
	@echo "→ Running Rust linters..."
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- \
		-D warnings \
		-A clippy::result_large_err \
		-A clippy::field_reassign_with_default

lint-python:
	@echo "→ Running Python linter..."
	cd packages/python/holoconf && ruff check src/ tests/
	cd packages/python/holoconf && ruff format --check src/ tests/

# =============================================================================
# Formatting
# =============================================================================

format: format-rust format-python
	@echo "✓ All formatting complete"

format-rust:
	@echo "→ Formatting Rust code..."
	cargo fmt --all

format-python:
	@echo "→ Formatting Python code..."
	cd packages/python/holoconf && ruff format src/ tests/
	cd packages/python/holoconf && ruff check --fix src/ tests/ || true

# =============================================================================
# Security
# =============================================================================

security: security-rust security-python
	@echo "✓ All security checks passed"

security-rust:
	@echo "→ Running Rust security checks..."
	cargo deny check
	cargo audit

security-python:
	@echo "→ Running Python security checks..."
	cd packages/python/holoconf && pip-audit

# Audit unsafe code - informational, not a blocker
audit-unsafe:
	@echo "→ Auditing unsafe code usage..."
	@which cargo-geiger > /dev/null 2>&1 || (echo "Error: cargo-geiger not found. Run 'make install-tools' first." && exit 1)
	@echo "→ holoconf-core:"
	cd crates/holoconf-core && cargo geiger 2>&1 | tail -5 || true
	@echo "→ holoconf-cli:"
	cd crates/holoconf-cli && cargo geiger 2>&1 | tail -5 || true
	@echo "→ holoconf-python:"
	cd crates/holoconf-python && cargo geiger 2>&1 | tail -5 || true
	@echo "✓ Unsafe audit complete (see above for details)"

# =============================================================================
# Testing
# =============================================================================

test: test-rust test-python test-acceptance
	@echo "✓ All tests passed"

test-rust:
	@echo "→ Running Rust tests..."
	cargo test --all

test-python:
	@echo "→ Running Python tests..."
	cd packages/python/holoconf && pytest tests/ -v

test-acceptance:
	@echo "→ Running acceptance tests (Rust driver)..."
	python tools/test_runner.py --driver rust 'tests/acceptance/**/*.yaml' -v
	@echo "→ Running acceptance tests (Python driver)..."
	python tools/test_runner.py --driver python 'tests/acceptance/**/*.yaml' -v

# =============================================================================
# Build
# =============================================================================

build:
	@echo "→ Building Rust crates..."
	cargo build --all
	@echo "→ Building Python bindings..."
	cd packages/python/holoconf && maturin develop

# =============================================================================
# Combined Targets
# =============================================================================

# Full check: lint + security + test + audit
check: lint security test audit-unsafe
	@echo ""
	@echo "══════════════════════════════════════════════════════════════════"
	@echo "✓ All checks passed!"
	@echo "══════════════════════════════════════════════════════════════════"

# Alias for check
all: check

# =============================================================================
# Release Checks
# =============================================================================

# Check for semver violations (compare against last published version)
semver-check:
	@echo "→ Checking for semver violations..."
	cargo semver-checks check-release --package holoconf-core

# =============================================================================
# SBOM Generation
# =============================================================================

SBOM_DIR := sbom

sbom:
	@echo "→ Generating SBOMs..."
	@mkdir -p $(SBOM_DIR)
	@echo "→ Generating Rust SBOM (CycloneDX)..."
	cargo cyclonedx --manifest-path Cargo.toml --format json > $(SBOM_DIR)/holoconf-rust.cdx.json
	@echo "→ Generating Python SBOM (CycloneDX)..."
	cd packages/python/holoconf && pip-audit --format cyclonedx-json > ../../../$(SBOM_DIR)/holoconf-python.cdx.json 2>/dev/null || \
		cyclonedx-py environment --output-format json > ../../../$(SBOM_DIR)/holoconf-python.cdx.json
	@echo "✓ SBOMs generated in $(SBOM_DIR)/"
	@ls -la $(SBOM_DIR)/

# =============================================================================
# Documentation
# =============================================================================

DOCS_VENV := .venv-docs
MKDOCS := $(DOCS_VENV)/bin/mkdocs

$(DOCS_VENV)/bin/mkdocs:
	@echo "→ Creating docs virtual environment..."
	python -m venv $(DOCS_VENV)
	$(DOCS_VENV)/bin/pip install --quiet mkdocs-material mike "mkdocstrings[python]" ruff

docs: docs-build
	@echo "✓ Documentation built in site/"

docs-build: $(MKDOCS)
	@echo "→ Building documentation..."
	$(MKDOCS) build --strict

docs-serve: $(MKDOCS)
	@echo "→ Starting documentation server..."
	@echo "→ Open http://127.0.0.1:8000 in your browser"
	$(MKDOCS) serve

# =============================================================================
# Cleanup
# =============================================================================

clean:
	@echo "→ Cleaning build artifacts..."
	cargo clean
	rm -rf packages/python/holoconf/target/
	rm -rf packages/python/holoconf/.venv/
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
	find . -type d -name "*.egg-info" -exec rm -rf {} + 2>/dev/null || true
	find . -type f -name "*.so" -delete 2>/dev/null || true
	@echo "✓ Clean complete"

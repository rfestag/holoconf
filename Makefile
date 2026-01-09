# Holoconf Development Makefile
# Run `make help` to see available targets

.PHONY: help install-tools lint lint-rust lint-python format format-rust format-python \
        security security-rust security-python test test-rust test-python \
        test-acceptance test-acceptance-json build clean check all audit-unsafe semver-check sbom \
        docs docs-serve docs-build coverage coverage-rust coverage-python coverage-html \
        coverage-acceptance coverage-full

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
	@echo "  make test                 - Run all tests"
	@echo "  make test-rust            - Run Rust unit tests"
	@echo "  make test-python          - Run Python unit tests"
	@echo "  make test-acceptance      - Run acceptance tests (both drivers)"
	@echo "  make test-acceptance-json - Run acceptance tests and save JSON results"
	@echo ""
	@echo "Coverage:"
	@echo "  make coverage            - Run tests with coverage (Rust + Python)"
	@echo "  make coverage-rust       - Run Rust unit tests with coverage"
	@echo "  make coverage-python     - Run Python tests with coverage"
	@echo "  make coverage-acceptance - Run acceptance tests with Rust coverage"
	@echo "  make coverage-full       - Run Rust unit + acceptance tests with coverage"
	@echo "  make coverage-html       - Generate HTML coverage reports"
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
	cargo install cargo-deny cargo-audit cargo-machete cargo-geiger cargo-semver-checks cargo-cyclonedx cargo-llvm-cov
	rustup component add llvm-tools-preview
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

# Python executable - use venv if available
VENV_PYTHON := packages/python/holoconf/.venv/bin/python

test-acceptance:
	@echo "→ Running acceptance tests (Rust driver)..."
	$(VENV_PYTHON) tools/test_runner.py --driver rust 'tests/acceptance/**/*.yaml' -v
	@echo "→ Running acceptance tests (Python driver)..."
	$(VENV_PYTHON) tools/test_runner.py --driver python 'tests/acceptance/**/*.yaml' -v

# Generate JSON results for documentation matrix
test-acceptance-json:
	@echo "→ Running acceptance tests and generating JSON results..."
	@mkdir -p coverage/acceptance
	$(VENV_PYTHON) tools/test_runner.py --driver rust 'tests/acceptance/**/*.yaml' --json coverage/acceptance/rust.json || true
	$(VENV_PYTHON) tools/test_runner.py --driver python 'tests/acceptance/**/*.yaml' --json coverage/acceptance/python.json || true
	@echo "✓ Results written to coverage/acceptance/"

# =============================================================================
# Coverage
# =============================================================================

COVERAGE_DIR := coverage

coverage: coverage-rust coverage-python
	@echo "✓ Coverage reports generated in $(COVERAGE_DIR)/"

coverage-rust:
	@echo "→ Running Rust tests with coverage..."
	@mkdir -p $(COVERAGE_DIR)
	cargo llvm-cov --all-features --workspace --lcov --output-path $(COVERAGE_DIR)/rust-lcov.info
	@echo "✓ Rust coverage: $(COVERAGE_DIR)/rust-lcov.info"

coverage-python:
	@echo "→ Running Python tests with coverage..."
	@mkdir -p $(COVERAGE_DIR)
	cd packages/python/holoconf && pytest tests/ --cov=holoconf --cov-report=xml:../../../$(COVERAGE_DIR)/python-coverage.xml --cov-report=term
	@echo "✓ Python coverage: $(COVERAGE_DIR)/python-coverage.xml"

# Python venv paths
PYTHON_VENV := packages/python/holoconf/.venv
VENV_PYTHON := $(PYTHON_VENV)/bin/python
VENV_MATURIN := $(PYTHON_VENV)/bin/maturin

# Run acceptance tests with Rust coverage instrumentation
# This measures which Rust code is exercised by the acceptance test suite
# All commands must run in the same shell with the coverage environment set
coverage-acceptance:
	@echo "→ Running acceptance tests with Rust coverage..."
	@mkdir -p $(COVERAGE_DIR)
	@bash -c '\
		set -e; \
		export PATH="$$HOME/.cargo/bin:$$PATH"; \
		cargo llvm-cov clean --workspace; \
		source <(cargo llvm-cov show-env --export-prefix); \
		export CARGO_TARGET_DIR=$$CARGO_LLVM_COV_TARGET_DIR; \
		export CARGO_INCREMENTAL=1; \
		echo "→ Building instrumented Python bindings..."; \
		cd packages/python/holoconf && $(CURDIR)/$(VENV_MATURIN) develop; \
		cd $(CURDIR); \
		echo "→ Running acceptance tests..."; \
		$(CURDIR)/$(VENV_PYTHON) tools/test_runner.py --driver rust "tests/acceptance/**/*.yaml" -v; \
		echo "→ Generating coverage report..."; \
		cargo llvm-cov report --lcov --output-path $(COVERAGE_DIR)/acceptance-lcov.info; \
	'
	@echo "✓ Acceptance test coverage: $(COVERAGE_DIR)/acceptance-lcov.info"

# Combined coverage: Rust unit tests + acceptance tests
# All commands must run in the same shell with the coverage environment set
coverage-full:
	@echo "→ Running full Rust coverage (unit + acceptance tests)..."
	@mkdir -p $(COVERAGE_DIR)
	@bash -c '\
		set -e; \
		export PATH="$$HOME/.cargo/bin:$$PATH"; \
		cargo llvm-cov clean --workspace; \
		source <(cargo llvm-cov show-env --export-prefix); \
		export CARGO_TARGET_DIR=$$CARGO_LLVM_COV_TARGET_DIR; \
		export CARGO_INCREMENTAL=1; \
		echo "→ Running Rust unit tests..."; \
		cargo test --all-features --workspace; \
		echo "→ Building instrumented Python bindings..."; \
		cd packages/python/holoconf && $(CURDIR)/$(VENV_MATURIN) develop; \
		cd $(CURDIR); \
		echo "→ Running acceptance tests..."; \
		$(CURDIR)/$(VENV_PYTHON) tools/test_runner.py --driver rust "tests/acceptance/**/*.yaml" -v; \
		echo "→ Generating combined coverage report..."; \
		cargo llvm-cov report --lcov --output-path $(COVERAGE_DIR)/rust-lcov.info; \
	'
	@echo "✓ Combined Rust coverage: $(COVERAGE_DIR)/rust-lcov.info"

coverage-html:
	@echo "→ Generating HTML coverage reports..."
	@mkdir -p docs/coverage/rust docs/coverage/python
	cargo llvm-cov --all-features --workspace --html --output-dir docs/coverage/rust
	cd packages/python/holoconf && pytest tests/ --cov=holoconf --cov-report=html:../../../docs/coverage/python || true
	@echo "✓ HTML reports generated in docs/coverage/"
	@echo "  Rust:   docs/coverage/rust/html/index.html"
	@echo "  Python: docs/coverage/python/index.html"
	@echo ""
	@echo "Run 'make docs-serve' to view in documentation site"

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

docs-build: $(MKDOCS) test-acceptance-json
	@echo "→ Building documentation..."
	$(MKDOCS) build --strict

docs-serve: $(MKDOCS) test-acceptance-json
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
	rm -rf $(COVERAGE_DIR)/
	rm -rf packages/python/holoconf/.coverage
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
	find . -type d -name "*.egg-info" -exec rm -rf {} + 2>/dev/null || true
	find . -type f -name "*.so" -delete 2>/dev/null || true
	@echo "✓ Clean complete"

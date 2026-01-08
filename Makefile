# Holoconf Development Makefile
# Run `make help` to see available targets

.PHONY: help lint lint-rust lint-python format format-rust format-python \
        security security-rust security-python test test-rust test-python \
        test-acceptance build clean check all

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
	@echo "  make security      - Run all security checks"
	@echo "  make security-rust - Run Rust security (cargo-deny, cargo-audit)"
	@echo "  make security-python - Run Python security (pip-audit)"
	@echo ""
	@echo "Testing:"
	@echo "  make test          - Run all tests"
	@echo "  make test-rust     - Run Rust unit tests"
	@echo "  make test-python   - Run Python unit tests"
	@echo "  make test-acceptance - Run acceptance tests (both drivers)"
	@echo ""
	@echo "Other:"
	@echo "  make build         - Build all packages"
	@echo "  make check         - Run all checks (lint + security + test)"
	@echo "  make clean         - Clean build artifacts"
	@echo ""

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

# Full check: lint + security + test
check: lint security test
	@echo ""
	@echo "══════════════════════════════════════════════════════════════════"
	@echo "✓ All checks passed!"
	@echo "══════════════════════════════════════════════════════════════════"

# Alias for check
all: check

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

# capsule-run Makefile
# Convenient commands for development and testing

.PHONY: help install test test-local test-ci clean fmt clippy check build release

# Default target
help: ## Show this help message
	@echo "Available commands:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

# Development commands
install: ## Install development dependencies
	@echo "Installing Rust toolchain and components..."
	rustup component add rustfmt clippy
	@echo "Installing act for local CI testing..."
	@if command -v brew >/dev/null 2>&1; then \
		brew install act; \
	elif command -v apt-get >/dev/null 2>&1; then \
		curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash; \
	else \
		echo "Please install act manually: https://github.com/nektos/act"; \
	fi

check: ## Check code compilation
	cargo check
	cargo check --no-default-features

fmt: ## Format code
	cargo fmt --all

fmt-check: ## Check code formatting
	cargo fmt --all -- --check

clippy: ## Run clippy lints
	cargo clippy -- -D warnings

test: ## Run tests
	cargo test
	cargo test --no-default-features

test-doc: ## Run documentation tests
	cargo test --doc

build: ## Build in debug mode
	cargo build
	cargo build --no-default-features

build-release: ## Build in release mode
	cargo build --release
	cargo build --release --no-default-features

clean: ## Clean build artifacts
	cargo clean

# Local CI testing with act
test-local-quick: ## Quick local CI test
	./scripts/test-ci-locally.sh quick

test-local-basic: ## Basic local CI test
	./scripts/test-ci-locally.sh basic

test-local-test: ## Run test suite locally
	./scripts/test-ci-locally.sh test

test-local-security: ## Run security tests locally
	./scripts/test-ci-locally.sh security

test-local-comprehensive: ## Run comprehensive test suite locally
	./scripts/test-ci-locally.sh comprehensive

test-local-features: ## Test all feature combinations locally
	./scripts/test-ci-locally.sh feature-matrix

test-local-platforms: ## Test cross-platform compatibility locally
	./scripts/test-ci-locally.sh cross-platform

list-ci-jobs: ## List all available CI jobs
	./scripts/test-ci-locally.sh list

# Direct act commands for advanced usage
act-list: ## List GitHub Actions workflows and jobs
	act -l

act-check: ## Run compilation check locally
	act push -j check --reuse

act-test: ## Run tests locally
	act push -j test --reuse

act-fmt: ## Run formatting check locally
	act push -j fmt --dry-run --reuse

act-clippy: ## Run clippy locally
	act push -j clippy --reuse

act-security: ## Run security checks locally
	act -W .github/workflows/security.yml --reuse

# Convenience combinations
pre-commit: fmt-check clippy test ## Run pre-commit checks
	@echo "✅ Pre-commit checks passed!"

pre-push: test-local-basic ## Run checks before pushing
	@echo "✅ Pre-push validation passed!"

ci-validate: test-local-comprehensive ## Validate full CI pipeline locally
	@echo "✅ Full CI validation passed!"

# Release preparation
release-check: ## Check if ready for release
	@echo "🔍 Checking release readiness..."
	cargo fmt --all -- --check
	cargo clippy -- -D warnings
	cargo test
	cargo test --no-default-features
	cargo build --release
	cargo build --release --no-default-features
	./scripts/test-ci-locally.sh comprehensive
	@echo "✅ Release checks passed!"

# Documentation
docs: ## Build documentation
	cargo doc --no-deps --all-features

docs-open: ## Build and open documentation
	cargo doc --no-deps --all-features --open

# Security
audit: ## Run security audit
	cargo audit

# Performance testing
bench: ## Run benchmarks (if available)
	cargo bench

# Development workflow examples
dev-setup: install ## Setup development environment
	@echo "🚀 Development environment setup complete!"
	@echo "💡 Try these commands:"
	@echo "  make pre-commit     # Before committing"
	@echo "  make test-local-quick  # Quick CI validation"
	@echo "  make pre-push       # Before pushing"

example-workflow: ## Show example development workflow
	@echo "📖 Example development workflow:"
	@echo ""
	@echo "1. Setup (run once):"
	@echo "   make dev-setup"
	@echo ""
	@echo "2. Before each commit:"
	@echo "   make pre-commit"
	@echo ""
	@echo "3. Test locally:"
	@echo "   make test-local-quick    # Fast validation"
	@echo "   make test-local-basic    # Basic CI tests"
	@echo ""
	@echo "4. Before pushing:"
	@echo "   make pre-push"
	@echo ""
	@echo "5. Before releasing:"
	@echo "   make release-check"
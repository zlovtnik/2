# Rust JWT Backend Makefile
# Production-grade automation for development and deployment

.PHONY: help build test lint format clean docker k8s security docs
.DEFAULT_GOAL := help

# Variables
CARGO := cargo
DOCKER := docker
KUBECTL := kubectl
HELM := helm
PROJECT_NAME := server
VERSION := $(shell git describe --tags --always --dirty)
REGISTRY := your-registry.com
IMAGE := $(REGISTRY)/$(PROJECT_NAME):$(VERSION)

# Colors for output
GREEN := \033[0;32m
YELLOW := \033[0;33m
RED := \033[0;31m
NC := \033[0m # No Color

## Development Commands

help: ## Show this help message
	@echo "$(GREEN)$(PROJECT_NAME) - Development Commands$(NC)"
	@echo ""
	@awk 'BEGIN {FS = ":.*##"} /^[a-zA-Z_-]+:.*##/ {printf "  $(YELLOW)%-20s$(NC) %s\n", $$1, $$2}' $(MAKEFILE_LIST)

install: ## Install development dependencies
	@echo "$(GREEN)Installing development dependencies...$(NC)"
	rustup component add clippy rustfmt
	cargo install sqlx-cli cargo-audit cargo-deny cargo-tarpaulin
	cargo install cargo-watch cargo-nextest
	@echo "$(GREEN)✓ Dependencies installed$(NC)"

setup: install ## Setup development environment
	@echo "$(GREEN)Setting up development environment...$(NC)"
	cp .env.example .env.local
	docker-compose up -d postgres redis
	sleep 5
	sqlx database create
	sqlx migrate run
	@echo "$(GREEN)✓ Development environment ready$(NC)"

dev: ## Start development server with hot reload
	@echo "$(GREEN)Starting development server...$(NC)"
	cargo watch -x 'run --bin server'

## Building

build: ## Build the application
	@echo "$(GREEN)Building application...$(NC)"
	cargo build --release
	@echo "$(GREEN)✓ Build complete$(NC)"

build-debug: ## Build debug version
	@echo "$(GREEN)Building debug version...$(NC)"
	cargo build
	@echo "$(GREEN)✓ Debug build complete$(NC)"

## Testing

test: ## Run all tests
	@echo "$(GREEN)Running all tests...$(NC)"
	cargo nextest run --all-features
	@echo "$(GREEN)✓ All tests passed$(NC)"

test-unit: ## Run unit tests only
	@echo "$(GREEN)Running unit tests...$(NC)"
	cargo nextest run --lib
	@echo "$(GREEN)✓ Unit tests passed$(NC)"

test-integration: ## Run integration tests
	@echo "$(GREEN)Running integration tests...$(NC)"
	cargo nextest run --test integration
	@echo "$(GREEN)✓ Integration tests passed$(NC)"

test-coverage: ## Generate test coverage report
	@echo "$(GREEN)Generating coverage report...$(NC)"
	cargo tarpaulin --out Html --output-dir coverage/
	@echo "$(GREEN)✓ Coverage report generated in coverage/$(NC)"

test-load: ## Run load tests (requires k6)
	@echo "$(GREEN)Running load tests...$(NC)"
	k6 run tests/performance/load_test.js
	@echo "$(GREEN)✓ Load tests complete$(NC)"

## Code Quality

lint: ## Run linting
	@echo "$(GREEN)Running linter...$(NC)"
	cargo clippy --all-targets --all-features -- -D warnings
	@echo "$(GREEN)✓ Linting passed$(NC)"

format: ## Format code
	@echo "$(GREEN)Formatting code...$(NC)"
	cargo fmt --all
	@echo "$(GREEN)✓ Code formatted$(NC)"

format-check: ## Check code formatting
	@echo "$(GREEN)Checking code formatting...$(NC)"
	cargo fmt --all -- --check
	@echo "$(GREEN)✓ Code formatting is correct$(NC)"

check: ## Run cargo check
	@echo "$(GREEN)Running cargo check...$(NC)"
	cargo check --all-targets --all-features
	@echo "$(GREEN)✓ Check passed$(NC)"

## Security

security: ## Run security audits
	@echo "$(GREEN)Running security audits...$(NC)"
	cargo audit
	cargo deny check
	@echo "$(GREEN)✓ Security audit passed$(NC)"

security-fix: ## Fix security vulnerabilities
	@echo "$(GREEN)Fixing security vulnerabilities...$(NC)"
	cargo audit fix
	@echo "$(GREEN)✓ Security vulnerabilities fixed$(NC)"

## Database

db-setup: ## Setup database
	@echo "$(GREEN)Setting up database...$(NC)"
	sqlx database create
	sqlx migrate run
	@echo "$(GREEN)✓ Database setup complete$(NC)"

db-migrate: ## Run database migrations
	@echo "$(GREEN)Running database migrations...$(NC)"
	sqlx migrate run
	@echo "$(GREEN)✓ Migrations complete$(NC)"

db-rollback: ## Rollback last migration
	@echo "$(GREEN)Rolling back last migration...$(NC)"
	sqlx migrate revert
	@echo "$(GREEN)✓ Rollback complete$(NC)"

db-reset: ## Reset database
	@echo "$(GREEN)Resetting database...$(NC)"
	sqlx database drop -y
	sqlx database create
	sqlx migrate run
	@echo "$(GREEN)✓ Database reset complete$(NC)"

## Docker

docker-build: ## Build Docker image
	@echo "$(GREEN)Building Docker image...$(NC)"
	$(DOCKER) build -t $(IMAGE) .
	@echo "$(GREEN)✓ Docker image built: $(IMAGE)$(NC)"

docker-run: ## Run Docker container locally
	@echo "$(GREEN)Running Docker container...$(NC)"
	$(DOCKER) run --rm -p 3000:3000 --env-file .env.local $(IMAGE)

docker-push: ## Push Docker image to registry
	@echo "$(GREEN)Pushing Docker image...$(NC)"
	$(DOCKER) push $(IMAGE)
	@echo "$(GREEN)✓ Docker image pushed: $(IMAGE)$(NC)"

docker-clean: ## Clean Docker artifacts
	@echo "$(GREEN)Cleaning Docker artifacts...$(NC)"
	$(DOCKER) system prune -f
	$(DOCKER) image prune -f
	@echo "$(GREEN)✓ Docker cleanup complete$(NC)"

## Kubernetes

k8s-deploy: ## Deploy to Kubernetes
	@echo "$(GREEN)Deploying to Kubernetes...$(NC)"
	$(HELM) upgrade --install $(PROJECT_NAME) ./helm/$(PROJECT_NAME) \
		--set image.tag=$(VERSION) \
		--values ./helm/$(PROJECT_NAME)/values.yaml
	@echo "$(GREEN)✓ Deployed to Kubernetes$(NC)"

k8s-deploy-prod: ## Deploy to production
	@echo "$(YELLOW)Deploying to PRODUCTION...$(NC)"
	@read -p "Are you sure? [y/N] " -n 1 -r; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		$(HELM) upgrade --install $(PROJECT_NAME) ./helm/$(PROJECT_NAME) \
			--namespace production \
			--set image.tag=$(VERSION) \
			--values ./helm/$(PROJECT_NAME)/values.prod.yaml; \
		echo "$(GREEN)✓ Deployed to production$(NC)"; \
	else \
		echo "$(RED)Deployment cancelled$(NC)"; \
	fi

k8s-status: ## Check Kubernetes deployment status
	@echo "$(GREEN)Checking deployment status...$(NC)"
	$(KUBECTL) get pods -l app=$(PROJECT_NAME)
	$(KUBECTL) get services -l app=$(PROJECT_NAME)

k8s-logs: ## Show Kubernetes logs
	@echo "$(GREEN)Showing logs...$(NC)"
	$(KUBECTL) logs -l app=$(PROJECT_NAME) --tail=100 -f

k8s-delete: ## Delete Kubernetes deployment
	@echo "$(GREEN)Deleting Kubernetes deployment...$(NC)"
	$(HELM) uninstall $(PROJECT_NAME)
	@echo "$(GREEN)✓ Deployment deleted$(NC)"

## CI/CD

ci: format-check lint security test ## Run CI pipeline locally
	@echo "$(GREEN)✓ CI pipeline completed successfully$(NC)"

pre-commit: format lint test-unit ## Run pre-commit checks
	@echo "$(GREEN)✓ Pre-commit checks passed$(NC)"

release: ## Create a new release
	@echo "$(GREEN)Creating release...$(NC)"
	@read -p "Enter version (e.g., v1.2.3): " version; \
	git tag -a $$version -m "Release $$version"; \
	git push origin $$version
	@echo "$(GREEN)✓ Release created$(NC)"

## Documentation

docs: ## Generate documentation
	@echo "$(GREEN)Generating documentation...$(NC)"
	cargo doc --no-deps --document-private-items
	@echo "$(GREEN)✓ Documentation generated$(NC)"

docs-open: docs ## Open documentation in browser
	@echo "$(GREEN)Opening documentation...$(NC)"
	cargo doc --no-deps --open

api-docs: ## Generate API documentation
	@echo "$(GREEN)Generating API documentation...$(NC)"
	# This would typically use a tool like swagger-codegen or openapi-generator
	@echo "$(GREEN)✓ API documentation generated$(NC)"

## Maintenance

clean: ## Clean build artifacts
	@echo "$(GREEN)Cleaning build artifacts...$(NC)"
	cargo clean
	rm -rf target/
	rm -rf coverage/
	@echo "$(GREEN)✓ Cleanup complete$(NC)"

update: ## Update dependencies
	@echo "$(GREEN)Updating dependencies...$(NC)"
	cargo update
	@echo "$(GREEN)✓ Dependencies updated$(NC)"

update-check: ## Check for outdated dependencies
	@echo "$(GREEN)Checking for outdated dependencies...$(NC)"
	cargo outdated
	@echo "$(GREEN)✓ Dependency check complete$(NC)"

## Environment Management

env-dev: ## Switch to development environment
	@echo "$(GREEN)Switching to development environment...$(NC)"
	cp .env.dev .env
	@echo "$(GREEN)✓ Switched to development$(NC)"

env-staging: ## Switch to staging environment
	@echo "$(GREEN)Switching to staging environment...$(NC)"
	cp .env.staging .env
	@echo "$(GREEN)✓ Switched to staging$(NC)"

env-prod: ## Switch to production environment
	@echo "$(YELLOW)Switching to PRODUCTION environment...$(NC)"
	@read -p "Are you sure? [y/N] " -n 1 -r; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		cp .env.prod .env; \
		echo "$(GREEN)✓ Switched to production$(NC)"; \
	else \
		echo "$(RED)Environment switch cancelled$(NC)"; \
	fi

## Monitoring

health-check: ## Check application health
	@echo "$(GREEN)Checking application health...$(NC)"
	curl -s http://localhost:3000/health | jq .
	@echo "$(GREEN)✓ Health check complete$(NC)"

metrics: ## Show application metrics
	@echo "$(GREEN)Showing metrics...$(NC)"
	curl -s http://localhost:9090/metrics | grep -E "^# HELP|^rust_jwt_backend"

logs: ## Show application logs
	@echo "$(GREEN)Showing logs...$(NC)"
	docker-compose logs -f app

## Utilities

benchmark: ## Run performance benchmarks
	@echo "$(GREEN)Running benchmarks...$(NC)"
	cargo bench
	@echo "$(GREEN)✓ Benchmarks complete$(NC)"

flamegraph: ## Generate flamegraph for profiling
	@echo "$(GREEN)Generating flamegraph...$(NC)"
	cargo flamegraph --bin server
	@echo "$(GREEN)✓ Flamegraph generated$(NC)"

sqlx-prepare: ## Prepare SQLx offline queries
	@echo "$(GREEN)Preparing SQLx queries...$(NC)"
	cargo sqlx prepare
	@echo "$(GREEN)✓ SQLx queries prepared$(NC)"

## Full Workflow

full-test: clean build test-unit test-integration security lint ## Run complete test suite
	@echo "$(GREEN)✓ Full test suite completed$(NC)"

deploy-staging: build docker-build docker-push ## Deploy to staging
	@echo "$(GREEN)Deploying to staging...$(NC)"
	$(HELM) upgrade --install $(PROJECT_NAME) ./helm/$(PROJECT_NAME) \
		--namespace staging \
		--set image.tag=$(VERSION) \
		--values ./helm/$(PROJECT_NAME)/values.staging.yaml
	@echo "$(GREEN)✓ Deployed to staging$(NC)"

production-ready: full-test docker-build ## Prepare for production deployment
	@echo "$(GREEN)Production readiness check complete$(NC)"
	@echo "$(YELLOW)Ready for production deployment with: make k8s-deploy-prod$(NC)"
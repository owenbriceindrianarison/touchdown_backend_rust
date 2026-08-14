SHELL := /bin/bash
COMPOSE := docker compose -f docker-compose.dev.yml

.DEFAULT_GOAL := help
.PHONY: help setup up down restart logs ps dev keys migrate-auth migrate-catalog \
        migrate-inventory migrate-media migrate test test-unit test-it fmt lint check clean nuke

help: ## Display this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
	 | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

setup: ## First-time setup: .env + PASETO keys
	@test -f .env || cp .env.example .env
	@grep -q '^PASETO_SECRET_KEY=.\+' .env || $(MAKE) keys
	@echo "✅ Setup complete. Run 'make dev'."

keys: ## Generate PASETO keys and inject them into .env
	@cargo run -q -p shared --example gen_keys 2>/dev/null > .paseto.tmp
	@sed -i.bak '/^PASETO_SECRET_KEY=/d;/^PASETO_PUBLIC_KEY=/d' .env && rm -f .env.bak
	@cat .paseto.tmp >> .env && rm -f .paseto.tmp
	@echo "🔑 PASETO keys written to .env"

up: ## Start infrastructure only (Postgres, NATS, Redis, MinIO, Meili)
	$(COMPOSE) up -d postgres-auth postgres-catalog postgres-inventory postgres-media \
	                nats redis minio minio-init meilisearch
	@echo "⏳ Waiting for health checks…" && sleep 3 && $(COMPOSE) ps

dev: up ## Start infrastructure + all application services (hot-reload)
	$(COMPOSE) up -d --build gateway
	$(COMPOSE) logs -f gateway

down: ## Stop everything (volumes are preserved)
	$(COMPOSE) down

nuke: ## Stop everything AND remove volumes (data will be lost)
	$(COMPOSE) down -v

restart: down dev ## Restart the stack

logs: ## Follow logs (make logs S=gateway)
	$(COMPOSE) logs -f $(S)

ps: ## Show container status
	$(COMPOSE) ps

migrate-auth: ## Apply Auth service migrations
	DATABASE_URL=$$(grep '^AUTH_DATABASE_URL=' .env | cut -d= -f2-) \
	  sqlx migrate run --source services/auth/migrations

migrate-catalog: ## Apply Catalog service migrations
	DATABASE_URL=$$(grep '^CATALOG_DATABASE_URL=' .env | cut -d= -f2-) \
	  sqlx migrate run --source services/catalog/migrations

migrate-inventory: ## Apply Inventory service migrations
	DATABASE_URL=$$(grep '^INVENTORY_DATABASE_URL=' .env | cut -d= -f2-) \
	  sqlx migrate run --source services/inventory/migrations

migrate-media: ## Apply Media service migrations
	DATABASE_URL=$$(grep '^MEDIA_DATABASE_URL=' .env | cut -d= -f2-) \
	  sqlx migrate run --source services/media/migrations

migrate: migrate-auth migrate-catalog migrate-inventory migrate-media ## Apply all migrations

test: ## Run all tests (unit + integration tests with testcontainers, requires Docker)
	cargo test --workspace --all-features -- --test-threads=4

test-unit: ## Run unit tests only (fast, no Docker)
	cargo test --workspace --lib

test-it: ## Run integration tests only
	cargo test --workspace --all-features --tests -- --test-threads=4

fmt: ## Format the code
	cargo fmt --all

lint: ## Run Clippy in strict mode
	cargo clippy --workspace --all-features --all-targets -- -D warnings

check: fmt lint test ## Run the same checks as CI

clean: ## Clean build artifacts
	cargo clean && rm -rf target-docker
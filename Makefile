COMPOSE = docker compose -f docker-compose.dev.yml

.PHONY: dev infra down logs ps test fmt lint keys psql

dev:            ## infra + application services  (hot-reload)
	$(COMPOSE) up --build

infra:          ## infrastructure only (Postgres, NATS, Redis, MinIO, Meilisearch)
	$(COMPOSE) up -d

down:
	$(COMPOSE) down

logs:
	$(COMPOSE) logs -f $(S)

ps:
	$(COMPOSE) ps

test:           ## tests workspace (testcontainers → docker required)
	cargo test --workspace --all-features

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

keys:           ## build PASETO keys end paste them in .env
	cargo run -p shared --example gen_keys

psql:           ## make psql S=auth (open psql on auth_db)
	$(COMPOSE) exec $(S)-db psql -U touchdown -d $(S)_db
APP_NAME := raspi-observer
DOCKER_IMAGE := raspi-observer:local
COMPOSE := docker compose

.DEFAULT_GOAL := help

.PHONY: help fmt fmt-check clippy check test run ci \
	docker-build docker-run \
	compose-config compose-build compose-up compose-down compose-logs compose-ps \
	health metrics

help: ## Zeigt verfuegbare Make-Targets
	@grep -E '^[a-zA-Z0-9_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "%-18s %s\n", $$1, $$2}'

fmt: ## Formatiert den Rust-Code
	cargo fmt

fmt-check: ## Prueft Rust-Formatierung ohne Aenderung
	cargo fmt -- --check

clippy: ## Fuehrt Clippy-Pruefung mit Warnungen als Fehler aus
	cargo clippy --all-targets --all-features -- -D warnings

check: ## Prueft Kompilierung
	cargo check

test: ## Fuehrt Tests aus
	cargo test

run: ## Startet die App lokal
	cargo run

ci: fmt-check clippy test ## Lokaler CI-Check

docker-build: ## Baut das Service-Image ueber Dockerfile
	docker build -f deployments/raspi-observer/Dockerfile -t $(DOCKER_IMAGE) .

docker-run: docker-build ## Startet das Image mit Docker-Socket-Mount
	docker run --rm -p 8080:8080 -v /var/run/docker.sock:/var/run/docker.sock $(DOCKER_IMAGE)

compose-config: ## Validiert docker-compose.yml
	$(COMPOSE) config

compose-build: ## Baut Services aus docker-compose.yml
	$(COMPOSE) build

compose-up: ## Startet Services im Hintergrund
	$(COMPOSE) up -d

compose-down: ## Stoppt und entfernt Services
	$(COMPOSE) down

compose-logs: ## Zeigt Compose-Logs (follow)
	$(COMPOSE) logs -f

compose-ps: ## Zeigt Compose-Service-Status
	$(COMPOSE) ps

health: ## Prueft lokalen Health-Endpunkt
	curl -fsS http://127.0.0.1:8080/health

metrics: ## Holt den Prometheus-Metrics-Output lokal
	curl -fsS http://127.0.0.1:8080/metrics

release: ## Releases backend and frontend in parallel
	@echo "🚀 Starting release of raspi-observer..."
	$(MAKE) -j2 raspi-observer-release
	@echo "✅ All services released successfully!"

raspi-observer-build-release: ## Builds the raspi-observer image with version tag
	docker build -f deployments/raspi-observer/Dockerfile -t ottowagner/raspi-observer:$(VERSION) -t ottowagner/raspi-observer:latest .

raspi-observer-release: raspi-observer-build-release ## Releases the raspi-observer image (builds, pushes version and latest)
	docker push ottowagner/raspi-observer:$(VERSION) & \
	docker push ottowagner/raspi-observer:latest & \
	wait

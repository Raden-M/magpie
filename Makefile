# Magpie developer tasks. `make` (or `make help`) lists targets.

export PATH := $(HOME)/.cargo/bin:$(PATH)

.DEFAULT_GOAL := help

.PHONY: help dev dev-firefox extension extension-firefox backend test build docker lint-firefox licenses

help: ## List available targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "} {printf "  \033[1m%-16s\033[0m %s\n", $$1, $$2}'

dev: extension ## Build the Chromium extension, then run the backend (main dev command)
	cd backend && cargo run

dev-firefox: extension-firefox ## Build the Firefox extension, then run the backend
	cd backend && cargo run

extension: ## Install deps and build the Chromium extension into extension/dist
	cd extension && npm install && npm run build

extension-firefox: ## Install deps and build the Firefox extension into extension/dist-firefox
	cd extension && npm install && npm run build:firefox

backend: ## Run the backend only (http://127.0.0.1:8787)
	cd backend && cargo run

build: extension ## Release build of both the backend binary and the extension
	cd backend && cargo build --release

test: ## Run backend and extension tests
	cd backend && cargo test
	cd extension && npm run test

docker: ## Build the release container image
	cd backend && docker build -t magpie:latest .

lint-firefox: extension-firefox ## Run Mozilla's own add-on linter over the Firefox build
	cd extension && npx --yes web-ext lint --source-dir=dist-firefox --self-hosted

licenses: ## Regenerate THIRD-PARTY.md from the dependency graphs
	node scripts/generate-third-party.mjs

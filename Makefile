# Magpie developer tasks. `make` (or `make help`) lists targets.

export PATH := $(HOME)/.cargo/bin:$(PATH)

.DEFAULT_GOAL := help

.PHONY: help check-rust check-node dev extension extension-chromium backend \
        test build docker docker-up docker-down connect lint-firefox licenses

help: ## List available targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "} {printf "  \033[1m%-18s\033[0m %s\n", $$1, $$2}'

# Preflight. Both halves fail obscurely without their toolchain - a missing Node
# shows up as a syntax error from type stripping, not as "your Node is too old" -
# so the check is worth the two seconds.
check-rust:
	@command -v cargo >/dev/null 2>&1 || { \
		echo "magpie: cargo not found. Install Rust from https://rustup.rs"; exit 1; }

check-node:
	@command -v node >/dev/null 2>&1 || { \
		echo "magpie: node not found. Magpie needs Node >= 23.6 (https://nodejs.org)"; exit 1; }
	@node -e 'const [a,b]=process.versions.node.split(".").map(Number); if (a<23 || (a===23 && b<6)) { console.error("magpie: Node >= 23.6 required, found " + process.versions.node + ". The tests run TypeScript directly, which needs native type stripping."); process.exit(1); }'

# Firefox is the shipping target, so it is what the unqualified targets build.
dev: extension ## Build the extension, then run the backend - the one command to start
	@echo
	@echo "  Extension built. Load it once, in Firefox:"
	@echo "    about:debugging  ->  This Firefox  ->  Load Temporary Add-on"
	@echo "    ->  pick any file in extension/dist-firefox/"
	@echo
	@echo "  Then highlight something. No settings to fill in for a local backend."
	@echo
	cd backend && cargo run

extension: check-node ## Build the Firefox extension into extension/dist-firefox
	cd extension && npm install && npm run build:firefox

extension-chromium: check-node ## Build the Chromium extension into extension/dist
	cd extension && npm install && npm run build

backend: check-rust ## Run the backend only (http://127.0.0.1:8787)
	cd backend && cargo run

test: check-rust check-node ## Run backend and extension tests
	cd backend && cargo test
	cd extension && npm run test

build: check-rust extension ## Release build of both the backend binary and the extension
	cd backend && cargo build --release

docker: ## Build the release container image
	cd backend && docker build -t magpie:latest .

# Safe to re-run: an existing .env is never overwritten, so the token survives
# and the extension stays paired.
docker-up: extension ## Run the backend in Docker and build the extension
	@test -f .env || { \
		printf 'MAGPIE_TOKEN=%s\n' "$$(openssl rand -hex 32)" > .env; \
		echo "wrote .env with a fresh MAGPIE_TOKEN"; }
	docker compose up -d
	@echo
	@echo "  Load the extension once, in Firefox:"
	@echo "    about:debugging  ->  This Firefox  ->  Load Temporary Add-on"
	@echo "    ->  pick any file in extension/dist-firefox/"
	@echo
	@$(MAKE) --no-print-directory connect

docker-down: ## Stop the Docker instance (data in ./data is kept)
	docker compose down

# Asks the running instance rather than reassembling the string here - it knows
# its own bind address and whether a proxy hostname is configured.
connect: ## Print the pairing string for the running Docker instance
	@echo "  Then click the Magpie icon in the toolbar and paste this into Backend URL:"
	@printf '    %s\n' "$$(docker compose exec -T magpie /usr/local/bin/magpie --connect)"
	@echo

lint-firefox: extension ## Run Mozilla's own add-on linter over the Firefox build
	cd extension && npx --yes web-ext lint --source-dir=dist-firefox --self-hosted

licenses: check-rust check-node ## Regenerate THIRD-PARTY.md from the dependency graphs
	node scripts/generate-third-party.mjs

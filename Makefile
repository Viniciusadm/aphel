SHELL := /usr/bin/env bash

CARGO := cd back && cargo
NPM := npm --prefix web

.DEFAULT_GOAL := help

.PHONY: help up down modules dev-back dev-web lint fmt api-types build
.PHONY: sdk

help:
	@echo "Targets:"
	@echo "  up         start MySQL and Mailpit (docker compose)"
	@echo "  down       stop the development environment"
	@echo "  modules    generate the module registries from back/aphel.modules.toml"
	@echo "  dev-back   run the backend at http://127.0.0.1:3001"
	@echo "  dev-web    run Vite with /api proxied to the backend"
	@echo "  lint       cargo fmt --check, clippy -D warnings, oxlint, typecheck"
	@echo "  fmt        cargo fmt"
	@echo "  api-types  generate web/src/api/schema.d.ts from the OpenAPI document"
	@echo "  build      release build (web + binary)"
	@echo "  sdk        generate types and build the TypeScript SDK (sdk/typescript)"

up:
	docker compose up -d --wait

down:
	docker compose down

modules:
	$(CARGO) run -q -p xtask -- modules sync

web/node_modules: web/package-lock.json
	$(NPM) ci
	@touch $@

web/dist: | web/node_modules
	$(NPM) run build

dev-back: modules | web/dist
	cd back && cargo run -- serve

dev-web: modules | web/node_modules
	$(NPM) run dev


lint: | web/dist
	$(CARGO) run -q -p xtask -- modules sync --check
	$(CARGO) fmt --all --check
	$(CARGO) clippy --workspace -- -D warnings
	$(NPM) run lint
	$(NPM) run typecheck

fmt:
	$(CARGO) fmt --all

api-types: | web/node_modules
	$(NPM) run api:types

build: modules
	./build.sh

sdk:
	npm --prefix sdk/typescript ci
	npm --prefix sdk/typescript run generate
	npm --prefix sdk/typescript run build

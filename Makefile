.PHONY: dev dev-rs dev-css build prod logs clean

# ── Local development ────────────────────────────────────────────────────────

# Start both watchers together (Ctrl-C kills both via trap)
dev:
	@trap 'kill 0' INT; \
	CONFIG_FILE=config.dev.toml RUST_LOG=debug \
	  cargo watch --watch src --watch templates --watch config.dev.toml -x run & \
	npm run watch:css & \
	wait

# Rust watcher only
dev-rs:
	CONFIG_FILE=config.dev.toml RUST_LOG=debug \
	cargo watch --watch src --watch templates --watch config.dev.toml -x run

# CSS watcher only
dev-css:
	npm run watch:css

# Tailwind one-shot build (minified)
css:
	npm run build:css

# ── Production Docker ────────────────────────────────────────────────────────
build:
	npm run build:css
	docker compose build

prod:
	npm run build:css
	docker compose up -d

logs:
	docker compose logs -f

# ── Misc ─────────────────────────────────────────────────────────────────────
clean:
	cargo clean
	rm -f dev.db dev.db-shm dev.db-wal

fix:
	cargo fix --allow-dirty
	cargo fmt

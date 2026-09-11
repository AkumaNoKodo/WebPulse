TAILWIND_VERSION := 4.3.3
TAILWIND_SHA256  := dc61b3ac6b8c9ca874c0cc4c57b2409791a64c5540404ca5f5367360babc313a
TAILWIND         := bin/tailwindcss

.DELETE_ON_ERROR:
.PHONY: dev dev-rs dev-css css build prod logs lint fmt test clean fix

# ── Tooling ──────────────────────────────────────────────────────────────────

# Standalone Tailwind CLI, version-locked with the css stage in the Dockerfile
$(TAILWIND):
	mkdir -p $(dir $@)
	curl -fsSL -o $@ "https://github.com/tailwindlabs/tailwindcss/releases/download/v$(TAILWIND_VERSION)/tailwindcss-linux-x64"
	echo "$(TAILWIND_SHA256)  $@" | sha256sum -c -
	chmod +x $@

# ── Local development ────────────────────────────────────────────────────────

# Start both watchers together (Ctrl-C kills both via trap)
dev: $(TAILWIND)
	@trap 'kill 0' INT; \
	CONFIG_FILE=config.dev.toml RUST_LOG=debug \
	  cargo watch --watch src --watch templates --watch config.dev.toml -x run & \
	$(TAILWIND) -i ./static/input.css -o ./static/output.css --watch & \
	wait

# Rust watcher only
dev-rs:
	CONFIG_FILE=config.dev.toml RUST_LOG=debug \
	cargo watch --watch src --watch templates --watch config.dev.toml -x run

# CSS watcher only
dev-css: $(TAILWIND)
	$(TAILWIND) -i ./static/input.css -o ./static/output.css --watch

# Tailwind one-shot build (minified)
css: $(TAILWIND)
	$(TAILWIND) -i ./static/input.css -o ./static/output.css --minify

# ── Checks ───────────────────────────────────────────────────────────────────

lint:
	cargo fmt --check
	cargo clippy --all-targets --locked -- -D warnings

fmt:
	cargo fmt

test:
	cargo test --locked

# ── Production Docker ────────────────────────────────────────────────────────

# CSS is built inside the image by the css stage; no host build needed
build:
	docker compose build

prod:
	docker compose up -d

logs:
	docker compose logs -f

# ── Misc ─────────────────────────────────────────────────────────────────────
clean:
	cargo clean
	rm -f dev.db dev.db-shm dev.db-wal static/output.css

fix:
	cargo fix --allow-dirty
	cargo fmt

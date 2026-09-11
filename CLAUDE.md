# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
make dev          # cargo watch (src, templates, config.dev.toml) + Tailwind watcher, both under CONFIG_FILE=config.dev.toml RUST_LOG=debug
make dev-rs       # Rust watcher only
make dev-css      # Tailwind watcher only
make css          # one-shot minified static/output.css
make lint         # cargo fmt --check + cargo clippy --all-targets --locked -- -D warnings
make fix          # cargo fix --allow-dirty && cargo fmt
make build        # docker compose build
make clean        # cargo clean + remove dev.db* and static/output.css

cargo run                       # uses config.toml -> /data/webpulse.db; for local runs prefer CONFIG_FILE=config.dev.toml
cargo test                      # no tests exist yet
```

`make dev` requires `cargo-watch` (`cargo install cargo-watch`). The Tailwind CLI is a standalone binary auto-downloaded to `bin/tailwindcss` by the Makefile and checksum-verified; its version is duplicated in the `Makefile` (`TAILWIND_VERSION`/`TAILWIND_SHA256`, linux-x64) and in the Dockerfile `css` stage (linux-x64-musl, different SHA) — bump both together.

The app will not render correctly until `static/output.css` exists; it is gitignored and produced only by a Tailwind build.

`webpulse --health` performs a local HTTP request against the configured port and exits non-zero on failure. It is the container `HEALTHCHECK` and the Compose healthcheck; `scratch` has no shell, so this is the only way to probe the image.

## Architecture

Single crate, both a library (`src/lib.rs`) and a binary (`src/main.rs`); the binary only wires things up.

`main.rs` reads `CONFIG_FILE` (default `config.toml`), handles `--health`, builds the SQLite pool, runs migrations, constructs `AppState`, spawns the scheduler task, then serves `/api/*` (JSON) merged with the web router (HTML), with graceful shutdown on SIGINT/SIGTERM.

`AppState` (`src/store.rs`) is the single piece of shared state — the SQLite pool, the `[monitor]` config section, and one shared `reqwest::Client`. Both routers are `Router<AppState>` and `main` applies the state once.

`src/store.rs` owns **all** SQL and all input validation. Both presentation layers and the scheduler call it, so a query or a rule exists in exactly one place:

- `src/api/monitor.rs` — REST JSON, errors via `AppError` (`src/error.rs`) which returns `{"error": ...}`.
- `src/web.rs` — the HTMX dashboard: Askama template structs, the `MonitorView` presentation type, the ASCII chart renderer, handlers and the router. Errors go through `WebError`, which renders an HTML fragment rather than JSON, because **htmx 4 swaps error responses into the page**.

`src/services/` is the monitoring engine. `scheduler::run` ticks every `check_batch_interval_ms` and calls `store::claim_due_monitors`, a single `UPDATE ... RETURNING` that marks due monitors as checked and returns them — a probe that outlives its own interval therefore cannot be started twice. Each probe runs in a detached task behind a `Semaphore` sized by `max_concurrent_checks`. `checker::probe` is a free function over the shared client; a new probe type means another function plus a branch in the scheduler.

`src/models/` holds the row types. They derive `FromRow`, and the status enums derive `sqlx::Type` with `rename_all = "lowercase"` so the `TEXT` columns map directly — keep the SQL `CHECK` constraint in `migrations/` and the enum variants in sync.

## Conventions and constraints

- All SQL is runtime `sqlx::query`/`query_as` — no `query!` macros, so no `DATABASE_URL` or offline metadata is needed at build time. sqlx 0.9 only accepts `&'static str`, so a query cannot be assembled with `format!`.
- Migrations are embedded with `sqlx::migrate!("./migrations")` and run at startup; add `NNN_*.sql` files, never edit an applied one.
- Askama is compile-time: every template edit requires a Rust rebuild. Templates live in `templates/`, partials in `templates/components/`, and are pulled in as Tailwind sources via `@source "../templates"` in `static/input.css`.
- The dashboard refreshes by HTMX polling `/partial/stats` and `/partial/monitors`; new interactive fragments should be new `/partial/...` or `/monitors/...` handlers returning HTML fragments, not client-side JS. htmx 4 is vendored at `static/vendor/htmx.min.js`; there is no `package.json` and no Node build — CSS is built only through the Makefile.
- htmx 4 specifics that bit us: events are `htmx:after:request` (so `hx-on::after:request`, not `after-request`), attribute inheritance is explicit, and only 204/304 are exempt from swapping.
- `ServeDir::new("static")` and the SQLite path are resolved relative to the working directory; the container therefore runs with `WORKDIR /app`.
- Container image is `scratch` with a static `x86_64-unknown-linux-musl` binary, running as uid 65534, base images pinned by digest. `.dockerignore` is allowlist-style — a new runtime input must be un-ignored there explicitly.
- There is no authentication. The dashboard and the API are both open to anyone who can reach the port.

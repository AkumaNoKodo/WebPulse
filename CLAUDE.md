# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
make dev          # cargo watch (src, templates, config.dev.toml) + Tailwind watcher, both under CONFIG_FILE=config.dev.toml RUST_LOG=debug
make dev-rs       # Rust watcher only
make dev-css      # Tailwind watcher only
make css          # one-shot minified static/output.css
make fix          # cargo fix --allow-dirty && cargo fmt
make build        # docker compose build
make clean        # cargo clean + remove dev.db* and static/output.css

cargo check
cargo run                       # uses config.toml -> /data/rustpulse.db; for local runs prefer CONFIG_FILE=config.dev.toml
cargo test                      # no tests exist yet
```

`make dev` requires `cargo-watch` (`cargo install cargo-watch`). The Tailwind CLI is a standalone binary auto-downloaded to `bin/tailwindcss` by the Makefile and checksum-verified; its version is duplicated in the `Makefile` (`TAILWIND_VERSION`/`TAILWIND_SHA256`, linux-x64) and in the Dockerfile `css` stage (linux-x64-musl, different SHA) — bump both together.

The app will not render correctly until `static/output.css` exists; it is gitignored and produced only by a Tailwind build.

## Architecture

Single crate, both a library (`src/lib.rs`) and a binary (`src/main.rs`); the binary only wires things up. Crate and binary are still named `rustpulse` — the repository was renamed to WebPulse but the crate name, container name, volume, default DB filename and UI strings were not.

`main.rs` reads `CONFIG_FILE` (default `config.toml`), builds the SQLite pool, runs migrations, spawns the scheduler task, then serves `/api/*` (JSON) merged with the web router (HTML), with graceful shutdown on SIGINT/SIGTERM.

Two independent presentation layers over the same tables:

- `src/api/` — REST JSON, `Router` with `State<Arc<DbPool>>`, errors via `AppError` (`src/error.rs`) which implements `IntoResponse` and returns `{"error": ...}`. Routes: `/api/monitors` CRUD, `/api/heartbeats` CRUD plus `POST /api/heartbeats/ping/{uuid}`.
- `src/web.rs` — one file holding the whole HTMX dashboard: Askama template structs, the `MonitorView`/`HistoryView` presentation types, the ASCII sparkline renderer, all handlers, and the router (`/`, `/partial/*`, `/monitors/*`, `nest_service("/static", ServeDir)`). It uses `State<DbPool>` directly and returns `(StatusCode, String)` tuples rather than `AppError`.

`src/services/` is the monitoring engine. `Scheduler` ticks every `check_batch_interval_ms`, selects monitors whose `last_check_at + check_interval_secs` is due, and spawns a detached task per monitor; `record_check_result` writes a `history` row and updates the monitor. HTTP probing sits behind the `Checkable` trait (`checker.rs`) with `HttpChecker` the only implementation — a new probe type means a new `Checkable` impl plus a branch in `Scheduler::check_due_monitors`. Heartbeats are the inverse (dead man's switch): `check_heartbeat_status` derives `healthy`/`late`/`down` from elapsed time against `expected_interval_secs + grace_period_secs` on every tick; the ping endpoint resets `last_ping_at`.

`src/models/` holds the row types. They implement `FromRow` **manually** because status columns are `TEXT` mapped to enums via `Display`/`From<&str>`; keep the SQL `CHECK` constraint in `migrations/`, the enum, and both conversions in sync.

## Conventions and constraints

- All SQL is runtime `sqlx::query`/`query_as` — no `query!` macros, so no `DATABASE_URL` or offline metadata is needed at build time. The `.env` `DATABASE_URL` is vestigial.
- Migrations are embedded with `sqlx::migrate!("./migrations")` and run at startup; add `NNN_*.sql` files, never edit an applied one.
- Askama is compile-time: every template edit requires a Rust rebuild. Templates live in `templates/`, partials in `templates/components/`, and are pulled in as Tailwind sources via `@source "../templates"` in `static/input.css`.
- The dashboard refreshes by HTMX polling `/partial/stats` and `/partial/monitors` every 10s; new interactive fragments should be new `/partial/...` or `/monitors/...` handlers returning HTML fragments, not client-side JS. htmx is vendored at `static/vendor/htmx.min.js`; there is no `package.json` and no Node build — CSS is built only through the Makefile.
- `ServeDir::new("static")` and the SQLite path are resolved relative to the working directory; the container therefore runs with `WORKDIR /app`.
- Container image is `scratch` with a static `x86_64-unknown-linux-musl` binary, running as uid 65534, base images pinned by digest. `.dockerignore` is allowlist-style — a new runtime input must be un-ignored there explicitly.
- The `[monitor]` config section (`default_timeout_secs`, `default_interval_secs`, `history_retention_count`) is parsed but never read; defaults are hardcoded in the API handlers and history is never pruned.

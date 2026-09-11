# WebPulse

A high-performance, self-hosted uptime monitoring tool written in Rust. Monitor HTTP endpoints in real time with a lightweight web dashboard — no external services required.

![Dashboard](https://img.shields.io/badge/status-active-brightgreen) ![Rust](https://img.shields.io/badge/Rust-1.98%2B-orange)
---

## Features

- **HTTP uptime monitoring** — periodically checks URLs, records status (up/down), response times, and keeps a rolling history
- **Real-time dashboard** — auto-refreshes via HTMX, no JS framework required
- **ASCII response-time charts** — one column per check, red markers for down events
- **REST JSON API** — full CRUD for monitors
- **SQLite storage** — zero-dependency database with automatic migrations at startup
- **Docker ready** — minimal `scratch`-based image built from a static musl binary

---

## Quick Start

### Native

**Prerequisites:** Rust >= 1.98

```bash
# 1. Clone
git clone https://github.com/youruser/webpulse.git
cd webpulse

# 2. Build CSS (required — the dashboard will not render without it)
make css

# 3. Run against the development config (SQLite file ./dev.db)
CONFIG_FILE=config.dev.toml cargo run
```

Open `http://localhost:3000`.

### Docker Compose

```bash
docker compose up --build
```

The SQLite database is persisted to a Docker volume (`webpulse_data`).

### Docker

```bash
docker build -t webpulse .
docker run -p 3000:3000 -v webpulse_data:/data webpulse
```

---

## Configuration

All settings live in `config.toml`; `CONFIG_FILE` selects a different file.

```toml
[server]
host = "0.0.0.0"
port = 3000

[database]
path = "/data/webpulse.db"
max_connections = 10

[scheduler]
max_concurrent_checks = 100      # in-flight HTTP probes
check_batch_interval_ms = 1000   # how often due monitors are claimed

[monitor]
default_timeout_secs = 30        # used when a monitor does not set its own
default_interval_secs = 60
history_retention_count = 100    # checks kept per monitor, and rows the UI shows

[logging]
level = "info"   # error | warn | info | debug | trace
```

---

## API

| Method | Path | Body |
|---|---|---|
| `GET` | `/api/monitors` | — |
| `POST` | `/api/monitors` | `{"name", "url", "check_interval_secs"?, "timeout_secs"?}` |
| `GET` | `/api/monitors/{id}` | — |
| `PUT` | `/api/monitors/{id}` | any subset of the create fields |
| `DELETE` | `/api/monitors/{id}` | — |

`url` must be `http`/`https`; `check_interval_secs` and `timeout_secs` must be at least 1. Invalid input returns `400` with `{"error": ...}`.

---

## Tech Stack

| | |
|---|---|
| Language | Rust 2021 |
| Async runtime | Tokio |
| Web framework | Axum 0.8 |
| Database | SQLite via SQLx 0.9 |
| HTTP client | reqwest 0.13 (rustls) |
| Templating | Askama 0.16 (compile-time Jinja2) |
| Frontend | htmx 4 + Tailwind CSS 4 |
| Container | Docker multi-stage → `scratch` image |

---

## Development

```bash
make dev       # cargo watch + Tailwind watcher
make dev-css   # Tailwind watcher only
make lint      # cargo fmt --check + clippy -D warnings
make clean     # cargo clean, remove dev.db and static/output.css
```

The dashboard is served at `http://localhost:3000`. Template changes require recompilation (Askama is compile-time). CSS changes are picked up automatically when using `make dev-css`.

---

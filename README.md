# RustPulse

A high-performance, self-hosted uptime monitoring tool written in Rust. Monitor HTTP endpoints and heartbeats in real time with a lightweight web dashboard — no external services required.

![Dashboard](https://img.shields.io/badge/status-active-brightgreen) ![Rust](https://img.shields.io/badge/Rust-1.95%2B-orange)
---

## Features

- **HTTP uptime monitoring** — periodically checks URLs, records status (up/down), response times, and maintains a rolling history
- **Heartbeat monitoring** — dead man's switch pattern; external services ping RustPulse to prove they are alive; statuses: `healthy`, `late`, `down`, `unknown`
- **Real-time dashboard** — auto-refreshes every 10 seconds via HTMX, no JS framework required
- **Sparklines** — inline SVG response-time graphs per monitor with red markers for down events
- **REST JSON API** — full CRUD for monitors and heartbeats
- **SQLite storage** — zero-dependency database with automatic migrations at startup
- **Docker ready** — minimal `scratch`-based image built from a static musl binary

---

## Quick Start

### Native

**Prerequisites:** Rust >= 1.95, Node.js + npm

```bash
# 1. Clone
git clone https://github.com/youruser/rustpulse.git
cd rustpulse

# 2. Build CSS (required)
npm install
npm run build:css

# 3. Configure (edit database path for local dev)
# In config.toml set: path = "db.sqlite"

# 4. Run
cargo run --release
```

Open `http://localhost:3000`.

### Docker Compose

```bash
docker compose up --build
```

The SQLite database is persisted to a Docker volume (`rustpulse_data`).

### Docker

```bash
docker build -t rustpulse .
docker run -p 3000:3000 -v rustpulse_data:/data rustpulse
```

---

## Configuration

All settings live in `config.toml`:

```toml
[server]
host = "0.0.0.0"
port = 3000

[database]
path = "/data/rustpulse.db"   # use "db.sqlite" for local dev
max_connections = 10

[scheduler]
max_concurrent_checks = 100
check_batch_interval_ms = 1000

[monitor]
default_timeout_secs = 30
default_interval_secs = 60
history_retention_count = 20

[logging]
level = "info"   # error | warn | info | debug | trace
```
---

## Tech Stack

| | |
|---|---|
| Language | Rust 2021 |
| Async runtime | Tokio |
| Web framework | Axum 0.8 |
| Database | SQLite via SQLx 0.8 |
| HTTP client | reqwest 0.13 (rustls) |
| Templating | Askama 0.15 (compile-time Jinja2) |
| Frontend | HTMX 1.9 + Tailwind CSS v4 |
| Container | Docker multi-stage → `scratch` image |

---

## Development

```bash
# Watch CSS changes
npm run watch:css

# Run with debug logging
RUST_LOG=debug cargo run

# Run tests
cargo test
```

The dashboard is served at `http://localhost:3000`. Template changes require recompilation (Askama is compile-time). CSS changes are picked up automatically when using `watch:css`.

---

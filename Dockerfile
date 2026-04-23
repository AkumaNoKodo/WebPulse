# ── Stage 1: Tailwind CSS build ───────────────────────────────────────────────
FROM node:24-slim AS tailwind

WORKDIR /tw

COPY static/input.css ./static/input.css
COPY templates ./templates

RUN npm install -D @tailwindcss/cli && \
    npx @tailwindcss/cli -i ./static/input.css -o ./static/output.css --minify

# ── Stage 2: Rust build ───────────────────────────────────────────────────────
FROM rust:1.95 AS builder

WORKDIR /build

RUN rustup target add x86_64-unknown-linux-musl && \
    apt-get update && apt-get install -y \
    musl-tools \
    pkg-config

COPY Cargo.toml ./
COPY Cargo.lock ./
COPY src ./src
COPY templates ./templates
COPY migrations ./migrations

RUN cargo build --release --target x86_64-unknown-linux-musl

# Create /data dir here so we can COPY it into scratch (scratch has no shell)
RUN mkdir -p /data

# ── Stage 3: Runtime image ────────────────────────────────────────────────────
FROM scratch

WORKDIR /app

# CA certs for outbound HTTPS checks (copied from builder, no apt needed)
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

# Writable data dir for SQLite (mount a volume over /data in production)
COPY --from=builder /data /data

COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/rustpulse /app/rustpulse
COPY --from=tailwind /tw/static/output.css /app/static/output.css
COPY config.toml /app/config.toml

EXPOSE 3000

ENV RUST_LOG=info

CMD ["/app/rustpulse"]

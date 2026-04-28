# Stage 1: Builder ──────────────────────────────────────────────────────────
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
COPY static ./static

RUN cargo build --release --target x86_64-unknown-linux-musl

# Create /data dir here so we can COPY it into scratch (scratch has no shell)
RUN mkdir -p /data

# Runtime image ────────────────────────────────────────────────────────────
FROM scratch

WORKDIR /app

# CA certs for outbound HTTPS checks (copied from builder, no apt needed)
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

# Writable data dir for SQLite (mount a volume over /data in production)
COPY --from=builder /data /data

COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/rustpulse /app/rustpulse
COPY config.toml /app/config.toml
COPY --from=builder /build/static /app/static

EXPOSE 3000

ENV RUST_LOG=info

CMD ["/app/rustpulse"]

# syntax=docker/dockerfile:1

# CSS build ─────────────────────────────────────────────────────────────────
FROM alpine:3.24@sha256:28bd5fe8b56d1bd048e5babf5b10710ebe0bae67db86916198a6eec434943f8b AS css

ARG TAILWIND_VERSION=4.3.3
ARG TAILWIND_SHA256=a04d34ceacc8f52cbe8920ad846cdeb61d3d0021dba32db0d1f77c9d9fad7a6c

RUN apk add --no-cache ca-certificates curl libgcc libstdc++

RUN curl -fsSL -o /usr/local/bin/tailwindcss \
      "https://github.com/tailwindlabs/tailwindcss/releases/download/v${TAILWIND_VERSION}/tailwindcss-linux-x64-musl" && \
    echo "${TAILWIND_SHA256}  /usr/local/bin/tailwindcss" | sha256sum -c - && \
    chmod +x /usr/local/bin/tailwindcss

WORKDIR /build

COPY static/input.css ./static/input.css
COPY templates ./templates

RUN tailwindcss -i ./static/input.css -o ./static/output.css --minify

# Rust build ────────────────────────────────────────────────────────────────
FROM rust:1.98.1@sha256:462a9af3c54fb4718850d3c602fc0e54452c20b1c12a4e4080fdb001d4b9acbf AS builder

WORKDIR /build

RUN rustup target add x86_64-unknown-linux-musl && \
    apt-get update && apt-get install -y --no-install-recommends \
      musl-tools \
      pkg-config \
 && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml ./
COPY Cargo.lock ./
COPY src ./src
COPY templates ./templates
COPY migrations ./migrations

RUN cargo build --release --locked --target x86_64-unknown-linux-musl

RUN mkdir -p /data

FROM scratch

WORKDIR /app

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder --chown=65534:65534 /data /data
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/rustpulse /app/rustpulse
COPY --from=css /build/static/output.css /app/static/output.css
COPY static/fonts /app/static/fonts
COPY static/vendor /app/static/vendor
COPY config.toml /app/config.toml

USER 65534:65534

EXPOSE 3000

ENV RUST_LOG=info

CMD ["/app/rustpulse"]

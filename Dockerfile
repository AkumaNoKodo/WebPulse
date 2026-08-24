# CSS build ─────────────────────────────────────────────────────────────────
FROM alpine:3.22@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce AS css

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
FROM rust:1.98@sha256:7f7a53a25a0319dd8284e279d529d45759cb384d59b14cc6806132910f45522e AS builder

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

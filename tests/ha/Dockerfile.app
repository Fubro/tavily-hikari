FROM rust:1.91-bookworm AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libsqlite3-dev ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && printf 'fn main() {}\n' > src/main.rs \
    && cargo fetch --locked

COPY src ./src
RUN cargo build --release --locked --bin tavily-hikari

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /srv/app
COPY --from=builder /app/target/release/tavily-hikari /usr/local/bin/tavily-hikari

ENV PROXY_BIND=0.0.0.0 \
    PROXY_PORT=8787 \
    XRAY_BINARY=/bin/true

EXPOSE 8787
ENTRYPOINT ["tavily-hikari"]

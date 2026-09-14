FROM rust:1-bookworm AS builder

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install --no-install-recommends -y ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 --shell /usr/sbin/nologin mcp

WORKDIR /app

COPY --from=builder /build/target/release/linux-mcp /usr/local/bin/linux-mcp

USER mcp

ENTRYPOINT ["/usr/local/bin/linux-mcp"]
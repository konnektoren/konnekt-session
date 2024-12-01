FROM rust:1.82-slim as builder

WORKDIR /usr/src/konnekt-session

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./

COPY src ./src

ENV WEBSOCKET_URL=ws://0.0.0.0:3000

RUN cargo build --release --features server

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    wget \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/konnekt-session/target/release/server /usr/local/bin/

CMD ["server"]

EXPOSE 3000

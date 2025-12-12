FROM rust:latest AS builder

RUN apt-get update -y && apt-get upgrade -y && apt install -y build-essential && apt-get clean

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY shared/Cargo.toml ./shared/
COPY server/Cargo.toml ./server/
COPY client/Cargo.toml ./client/
COPY client_app/Cargo.toml ./client_app/

RUN mkdir -p server/src && \
    echo "fn main() {}" > server/src/main.rs && \
    mkdir -p shared/src && \
    echo "" > shared/src/lib.rs && \
    mkdir -p client/src && \
    echo "" > client/src/lib.rs && \
    mkdir -p client_app/src && \
    echo "fn main() {}" > client_app/src/main.rs

RUN cargo build --release --bin gmod_tcp_server
RUN rm -rf server/src/main.rs shared/src/lib.rs client/src/lib.rs client_app/src/main.rs

COPY . .

RUN cargo build --release --bin gmod_tcp_server

FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y ca-certificates gosu && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/gmod_tcp_server /app/gmod_tcp_server

RUN useradd -m -u 1000 appuser && \
    chown -R appuser:appuser /app

COPY server/docker-entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

ENTRYPOINT ["docker-entrypoint.sh"]

EXPOSE 9060 25565

ENV HOST=0.0.0.0
ENV PORT=25565
ENV API_HOST=0.0.0.0
ENV API_PORT=9060
ENV RUST_LOG=info

CMD ["./gmod_tcp_server"]
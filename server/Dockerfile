FROM rust:latest AS builder

RUN apt-get update -y && apt-get upgrade -y && apt-get install -y build-essential && apt-get clean

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY shared/Cargo.toml ./shared/
COPY server/Cargo.toml ./server/
COPY client/Cargo.toml ./client/
COPY client_app/Cargo.toml ./client_app/

RUN mkdir -p server/src shared/src client/src client_app/src && \
    echo "fn main() {}" > server/src/main.rs && \
    echo "" > shared/src/lib.rs && \
    echo "" > client/src/lib.rs && \
    echo "fn main() {}" > client_app/src/main.rs

RUN cargo build --release --bin gmod_tcp_server && \
    rm -rf server/src shared/src client/src client_app/src

COPY . .

RUN cargo build --release --bin gmod_tcp_server

FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y ca-certificates gosu libc6 && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/gmod_tcp_server /app/gmod_tcp_server

RUN useradd -m -u 1000 appuser && \
    chmod +x /app/gmod_tcp_server && \
    chown -R appuser:appuser /app

COPY server/docker-entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]

EXPOSE 9060 25565

ENV HOST=0.0.0.0
ENV PORT=25565
ENV API_HOST=0.0.0.0
ENV API_PORT=9060
ENV RUST_LOG=info

CMD ["./gmod_tcp_server"]
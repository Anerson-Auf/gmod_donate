FROM rust:latest AS builder

WORKDIR /app

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
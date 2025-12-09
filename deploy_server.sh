#!/bin/bash
set -e

cd "$(dirname "$0")"

sudo apt install -y build-essential pkg-config libssl-dev # Мало ли
cargo build --release -p gmod_tcp_server

mkdir -p ../server
cp target/release/gmod_tcp_server ../server/ 

cat <<EOF > ../server/.env
HOST=0.0.0.0
PORT=25565
API_HOST=0.0.0.0
API_PORT=9060
ALLOWED_ORIGINS=*
API_PASSWORDS=test
EOF
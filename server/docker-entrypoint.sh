#!/bin/sh

mkdir -p /app/data
chown -R appuser:appuser /app/data 2>/dev/null || true

cd /app

if [ ! -f ./gmod_tcp_server ]; then
    echo "ERROR: gmod_tcp_server binary not found in /app"
    ls -la /app
    exit 1
fi

echo "Starting gmod_tcp_server..."
exec gosu appuser "$@"


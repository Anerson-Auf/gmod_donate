#!/bin/sh

echo "Entrypoint script started"
echo "Current user: $(id)"
echo "Working directory: $(pwd)"

mkdir -p /app/data
echo "Created /app/data directory"
chown -R appuser:appuser /app/data 2>/dev/null || echo "Warning: Could not chown /app/data"
chmod 755 /app/data || echo "Warning: Could not chmod /app/data"

cd /app
echo "Changed to /app directory"

if [ ! -f ./gmod_tcp_server ]; then
    echo "ERROR: gmod_tcp_server binary not found in /app"
    ls -la /app
    exit 1
fi

if [ ! -x ./gmod_tcp_server ]; then
    echo "ERROR: gmod_tcp_server is not executable"
    ls -la ./gmod_tcp_server
    chmod +x ./gmod_tcp_server
    echo "Made binary executable"
fi

echo "Checking binary dependencies..."
ldd ./gmod_tcp_server 2>&1 || echo "ldd check failed (static binary?)"

echo "File info:"
ls -la ./gmod_tcp_server

echo "Starting gmod_tcp_server as user appuser (UID: $(id -u appuser))..."
echo "Command: $@"

exec gosu appuser "$@"


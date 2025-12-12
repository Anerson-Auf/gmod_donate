#!/bin/sh
set -e

mkdir -p /app/data
chown -R appuser:appuser /app/data 2>/dev/null || true

exec gosu appuser "$@"


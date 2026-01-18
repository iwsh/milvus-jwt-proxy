#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

echo "Starting docker-compose mock upstream..."
docker compose -f "$ROOT_DIR/dev/docker-compose.yml" up -d --build
trap 'docker compose -f "$ROOT_DIR/dev/docker-compose.yml" down --remove-orphans' EXIT

echo "Waiting for mock-upstream to become healthy..."
for i in {1..20}; do
  if curl -fsS http://localhost:19530/health >/dev/null 2>&1; then
    echo "mock-upstream is up on port 19530"
    break
  fi
  sleep 1
done

if ! curl -fsS http://localhost:19530/health >/dev/null 2>&1; then
  echo "mock-upstream did not start" >&2
  docker compose -f "$ROOT_DIR/dev/docker-compose.yml" logs
  exit 1
fi



echo ""
echo "Mock upstream is ready."
echo "To test:"
echo "1. In another terminal, start the proxy: UPSTREAM_URL=http://localhost:19530 ./target/debug/milvus-auth-gateway"
echo "2. Send a test request: curl -X GET 'http://localhost:8000/' -H 'Authorization: \$(echo -n \"x-jwt-token:TESTTOKEN\" | base64)'"
echo "3. Check the proxy logs and upstream response for 'Bearer TESTTOKEN'"
echo ""

# Start milvus-auth-gateway
echo "Building proxy binary..."
cargo run

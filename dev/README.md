Dev integration environment

This directory contains a minimal docker-compose mock upstream and a helper script to run a quick end-to-end smoke test against the local proxy binary.

Prerequisites
- Docker and docker-compose (Docker Compose V2 as `docker compose`)
- Rust toolchain to build the proxy

Quick start

1. Build the proxy and start the mock upstream:

   cd repo_root
   ./dev/run-integration.sh

2. In another terminal, start the proxy pointing to the mock:

   UPSTREAM_URL=http://localhost:19530 ./target/debug/milvus-auth-gateway

3. Send a test request:

   curl -X GET "http://localhost:8000/" -H "Authorization: $(echo -n 'x-jwt-token:TESTTOKEN' | base64)"

4. Verify the response from the mock upstream contains `Authorization: Bearer TESTTOKEN`.

Notes
- The `docker-compose.yml` starts only the `mock-upstream` container on port 19530.
- The proxy runs locally for faster iteration.
- Adjust ports or env vars if needed.

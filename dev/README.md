Dev integration environment

This directory contains integration test environments for milvus-jwt-proxy.

Prerequisites
- Docker and docker-compose (Docker Compose V2 as `docker compose`)
- Rust toolchain to build the proxy
- Python 3 with pip (for Milvus connection tests)

## Test Environments

### 1. Mock Upstream (Quick Test)

A minimal docker-compose mock upstream for quick end-to-end smoke testing.

**Quick start:**

1. Build the proxy and start the mock upstream:

   ```bash
   cd repo_root
   ./dev/run-integration.sh
   ```

2. In another terminal, start the proxy pointing to the mock:

   ```bash
   UPSTREAM_URL=http://localhost:19530 ./target/debug/milvus-jwt-proxy
   ```

3. Send a test request:

   ```bash
   curl -X GET "http://localhost:8000/" -H "Authorization: $(echo -n 'x-jwt-token:TESTTOKEN' | base64)"
   ```

4. Verify the response from the mock upstream contains `Authorization: Bearer TESTTOKEN`.

**Notes:**
- The `docker-compose.yml` starts only the `mock-upstream` container on port 19530.
- The proxy runs locally for faster iteration.
- Adjust ports or env vars if needed.

### 2. Real Milvus Connection Test

A complete integration test with a real Milvus standalone instance.

**Quick start:**

```bash
cd repo_root
./dev/test-milvus-connection.sh
```

This will:
1. Start a minimal Milvus standalone instance with Docker Compose (including etcd and MinIO)
2. Build and start the milvus-jwt-proxy
3. Run Python tests that connect through the proxy to perform:
   - Connection test with JWT token transformation
   - Collection creation
   - Data insertion
   - Data querying
4. Clean up all resources

**Requirements:**
- Docker with at least 4GB memory available
- Python 3 with pip
- pymilvus package (automatically installed if missing)

**Notes:**
- Initial startup takes 1-2 minutes for Milvus to initialize
- Uses `docker-compose-milvus.yml` configuration
- Proxy is built as a Docker container in this setup
- All data is cleaned up after the test completes

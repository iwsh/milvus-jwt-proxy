#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

echo "========================================"
echo "Milvus JWT Proxy Integration Test"
echo "========================================"
echo ""

# Cleanup function
cleanup() {
    echo ""
    echo "Cleaning up..."
    docker compose -f "$ROOT_DIR/dev/docker-compose-milvus.yml" down -v --remove-orphans
}

trap cleanup EXIT

echo "Starting Docker Compose with Milvus..."
docker compose -f "$ROOT_DIR/dev/docker-compose-milvus.yml" up -d --build

echo ""
echo "Waiting for services to be healthy..."
echo "This may take 1-2 minutes for Milvus to initialize..."

# Wait for Milvus to be healthy
MAX_WAIT=180
ELAPSED=0
while [ $ELAPSED -lt $MAX_WAIT ]; do
    if docker compose -f "$ROOT_DIR/dev/docker-compose-milvus.yml" ps | grep -q "milvus.*healthy"; then
        echo "✓ Milvus is healthy"
        break
    fi
    sleep 5
    ELAPSED=$((ELAPSED + 5))
    echo "Waiting... ($ELAPSED seconds)"
done

if [ $ELAPSED -ge $MAX_WAIT ]; then
    echo "✗ Milvus failed to become healthy within $MAX_WAIT seconds"
    docker compose -f "$ROOT_DIR/dev/docker-compose-milvus.yml" logs milvus
    exit 1
fi

# Wait for proxy to be healthy
sleep 5
if ! docker compose -f "$ROOT_DIR/dev/docker-compose-milvus.yml" ps | grep -q "proxy.*Up"; then
    echo "✗ Proxy failed to start"
    docker compose -f "$ROOT_DIR/dev/docker-compose-milvus.yml" logs proxy
    exit 1
fi
echo "✓ Proxy is running"

echo ""
echo "Running connection tests..."

# Create a temporary Python test script
cat > /tmp/milvus_test.py << 'EOF'
#!/usr/bin/env python3
import sys
import time
import base64

try:
    from pymilvus import connections, Collection, CollectionSchema, FieldSchema, DataType, utility
except ImportError:
    print("Error: pymilvus is not installed. Installing...")
    import subprocess
    subprocess.check_call([sys.executable, "-m", "pip", "install", "pymilvus"])
    from pymilvus import connections, Collection, CollectionSchema, FieldSchema, DataType, utility

def test_connection():
    """Test basic connection to Milvus through the proxy"""
    print("\n1. Testing connection...")
    
    # Generate a test JWT token (base64 encoded "x-jwt-token:test_token_123")
    test_token = base64.b64encode(b"x-jwt-token:test_token_123").decode()
    
    try:
        # Connect through the proxy at localhost:8000
        connections.connect(
            alias="default",
            host="localhost",
            port="8000",
            user="root",
            password=test_token
        )
        print("   ✓ Connected successfully")
        return True
    except Exception as e:
        print(f"   ✗ Connection failed: {e}")
        return False

def test_create_collection():
    """Test creating a collection"""
    print("\n2. Testing collection creation...")
    
    try:
        collection_name = "test_collection"
        
        # Drop if exists
        if utility.has_collection(collection_name):
            utility.drop_collection(collection_name)
        
        # Define schema
        fields = [
            FieldSchema(name="id", dtype=DataType.INT64, is_primary=True, auto_id=False),
            FieldSchema(name="embedding", dtype=DataType.FLOAT_VECTOR, dim=128)
        ]
        schema = CollectionSchema(fields=fields, description="Test collection")
        
        # Create collection
        collection = Collection(name=collection_name, schema=schema)
        print(f"   ✓ Collection '{collection_name}' created successfully")
        return True
    except Exception as e:
        print(f"   ✗ Collection creation failed: {e}")
        return False

def test_insert_data():
    """Test inserting data into collection"""
    print("\n3. Testing data insertion...")
    
    try:
        collection_name = "test_collection"
        collection = Collection(name=collection_name)
        
        # Prepare data
        import random
        entities = [
            [i for i in range(10)],  # IDs
            [[random.random() for _ in range(128)] for _ in range(10)]  # Embeddings
        ]
        
        # Insert data
        insert_result = collection.insert(entities)
        collection.flush()
        print(f"   ✓ Inserted {len(entities[0])} entities successfully")
        return True
    except Exception as e:
        print(f"   ✗ Data insertion failed: {e}")
        return False

def test_query():
    """Test querying data"""
    print("\n4. Testing data query...")
    
    try:
        collection_name = "test_collection"
        collection = Collection(name=collection_name)
        
        # Query
        results = collection.query(
            expr="id in [0, 1, 2]",
            output_fields=["id"]
        )
        
        print(f"   ✓ Query returned {len(results)} results")
        return True
    except Exception as e:
        print(f"   ✗ Query failed: {e}")
        return False

def cleanup():
    """Clean up test resources"""
    print("\n5. Cleaning up test data...")
    try:
        if utility.has_collection("test_collection"):
            utility.drop_collection("test_collection")
        connections.disconnect("default")
        print("   ✓ Cleanup completed")
        return True
    except Exception as e:
        print(f"   ✗ Cleanup failed: {e}")
        return False

def main():
    print("=" * 50)
    print("Milvus Connection Test via JWT Proxy")
    print("=" * 50)
    
    tests = [
        test_connection,
        test_create_collection,
        test_insert_data,
        test_query,
        cleanup
    ]
    
    passed = 0
    failed = 0
    
    for test in tests:
        if test():
            passed += 1
        else:
            failed += 1
    
    print("\n" + "=" * 50)
    print(f"Test Results: {passed} passed, {failed} failed")
    print("=" * 50)
    
    if failed > 0:
        sys.exit(1)
    else:
        print("\n✓ All tests passed!")
        sys.exit(0)

if __name__ == "__main__":
    main()
EOF

# Install pymilvus if not available and run the test
python3 /tmp/milvus_test.py

echo ""
echo "========================================"
echo "✓ Integration test completed successfully!"
echo "========================================"

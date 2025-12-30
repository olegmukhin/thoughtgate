CLUSTER := "kind"

# Run integration tests
test-kind: ensure-cluster build load
    @echo "🚀 Running Integration Tests..."
    cargo test --test integration_k8s -- --nocapture

# Build Docker image
build:
    docker build -t thoughtgate:test .

# Load image into Kind cluster
load:
    kind load docker-image thoughtgate:test --name {{CLUSTER}}

# Ensure Kind cluster exists
ensure-cluster:
    @kind get clusters | grep -q ^{{CLUSTER}}$ || kind create cluster --name {{CLUSTER}}

# Clean up cluster and image
clean:
    kind delete cluster --name {{CLUSTER}}
    docker rmi thoughtgate:test || true

# Quick rebuild and test
quick: ensure-cluster build load
    cargo test --test integration_k8s -- --nocapture

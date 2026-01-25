---
sidebar_position: 2
---

# Install ThoughtGate

## From Source

### Prerequisites

- Rust 1.75 or later
- Cargo (included with Rust)

### Build

```bash
git clone https://github.com/thoughtgate/thoughtgate
cd thoughtgate
cargo build --release
```

The binary is at `target/release/thoughtgate`.

### Install Globally

```bash
cargo install --path .
```

## Docker

### Pull the Image

```bash
docker pull ghcr.io/thoughtgate/thoughtgate:latest
```

### Available Tags

| Tag | Description |
|-----|-------------|
| `latest` | Latest stable release |
| `v0.2.0` | Specific version |
| `main-abc1234` | Main branch build |

### Run

```bash
docker run -d \
  --name thoughtgate \
  -p 8080:8080 \
  -p 8081:8081 \
  -e THOUGHTGATE_UPSTREAM_URL=http://host.docker.internal:3000 \
  ghcr.io/thoughtgate/thoughtgate:latest
```

## Kubernetes

See [Deploy to Kubernetes](/docs/how-to/deploy-kubernetes) for Helm charts and manifests.

## Verify Installation

```bash
# Check version
thoughtgate --version

# Check health (after starting)
curl http://localhost:8081/health
```

## System Requirements

| Resource | Minimum | Recommended |
|----------|---------|-------------|
| Memory | 20 MB | 50 MB |
| CPU | 0.1 cores | 0.5 cores |
| Disk | 15 MB (binary) | 15 MB |

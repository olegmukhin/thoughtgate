# DevContainer Setup Guide

## Quick Start (3 Steps)

### 1. Install Prerequisites

**VS Code + Extension:**
```bash
# Install VS Code if not already installed
# Then install the Dev Containers extension:
code --install-extension ms-vscode-remote.remote-containers
```

**Docker Desktop:**
- Download from: [Docker Desktop](https://www.docker.com/products/docker-desktop/)
- Ensure it's running: `docker ps` should work

### 2. Open in Container

In VS Code:
1. Open the ThoughtGate project folder
2. Press `F1` (or `Cmd+Shift+P` on Mac)
3. Type: "Dev Containers: Reopen in Container"
4. Press Enter

**First-time build takes 5-10 minutes** (downloads Rust image + installs tools)

### 3. Verify Setup

Once the container is running, open a new terminal in VS Code and run:

```bash
bash .devcontainer/validate.sh
```

This will verify all tools are installed and working.

---

## What Happens Automatically

The devcontainer will:

1. **Build Container** (first time only)
   - Pull Debian-based Rust image (~500MB)
   - Install system dependencies (valgrind, perf, etc.)
   - Set up Rust stable + nightly toolchains

2. **Run Post-Create Script** (after every rebuild)
   - Install cargo-fuzz (for L3 Verification)
   - Install cargo-nextest (faster tests)
   - Install cargo-watch (auto-rebuild)
   - Install mantra (for L1 Verification)
   - Download all project dependencies
   - Build the project
   - Run test suite

3. **Configure VS Code**
   - Install rust-analyzer, CodeLLDB, and other extensions
   - Enable clippy on save
   - Configure formatting
   - Set up debugger

---

## Troubleshooting

### "Cannot connect to Docker"

**Cause:** Docker Desktop not running

**Fix:**
```bash
# Start Docker Desktop, then verify:
docker ps
```

### "Container build fails"

**Cause:** Insufficient Docker resources

**Fix:**
1. Open Docker Desktop → Settings → Resources
2. Increase Memory to 4GB+ and Disk to 20GB+
3. Click "Apply & Restart"
4. In VS Code: `F1` → "Dev Containers: Rebuild Container"

### "cargo-fuzz not found"

**Cause:** Post-create script didn't complete

**Fix:**
```bash
# Inside container, re-run:
bash .devcontainer/post-create.sh
```

### "mantra command not found"

**Cause:** Mantra installation may fail (known upstream issue)

**Fix:**
```bash
# Inside container:
cargo install mantra --force

# Or continue without it (manual traceability checking via grep)
grep -r "Implements: REQ-" src/
```

### "Tests fail after opening"

**Cause:** Cached build artifacts from host may be incompatible

**Fix:**
```bash
# Inside container:
cargo clean
cargo build
cargo test
```

---

## Verifying the Setup

### Quick Check

```bash
# All of these should work:
rustc --version
cargo --version
cargo +nightly --version
cargo fuzz --version
cargo nextest --version
```

### Run Verification Hierarchy

From `.cursor/rules/base.mdc`:

```bash
# L0: Functional Correctness
cargo test

# L1: Traceability (if mantra installed)
mantra check || grep -r "Implements: REQ-" src/

# L2: Property-Based Testing
cargo test --test prop_* || echo "No prop tests yet"

# L3: Fuzzing
cargo +nightly fuzz list
cargo +nightly fuzz run peeking_fuzz -- -max_total_time=30

# L4: Idiomatic Rust
cargo clippy -- -D warnings

# L5: Formal Verification (not yet configured)
# cargo kani
```

### Memory Profiling

```bash
# Run memory profile test (REQ-CORE-001 Section 5)
/usr/bin/time -v cargo test --test memory_profile -- --nocapture 2>&1 | grep "Maximum resident set size"
```

### Benchmarks

```bash
# Run TTFB benchmarks
cargo bench
```

---

## Development Workflow

### Running the Proxy

```bash
# Build and run (listens on 8080)
cargo run -- --listen-addr 127.0.0.1:8080 --upstream https://api.openai.com

# Or use the Justfile
just run
```

### Auto-Testing

```bash
# Watch for changes and auto-run tests
cargo watch -x test

# Watch and run specific test
cargo watch -x 'test test_peeking_forward_no_buffering'
```

### Debugging

1. Set a breakpoint in VS Code (click left of line number)
2. Press `F5` or Run → Start Debugging
3. CodeLLDB will attach to the process

### Running Integration Tests

```bash
# Kubernetes integration test
cargo test --test integration_k8s

# Streaming integration tests
cargo test --test integration_streaming
```

---

## Container Details

### Exposed Ports

- **8080** - ThoughtGate proxy (main application)
- **8081** - Mock LLM server (for testing)

VS Code will automatically forward these ports from the container to your host.

### Mounted Volumes

- **Project directory** → `/workspaces/thoughtgate`
- **Git config** → `~/.gitconfig` (read-only, for commit authorship)

### User

- Container runs as `vscode` user (non-root)
- Has sudo access if needed: `sudo apt-get install <package>`

---

## Customization

### Adding VS Code Extensions

Edit `.devcontainer/devcontainer.json`:

```json
"customizations": {
  "vscode": {
    "extensions": [
      "rust-lang.rust-analyzer",
      "your-extension-id"
    ]
  }
}
```

Then rebuild: `F1` → "Dev Containers: Rebuild Container"

### Installing Additional Tools

Edit `.devcontainer/post-create.sh` and add:

```bash
echo "Installing my-tool..."
cargo install my-tool
```

### Changing Rust Version

Edit `.devcontainer/Dockerfile`:

```dockerfile
# Pin to specific Rust version
RUN rustup default 1.75.0
```

---

## CI/CD Integration

The Dockerfile can be used in CI pipelines for consistency:

```yaml
# .github/workflows/ci.yml
jobs:
  test:
    runs-on: ubuntu-latest
    container:
      image: mcr.microsoft.com/devcontainers/rust:1-bookworm
    steps:
      - uses: actions/checkout@v3
      - run: rustup toolchain install nightly
      - run: cargo install cargo-fuzz
      - run: cargo test
      - run: cargo clippy -- -D warnings
      - run: cargo +nightly fuzz run peeking_fuzz -- -max_total_time=30
```

---

## Next Steps

1. **Open in Container** - `F1` → "Dev Containers: Reopen in Container"
2. **Run validation** - `bash .devcontainer/validate.sh`
3. **Start developing** - All tools are ready!

For more details, see [README.md](README.md).


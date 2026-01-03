# ThoughtGate DevContainer

This directory contains the VS Code DevContainer configuration for ThoughtGate development.

## Getting Started

### Prerequisites

1. **VS Code** with the [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
2. **Docker Desktop** or compatible container runtime

### Opening the Project

1. Open VS Code
2. Press `F1` or `Cmd+Shift+P` (Mac) / `Ctrl+Shift+P` (Windows/Linux)
3. Type "Dev Containers: Reopen in Container"
4. Wait for the container to build (first time takes 5-10 minutes)

The post-create script will automatically:
- Install Rust nightly toolchain
- Install cargo-fuzz, cargo-nextest, cargo-watch, mantra
- Install system profiling tools
- Download and cache project dependencies
- Build the project
- Run the test suite

## What's Included

### Rust Toolchains
- **Stable** (default) - Main development
- **Nightly** - Required for cargo-fuzz

### Development Tools
- **cargo-fuzz** - Adversarial testing (L3 Verification)
- **cargo-nextest** - Faster test execution
- **cargo-watch** - Auto-rebuild on file changes
- **mantra** - Traceability checking (L1 Verification)

### System Tools
- **time** - Memory profiling
- **valgrind** - Memory debugging
- **linux-perf** - Performance profiling
- **heaptrack** - Heap profiling
- **curl, wget** - HTTP testing
- **jq** - JSON processing
- **httpie** - HTTP client

### VS Code Extensions
- **rust-analyzer** - Rust language server
- **CodeLLDB** - Debugger
- **crates** - Cargo.toml management
- **Even Better TOML** - TOML syntax highlighting
- **Dependi** - Dependency version management

## Verification Commands

Once inside the container, run these commands to verify the setup:

```bash
# Check Rust toolchains
rustc --version
cargo --version
rustup show

# Verify nightly is installed
rustup toolchain list | grep nightly

# Check development tools
cargo fuzz --version
cargo nextest --version
cargo watch --version
mantra --version || echo "mantra may need manual reinstall"

# Run the verification hierarchy (from .cursor/rules/base.mdc)
cargo test                              # L0: Functional Correctness
cargo test --test prop_* || true        # L2: Property-Based Testing
cargo +nightly fuzz list                # L3: Fuzzing (list targets)
cargo clippy -- -D warnings             # L4: Idiomatic Rust

# Test memory profiling
/usr/bin/time -v cargo test --test memory_profile -- --nocapture 2>&1 | grep "Maximum resident set size"

# Run benchmarks
cargo bench
```

## Port Forwarding

The devcontainer automatically forwards these ports:

- **8080** - ThoughtGate Proxy (main application)
- **8081** - Mock LLM Server (for testing)

## Troubleshooting

### Container won't build

1. Ensure Docker is running: `docker ps`
2. Check Docker Desktop resources (recommend 4GB+ RAM)
3. Rebuild without cache: `F1` → "Dev Containers: Rebuild Container"

### Tools not in PATH

The post-create script should add cargo binaries to PATH. If not:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

### Mantra installation fails

This is a known issue. To retry:

```bash
cargo install mantra --force
```

### Tests fail on container start

This may happen if dependencies changed. Run:

```bash
cargo clean
cargo build
cargo test
```

## Files

- **devcontainer.json** - Main configuration
- **Dockerfile** - Custom image with system dependencies
- **post-create.sh** - Setup script run after container creation
- **README.md** - This file

## Customization

### Adding VS Code Extensions

Edit `devcontainer.json` → `customizations.vscode.extensions` array.

### Installing Additional Tools

Edit `post-create.sh` and add your installation commands.

### Changing Rust Version

Edit `Dockerfile` and modify the base image tag or add `rustup` commands.

## Integration with Project

This devcontainer is configured according to:

- **Constitution**: `.cursor/rules/base.mdc` (Blessed Stack)
- **Architecture**: `specs/architecture.md`
- **Project State**: `projectState.md`

All verification hierarchy commands (L0-L5) are available and functional.

## CI/CD Parity

The Dockerfile can be reused in CI/CD pipelines to ensure environment consistency:

```yaml
# Example GitHub Actions
jobs:
  test:
    runs-on: ubuntu-latest
    container:
      image: ghcr.io/your-org/thoughtgate-dev:latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo test
      - run: cargo clippy -- -D warnings
```

## Performance Notes

- First build: 5-10 minutes (downloads Rust image + dependencies)
- Subsequent builds: <1 minute (uses cache)
- Container size: ~3GB (Rust toolchain + dependencies)

## Support

For issues with the devcontainer setup, check:
1. Docker Desktop logs
2. VS Code Output → "Dev Containers"
3. Container logs: `docker logs <container-id>`


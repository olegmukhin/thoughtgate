# ThoughtGate DevContainer Dockerfile
# Extends the official Microsoft Rust devcontainer image with additional tools

FROM mcr.microsoft.com/devcontainers/rust:1-bookworm

# Install system dependencies required for ThoughtGate
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
    # Build essentials
    build-essential \
    pkg-config \
    libssl-dev \
    # Profiling and debugging tools
    time \
    valgrind \
    linux-perf \
    heaptrack \
    # Networking tools for testing
    curl \
    wget \
    netcat-openbsd \
    # Additional utilities
    jq \
    httpie \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

# Ensure cargo bin is in PATH for all users
ENV PATH="/usr/local/cargo/bin:${PATH}"

# Set up Rust environment variables
ENV RUST_BACKTRACE=1
ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup

# Pre-install commonly used Rust components
RUN rustup component add clippy rustfmt rust-src

# Create workspace directory
RUN mkdir -p /workspaces/thoughtgate && chown -R vscode:vscode /workspaces

# Switch to vscode user for remaining operations
USER vscode

# Set working directory
WORKDIR /workspaces/thoughtgate


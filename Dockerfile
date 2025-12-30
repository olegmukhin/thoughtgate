FROM rust:1.92 as builder
WORKDIR /app
COPY . .
# Build both the main proxy and the mock binary (mock_llm requires the 'mock' feature)
RUN cargo build --release --bin thoughtgate && \
    cargo build --release --bin mock_llm --features mock

FROM gcr.io/distroless/cc-debian12
# Copy both binaries
COPY --from=builder /app/target/release/thoughtgate /
COPY --from=builder /app/target/release/mock_llm /

ENTRYPOINT ["/thoughtgate"]

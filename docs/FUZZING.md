# Fuzzing Guide for REQ-CORE-001

## Overview

This guide explains how to run fuzz testing for REQ-CORE-001 Section 5:
- **Goal:** Verify that malformed chunks or interrupted streams don't cause panics or unbounded buffering
- **Tool:** cargo-fuzz with libFuzzer
- **Target:** `peeking_fuzz` - Tests HTTP parsing and chunk handling

## Prerequisites

### Install cargo-fuzz

```bash
cargo install cargo-fuzz
```

### Install Rust Nightly

Fuzzing requires nightly Rust:

```bash
rustup install nightly
```

## Running Fuzz Tests

### Basic Usage

```bash
# Run fuzz target for 30 seconds (REQ-CORE-001 requirement)
cargo +nightly fuzz run peeking_fuzz -- -max_total_time=30

# Run for 5 minutes for more thorough testing
cargo +nightly fuzz run peeking_fuzz -- -max_total_time=300

# Run with specific number of iterations
cargo +nightly fuzz run peeking_fuzz -- -runs=1000000
```

### Advanced Options

```bash
# Run with multiple workers (parallel fuzzing)
cargo +nightly fuzz run peeking_fuzz -- -jobs=4 -workers=4

# Run with memory limit (prevent OOM)
cargo +nightly fuzz run peeking_fuzz -- -rss_limit_mb=2048

# Run with verbose output
cargo +nightly fuzz run peeking_fuzz -- -verbosity=2

# Run with custom corpus directory
cargo +nightly fuzz run peeking_fuzz -- -artifact_prefix=fuzz/artifacts/
```

### With Address Sanitizer

```bash
# Build with address sanitizer to detect memory issues
cargo +nightly fuzz run peeking_fuzz --sanitizer=address -- -max_total_time=30
```

## Interpreting Results

### Success (No Issues Found)

```
#1000000 DONE   cov: 156 ft: 234 corp: 45/1234b
```

- ✅ **PASS:** Completed without crashes or hangs
- `cov`: Code coverage (higher is better)
- `ft`: Features covered
- `corp`: Corpus size (interesting inputs found)

### Failure (Crash Detected)

```
==12345==ERROR: AddressSanitizer: heap-buffer-overflow
```

- ❌ **FAIL:** Found a crash or memory safety issue
- Check `fuzz/artifacts/peeking_fuzz/` for crash inputs
- Reproduce with: `cargo +nightly fuzz run peeking_fuzz fuzz/artifacts/peeking_fuzz/crash-xyz`

### Timeout/Hang

```
ALARM: working on the last Unit for N seconds
```

- ⚠️ **WARNING:** Possible infinite loop or very slow path
- Check if input causes unbounded buffering

## Reproducing Crashes

If fuzzing finds a crash:

```bash
# List artifacts
ls fuzz/artifacts/peeking_fuzz/

# Reproduce specific crash
cargo +nightly fuzz run peeking_fuzz fuzz/artifacts/peeking_fuzz/crash-abc123

# Debug with RUST_BACKTRACE
RUST_BACKTRACE=1 cargo +nightly fuzz run peeking_fuzz fuzz/artifacts/peeking_fuzz/crash-abc123

# Run in debugger
cargo +nightly fuzz run --debug peeking_fuzz fuzz/artifacts/peeking_fuzz/crash-abc123
lldb target/debug/peeking_fuzz fuzz/artifacts/peeking_fuzz/crash-abc123
```

## Building Initial Corpus

You can seed the fuzzer with known-good inputs:

```bash
# Create corpus directory
mkdir -p fuzz/corpus/peeking_fuzz

# Add sample HTTP requests
echo "GET / HTTP/1.1\r\nHost: example.com\r\n\r\n" > fuzz/corpus/peeking_fuzz/http_get
echo "POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n" > fuzz/corpus/peeking_fuzz/chunked
echo "CONNECT example.com:443 HTTP/1.1\r\n\r\n" > fuzz/corpus/peeking_fuzz/connect

# Run fuzzer (will use and expand corpus)
cargo +nightly fuzz run peeking_fuzz
```

## CI Integration

### GitHub Actions Example

```yaml
- name: Install Nightly Rust
  run: rustup install nightly

- name: Install cargo-fuzz
  run: cargo install cargo-fuzz

- name: Run Fuzzer (30 seconds per REQ-CORE-001)
  run: |
    cargo +nightly fuzz run peeking_fuzz -- -max_total_time=30 -verbosity=1
    
    # Check for crashes
    if [ -d "fuzz/artifacts/peeking_fuzz" ] && [ "$(ls -A fuzz/artifacts/peeking_fuzz)" ]; then
      echo "❌ FAIL: Fuzzer found crashes"
      ls -la fuzz/artifacts/peeking_fuzz/
      exit 1
    else
      echo "✅ PASS: No crashes found"
    fi
```

## What the Fuzzer Tests

The `peeking_fuzz` target tests:

1. **HTTP Method Parsing** - Random bytes interpreted as HTTP methods
2. **Chunk Size Parsing** - Invalid hex chunk sizes
3. **Streaming Behavior** - Partial reads and EOF handling
4. **Memory Bounds** - No unbounded allocation from fuzzy input
5. **Header Injection** - CRLF injection attempts

### Expected Behavior

- ✅ Gracefully reject invalid input
- ✅ No panics on malformed data
- ✅ Bounded memory usage (< 10MB even with fuzzy input)
- ✅ No infinite loops
- ✅ No buffer overflows

## Traceability

- **Implements:** REQ-CORE-001 Section 5 (Fuzzing)
- **Requirement:** Parser must not panic or crash on random byte noise
- **Command:** `cargo fuzz run peeking_fuzz -- -max_total_time=30`
- **Fuzz Target:** `fuzz/fuzz_targets/peeking_fuzz.rs`

## Troubleshooting

### "error: no such subcommand: `fuzz`"

Install cargo-fuzz: `cargo install cargo-fuzz`

### "error: toolchain 'nightly' is not installed"

Install nightly: `rustup install nightly`

### "out of memory" during fuzzing

Reduce RSS limit: `cargo +nightly fuzz run peeking_fuzz -- -rss_limit_mb=512`

### Fuzzer runs but doesn't find new paths

- Expand corpus with more diverse inputs
- Run for longer time
- Check code coverage: `cargo +nightly fuzz coverage peeking_fuzz`

## References

- [cargo-fuzz documentation](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- [libFuzzer options](https://llvm.org/docs/LibFuzzer.html#options)
- REQ-CORE-001: Zero-Copy Peeking Strategy


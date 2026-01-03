# Memory Profiling Guide for REQ-CORE-001

## Overview

This guide explains how to verify REQ-CORE-001 Section 5 memory requirements:
- **Goal:** Peak RSS delta < 5MB when streaming 100MB payload
- **Rationale:** Zero-copy streaming should only buffer small chunks (8KB), not accumulate entire payloads

## Running Memory Profile Tests

### Test Suite

We have three memory profiling tests in `tests/memory_profile.rs`:

1. **`test_baseline_memory`** - Establishes baseline RSS without streaming
2. **`test_100mb_stream_memory_profile`** - Streams 100MB, measures memory
3. **`test_memory_bounded_by_buffer`** - Verifies memory is bounded by buffer size

### macOS

#### Using `/usr/bin/time`

```bash
# Run baseline test
/usr/bin/time -l cargo test --test memory_profile test_baseline_memory -- --nocapture 2>&1 | grep "maximum resident set size"

# Run 100MB streaming test
/usr/bin/time -l cargo test --test memory_profile test_100mb_stream_memory_profile -- --nocapture 2>&1 | grep "maximum resident set size"
```

**Expected output:**
```
maximum resident set size: ~15MB (baseline)
maximum resident set size: ~18MB (streaming)
Delta: ~3MB ✅ (< 5MB requirement)
```

#### Using Instruments (Advanced)

```bash
# Install cargo-instruments
cargo install cargo-instruments

# Profile with Allocations template
cargo instruments --template Allocations --test memory_profile -- test_100mb_stream_memory_profile

# Profile with Leaks template
cargo instruments --template Leaks --test memory_profile -- test_100mb_stream_memory_profile
```

### Linux

#### Using `/usr/bin/time`

```bash
# Run baseline test
/usr/bin/time -v cargo test --test memory_profile test_baseline_memory -- --nocapture 2>&1 | grep "Maximum resident set size"

# Run 100MB streaming test
/usr/bin/time -v cargo test --test memory_profile test_100mb_stream_memory_profile -- --nocapture 2>&1 | grep "Maximum resident set size"
```

#### Using heaptrack (Advanced)

```bash
# Install heaptrack
sudo apt-get install heaptrack  # Debian/Ubuntu
sudo dnf install heaptrack      # Fedora

# Run with heaptrack
heaptrack cargo test --test memory_profile test_100mb_stream_memory_profile -- --nocapture

# Analyze results
heaptrack_gui heaptrack.cargo.*.gz
```

### Windows

#### Using Windows Performance Toolkit

```powershell
# Install Windows Performance Toolkit (part of Windows SDK)

# Run test with memory tracking
wpr -start CPU -start ReferenceSet

cargo test --test memory_profile test_100mb_stream_memory_profile -- --nocapture

wpr -stop memory_profile.etl

# Analyze with Windows Performance Analyzer
wpa memory_profile.etl
```

## Interpreting Results

### Success Criteria

✅ **PASS:** Peak RSS delta < 5MB between baseline and streaming tests

❌ **FAIL:** Peak RSS delta >= 5MB (indicates buffering/accumulation)

### What to Look For

1. **Baseline RSS** (~10-20MB) - Runtime + test infrastructure overhead
2. **Streaming RSS** (~15-25MB) - Should only be ~3-5MB higher than baseline
3. **Delta** - The difference should be < 5MB

### Common Issues

**Problem:** RSS delta > 5MB

**Possible causes:**
- Accumulating chunks into `Vec<u8>`
- Not releasing buffers between chunks
- Memory leak in streaming path

**Solution:** Review zero-copy implementation, ensure using `bytes::Bytes` and immediate forwarding

## CI Integration

### GitHub Actions Example

```yaml
- name: Memory Profile Test
  run: |
    /usr/bin/time -v cargo test --test memory_profile test_100mb_stream_memory_profile -- --nocapture > memory_output.txt 2>&1
    
    # Extract RSS and verify < threshold
    BASELINE_RSS=$(grep "Maximum resident set size" baseline_output.txt | awk '{print $6}')
    STREAM_RSS=$(grep "Maximum resident set size" memory_output.txt | awk '{print $6}')
    DELTA=$((STREAM_RSS - BASELINE_RSS))
    
    echo "Baseline RSS: ${BASELINE_RSS}KB"
    echo "Streaming RSS: ${STREAM_RSS}KB"
    echo "Delta: ${DELTA}KB"
    
    if [ $DELTA -gt 5120 ]; then
      echo "❌ FAIL: Memory delta ${DELTA}KB exceeds 5MB threshold"
      exit 1
    else
      echo "✅ PASS: Memory delta ${DELTA}KB is within limits"
    fi
```

## Traceability

- **Implements:** REQ-CORE-001 Section 5 (Memory Profile)
- **Requirement:** Peak RSS delta < 5MB for 100MB payload
- **Test File:** `tests/memory_profile.rs`
- **Documentation:** This file

## References

- REQ-CORE-001: Zero-Copy Peeking Strategy
- Hyper BodyStream documentation
- Tokio AsyncRead/AsyncWrite performance guide


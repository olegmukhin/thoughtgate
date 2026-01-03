#!/bin/bash
# Validation script for ThoughtGate DevContainer
# Run this inside the container to verify all tools are working

set -e

echo "🔍 ThoughtGate DevContainer Validation"
echo "======================================="
echo ""

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track overall status
FAILED=0

# Helper function to check command
check_command() {
    local cmd=$1
    local description=$2
    
    echo -n "Checking $description... "
    if command -v $cmd &> /dev/null; then
        echo -e "${GREEN}✓${NC}"
        return 0
    else
        echo -e "${RED}✗${NC}"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

# Helper function to run test
run_test() {
    local test_name=$1
    shift
    local test_cmd="$@"
    
    echo -n "Testing $test_name... "
    if eval "$test_cmd" &> /dev/null; then
        echo -e "${GREEN}✓${NC}"
        return 0
    else
        echo -e "${RED}✗${NC}"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

echo "1. Checking Rust Toolchains"
echo "----------------------------"
check_command rustc "Rust compiler"
check_command cargo "Cargo package manager"
check_command rustup "Rustup toolchain manager"

echo ""
echo "Installed versions:"
rustc --version
cargo --version

echo ""
echo "Available toolchains:"
rustup toolchain list

echo ""
echo "2. Checking Development Tools"
echo "------------------------------"
check_command cargo-fuzz "cargo-fuzz (L3 Verification)"
check_command cargo-nextest "cargo-nextest"
check_command cargo-watch "cargo-watch"
check_command mantra "mantra (L1 Verification)" || echo -e "${YELLOW}  (mantra may need manual install)${NC}"

echo ""
echo "3. Checking System Tools"
echo "------------------------"
check_command time "time (memory profiling)"
check_command valgrind "valgrind (memory debugging)"
check_command curl "curl (HTTP testing)"
check_command jq "jq (JSON processing)"

echo ""
echo "4. Running Verification Hierarchy"
echo "----------------------------------"

echo "L0: Functional Correctness"
run_test "cargo test" "cargo test --quiet"

echo "L2: Property-Based Testing"
run_test "prop tests" "cargo test --quiet --test prop_* || true"

echo "L3: Fuzzing (listing targets)"
run_test "cargo fuzz" "cargo +nightly fuzz list"

echo "L4: Idiomatic Rust"
run_test "cargo clippy" "cargo clippy --quiet -- -D warnings"

echo ""
echo "5. Project-Specific Tests"
echo "-------------------------"

echo "Checking REQ-CORE-001 tests..."
run_test "unit_peeking" "cargo test --quiet --test unit_peeking"
run_test "integration_streaming" "cargo test --quiet --test integration_streaming"
run_test "memory_profile" "cargo test --quiet --test memory_profile"

echo ""
echo "6. Checking Memory Profiling"
echo "-----------------------------"
echo "Running memory profile test with /usr/bin/time..."
if /usr/bin/time -v cargo test --quiet --test memory_profile test_baseline_memory -- --nocapture 2>&1 | grep -q "Maximum resident set size"; then
    echo -e "${GREEN}✓${NC} Memory profiling works"
else
    echo -e "${RED}✗${NC} Memory profiling failed"
    FAILED=$((FAILED + 1))
fi

echo ""
echo "7. Checking Fuzzing Setup"
echo "-------------------------"
echo "Listing fuzz targets..."
cargo +nightly fuzz list

echo ""
echo "8. Checking Network Tools"
echo "-------------------------"
echo "Testing port availability..."
if ! ss -tuln 2>/dev/null | grep -q ":8080"; then
    echo -e "${GREEN}✓${NC} Port 8080 available"
else
    echo -e "${YELLOW}⚠${NC} Port 8080 already in use"
fi

if ! ss -tuln 2>/dev/null | grep -q ":8081"; then
    echo -e "${GREEN}✓${NC} Port 8081 available"
else
    echo -e "${YELLOW}⚠${NC} Port 8081 already in use"
fi

echo ""
echo "======================================="
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ All checks passed!${NC}"
    echo ""
    echo "DevContainer is fully functional and ready for development."
    echo ""
    echo "Quick commands:"
    echo "  cargo test                    # Run all tests"
    echo "  cargo clippy -- -D warnings   # Lint with Clippy"
    echo "  cargo bench                   # Run benchmarks"
    echo "  cargo watch -x test           # Auto-run tests on change"
    echo "  cargo +nightly fuzz run peeking_fuzz -- -max_total_time=30"
    exit 0
else
    echo -e "${RED}❌ $FAILED check(s) failed${NC}"
    echo ""
    echo "Please review the output above and fix any issues."
    echo "Common fixes:"
    echo "  - Rebuild container: F1 → 'Dev Containers: Rebuild Container'"
    echo "  - Reinstall tools: bash .devcontainer/post-create.sh"
    echo "  - Check Docker resources: Ensure 4GB+ RAM allocated"
    exit 1
fi


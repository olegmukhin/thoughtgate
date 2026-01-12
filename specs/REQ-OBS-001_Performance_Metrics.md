# REQ-OBS-001: Performance Metrics & Benchmarking

| Metadata | Value |
|----------|-------|
| **ID** | `REQ-OBS-001` |
| **Title** | Performance Metrics & Benchmarking |
| **Type** | Observability |
| **Status** | Draft |
| **Priority** | **Medium** |
| **Tags** | `#observability` `#benchmarking` `#ci` `#performance` `#marketing` |

## 1. Context & Decision Rationale

This requirement defines CI-based performance measurement for ThoughtGate. Performance metrics serve two critical purposes:

1. **Regression Detection:** Identify performance degradation before release
2. **Marketing Validation:** Provide reproducible numbers for "low footprint" claims

**Why CI-based measurement?**
- Reproducible: Same environment, same methodology
- Automated: No manual testing burden
- Historical: Track trends over time
- Gated: Block releases on regression

**Why Bencher.dev?**
- Statistical change detection (handles CI noise better than fixed thresholds)
- Branch-aware tracking (main vs releases vs PRs)
- Clean dashboards for marketing
- Embeddable badges for README
- Free for open source

## 2. Dependencies

| Requirement | Relationship | Notes |
|-------------|--------------|-------|
| REQ-CORE-005 | **Measures** | Startup time, health check latency |
| REQ-POL-001 | **Measures** | Policy evaluation performance |
| REQ-GOV-002 | **Measures** | Approval workflow overhead |

## 3. Intent

The system must:
1. Collect performance metrics during CI builds
2. Track metrics over time with statistical regression detection
3. Differentiate between development commits, PRs, and releases
4. Provide marketing-ready numbers with reproducible methodology
5. Alert on significant performance regressions

## 4. Scope

### 4.1 In Scope
- Binary characteristics (size, startup time)
- Memory footprint (idle, under load)
- Request latency (p50, p95, p99)
- Proxy overhead vs direct connection
- Throughput under load
- Policy evaluation performance
- CI integration with Bencher.dev
- Branch-aware tracking (main, releases, PRs)

### 4.2 Out of Scope
- Runtime observability (covered by future REQ-OBS-002)
- Distributed tracing
- Production monitoring
- Alerting/paging systems

## 5. Metric Definitions

### 5.1 Tier 1: Must Have (Critical for Marketing & Regression)

| ID | Metric | Unit | Target | Collection Method | Rationale |
|----|--------|------|--------|-------------------|-----------|
| M-BIN-001 | `binary/size` | bytes | < 15 MB | `stat -c%s target/release/thoughtgate` | "Lightweight" claim |
| M-MEM-001 | `memory/idle_rss` | bytes | < 20 MB | `/proc/<pid>/status` VmRSS at idle | "Low footprint" claim |
| M-LAT-001 | `latency/p50` | ms | < 5 ms | k6 `http_req_duration` | Typical request latency |
| M-LAT-002 | `latency/p95` | ms | < 15 ms | k6 `http_req_duration` | Tail latency |
| M-TTFB-001 | `ttfb/p95` | ms | < 10 ms | k6 `http_req_waiting` | Time to first byte (REQ-CORE-001) |
| M-TP-001 | `throughput/rps_10vu` | req/s | > 10,000 | k6 `http_reqs` rate | Capacity claim |
| M-OH-001 | `overhead/latency_p50` | ms | < 2 ms | `proxy_p50 - direct_p50` | "Minimal overhead" claim |

### 5.2 Tier 2: Should Have (Operational Insights)

| ID | Metric | Unit | Target | Collection Method | Rationale |
|----|--------|------|--------|-------------------|-----------|
| M-START-001 | `startup/to_healthy` | ms | < 100 ms | Time from exec to `/health` 200 | K8s cold start |
| M-MEM-002 | `memory/under_load_rss` | bytes | < 100 MB | RSS during k6 load test | Scaling behavior |
| M-LAT-003 | `latency/p99` | ms | < 50 ms | k6 `http_req_duration` | Worst-case tail |
| M-POL-001 | `policy/eval_p50` | µs | < 100 µs | Criterion micro-benchmark | Per-request policy cost |

### 5.3 Tier 3: Nice to Have (Deep Insights)

| ID | Metric | Unit | Target | Collection Method | Rationale |
|----|--------|------|--------|-------------------|-----------|
| M-BIN-002 | `binary/docker_image` | bytes | < 25 MB | `docker image inspect` | Container footprint |
| M-MEM-003 | `memory/peak_rss` | bytes | < 150 MB | Peak RSS during test | Worst-case memory |
| M-START-002 | `startup/policy_load` | ms | < 50 ms | Measure Cedar load time | Config reload speed |
| M-POL-002 | `policy/eval_p99` | µs | < 500 µs | Criterion micro-benchmark | Policy tail latency |
| M-POL-003 | `policy/reload_time` | ms | < 100 ms | Measure hot-reload | Zero-downtime updates |
| M-OH-002 | `overhead/percent_p50` | % | < 10% | `(proxy - direct) / direct * 100` | Relative overhead |
| M-APPR-001 | `approval/overhead` | ms | < 500 ms | Time to Slack post | Approval system latency |

## 6. Branch Scoping Strategy

### 6.1 Branch Types

| Branch | Purpose | Retention | Baseline Comparison |
|--------|---------|-----------|---------------------|
| `main` | Development tracking | Unlimited | Previous commit on main |
| `releases` | Release-to-release comparison | Unlimited | Previous release |
| `pr-*` | Pre-merge validation | 30 days | Head of main |

### 6.2 CI Trigger Mapping

| Git Event | Bencher Branch | Action |
|-----------|----------------|--------|
| Push to `main` | `main` | Track, alert on regression |
| Push tag `v*` | `releases` | Track, reset baseline |
| Pull request | `pr-{number}` | Compare to main, comment |

### 6.3 Bencher.dev Configuration

```yaml
# Development commits (main branch)
- name: Report benchmarks (main)
  if: github.ref == 'refs/heads/main'
  uses: bencherdev/bencher@main
  with:
    project: thoughtgate
    token: ${{ secrets.BENCHER_API_TOKEN }}
    branch: main
    testbed: ci-ubuntu-latest
    adapter: json
    file: bench_metrics.json
    err: true  # Fail on statistical regression

# Tagged releases
- name: Report benchmarks (release)
  if: startsWith(github.ref, 'refs/tags/v')
  uses: bencherdev/bencher@main
  with:
    project: thoughtgate
    token: ${{ secrets.BENCHER_API_TOKEN }}
    branch: releases
    testbed: ci-ubuntu-latest
    adapter: json
    file: bench_metrics.json
    branch-reset: true  # New baseline from this release

# Pull requests
- name: Report benchmarks (PR)
  if: github.event_name == 'pull_request'
  uses: bencherdev/bencher@main
  with:
    project: thoughtgate
    token: ${{ secrets.BENCHER_API_TOKEN }}
    branch: pr-${{ github.event.pull_request.number }}
    branch-start-point: main
    testbed: ci-ubuntu-latest
    adapter: json
    file: bench_metrics.json
    github-actions: ${{ secrets.GITHUB_TOKEN }}
    err: true  # Fail PR on regression
```

## 7. Collection Methods

### 7.1 Static Metrics (CI Build Job)

Collected after release build, before any tests:

```bash
# M-BIN-001: Binary size
BINARY_SIZE=$(stat -c%s target/release/thoughtgate)

# M-BIN-002: Docker image size (if built)
DOCKER_SIZE=$(docker image inspect thoughtgate:test --format='{{.Size}}')
```

### 7.2 Startup Metrics (Dedicated Job)

```bash
# M-START-001: Startup to healthy
START=$(date +%s%N)
./target/release/thoughtgate &
PID=$!

# Poll /health until 200
while ! curl -sf http://localhost:8080/health > /dev/null 2>&1; do
  sleep 0.01
done
END=$(date +%s%N)

STARTUP_MS=$(( (END - START) / 1000000 ))
```

### 7.3 Memory Metrics (During k6 Test)

```bash
# M-MEM-001: Idle RSS (before load)
IDLE_RSS=$(grep VmRSS /proc/$PID/status | awk '{print $2 * 1024}')

# Run k6 load test...

# M-MEM-002: Under load RSS (during test)
LOAD_RSS=$(grep VmRSS /proc/$PID/status | awk '{print $2 * 1024}')

# M-MEM-003: Peak RSS (after test)
PEAK_RSS=$(grep VmHWM /proc/$PID/status | awk '{print $2 * 1024}')
```

### 7.4 Latency & Throughput Metrics (k6)

Existing k8s integration test (`tests/integration_k8s.rs`) collects:
- `http_req_duration` → M-LAT-001, M-LAT-002, M-LAT-003
- `http_req_waiting` → M-TTFB-001
- `http_reqs` rate → M-TP-001

### 7.5 Overhead Metrics (k6 Comparison)

Run k6 against:
1. Direct connection to mock upstream
2. Connection through ThoughtGate proxy

Calculate: `overhead = proxy_latency - direct_latency`

### 7.6 Policy Evaluation Metrics (Criterion)

New Criterion benchmark (`benches/policy_eval.rs`):

```rust
fn bench_policy_eval(c: &mut Criterion) {
    let engine = PolicyEngine::new_from_string(SAMPLE_POLICY).unwrap();
    let context = create_test_context();

    c.bench_function("policy_eval", |b| {
        b.iter(|| engine.evaluate(&context))
    });
}
```

## 8. Output Format

### 8.1 Bencher JSON Schema

All metrics consolidated into single JSON file:

```json
[
  {"name": "binary/size", "value": 8432640, "unit": "bytes"},
  {"name": "binary/docker_image", "value": 12582912, "unit": "bytes"},
  {"name": "startup/to_healthy", "value": 45.2, "unit": "ms"},
  {"name": "startup/policy_load", "value": 12.1, "unit": "ms"},
  {"name": "memory/idle_rss", "value": 15728640, "unit": "bytes"},
  {"name": "memory/under_load_rss", "value": 52428800, "unit": "bytes"},
  {"name": "memory/peak_rss", "value": 67108864, "unit": "bytes"},
  {"name": "latency/p50", "value": 2.3, "unit": "ms"},
  {"name": "latency/p95", "value": 5.1, "unit": "ms"},
  {"name": "latency/p99", "value": 12.4, "unit": "ms"},
  {"name": "ttfb/p95", "value": 2.8, "unit": "ms"},
  {"name": "overhead/latency_p50", "value": 0.8, "unit": "ms"},
  {"name": "overhead/percent_p50", "value": 3.2, "unit": "%"},
  {"name": "throughput/rps_10vu", "value": 15420, "unit": "req/s"},
  {"name": "policy/eval_p50", "value": 45, "unit": "ns"},
  {"name": "policy/eval_p99", "value": 120, "unit": "ns"}
]
```

### 8.2 Metric Naming Convention

```
{category}/{metric_name}
```

Categories:
- `binary/` - Static binary characteristics
- `startup/` - Startup performance
- `memory/` - Memory footprint
- `latency/` - Request latency percentiles
- `ttfb/` - Time to first byte
- `throughput/` - Requests per second
- `overhead/` - Proxy overhead vs direct
- `policy/` - Cedar policy evaluation
- `approval/` - Approval workflow

## 9. Regression Detection

### 9.1 Statistical Method

Bencher.dev uses Student's t-test with configurable thresholds:

| Metric Category | Alert Threshold | Block Threshold |
|-----------------|-----------------|-----------------|
| Latency | +10% with p < 0.05 | +25% with p < 0.01 |
| Memory | +15% with p < 0.05 | +50% with p < 0.01 |
| Throughput | -10% with p < 0.05 | -25% with p < 0.01 |
| Binary size | +5% | +10% |

### 9.2 Noise Handling

CI environments have inherent variance. Bencher.dev addresses this by:
1. Using statistical significance (not just threshold)
2. Tracking historical variance per metric
3. Requiring multiple consecutive regressions for alert

### 9.3 Alert Actions

| Severity | Trigger | Action |
|----------|---------|--------|
| Warning | Statistical regression detected | PR comment, no block |
| Failure | Significant regression (> block threshold) | Fail CI, block merge |

## 10. Marketing Artifacts

### 10.1 README Badges

```markdown
[![Binary Size](https://bencher.dev/badge/thoughtgate/binary-size)](https://bencher.dev/perf/thoughtgate)
[![Latency p95](https://bencher.dev/badge/thoughtgate/latency-p95)](https://bencher.dev/perf/thoughtgate)
[![Throughput](https://bencher.dev/badge/thoughtgate/throughput)](https://bencher.dev/perf/thoughtgate)
```

### 10.2 Release Notes Template

```markdown
## Performance (v{version})

| Metric | Value | vs Previous |
|--------|-------|-------------|
| Binary size | {binary/size} MB | {delta}% |
| Memory (idle) | {memory/idle_rss} MB | {delta}% |
| Latency p95 | {latency/p95} ms | {delta}% |
| Throughput | {throughput/rps_10vu} req/s | {delta}% |
| Proxy overhead | {overhead/latency_p50} ms | {delta}% |
```

### 10.3 Marketing One-Liners

Based on target metrics:
- "< 15 MB binary, < 20 MB memory at idle"
- "Adds < 2ms p50 latency overhead"
- "> 10,000 requests/second throughput"
- "Sub-100µs policy evaluation"
- "Cold start in under 100ms"

## 11. Implementation Plan

### 11.1 CI Pipeline Changes

```
┌─────────────────────────────────────────────────────────┐
│                    CI PIPELINE                          │
│                                                         │
│  build:                                                 │
│    └─→ Collect M-BIN-001, M-BIN-002                    │
│                                                         │
│  k8s-tests:                                            │
│    └─→ Collect M-LAT-*, M-TTFB-*, M-TP-*, M-MEM-*     │
│    └─→ Collect M-OH-* (overhead)                       │
│                                                         │
│  bench (new):                                          │
│    └─→ Collect M-POL-* (Criterion)                     │
│    └─→ Collect M-START-* (startup timing)             │
│                                                         │
│  report-metrics (new):                                  │
│    └─→ Consolidate all metrics to JSON                 │
│    └─→ Upload to Bencher.dev                           │
│    └─→ Comment on PR if applicable                     │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### 11.2 New Files Required

| File | Purpose |
|------|---------|
| `benches/policy_eval.rs` | Criterion benchmark for policy evaluation |
| `scripts/collect_metrics.sh` | Consolidate metrics from various sources |
| `.github/workflows/ci.yml` (modify) | Add Bencher.dev integration |

### 11.3 Implementation Order

1. **Phase 1:** Tier 1 metrics + Bencher.dev integration
   - Wire existing k6 results to Bencher.dev
   - Add binary size collection
   - Add memory measurement to k8s tests

2. **Phase 2:** Tier 2 metrics
   - Add startup timing measurement
   - Add Criterion policy benchmark
   - Add p99 latency tracking

3. **Phase 3:** Tier 3 metrics + Polish
   - Add overhead comparison tests
   - Add approval workflow metrics
   - Add README badges
   - Add release notes automation

## 12. Verification Plan

### 12.1 Test Matrix

| Scenario | Expected Behavior | Test ID |
|----------|-------------------|---------|
| Normal commit to main | Metrics tracked, no alert | TC-OBS-001 |
| Regression on main | Alert, PR comment if applicable | TC-OBS-002 |
| Tagged release | Tracked on `releases` branch | TC-OBS-003 |
| PR opened | Compared to main baseline | TC-OBS-004 |
| PR with regression | CI fails, comment posted | TC-OBS-005 |
| Metrics collection failure | CI continues, warning logged | TC-OBS-006 |
| Bencher.dev unavailable | CI continues, artifact uploaded | TC-OBS-007 |

### 12.2 Acceptance Criteria

- [ ] All Tier 1 metrics collected in CI
- [ ] Metrics reported to Bencher.dev on main pushes
- [ ] Metrics reported to `releases` branch on tags
- [ ] PR comments show comparison to baseline
- [ ] Statistical regression detection working
- [ ] README badges displaying current values
- [ ] Release notes include performance section

## 13. Definition of Done

- [ ] Tier 1 metrics implemented and collecting
- [ ] Bencher.dev project configured
- [ ] Branch scoping (main/releases/pr-*) working
- [ ] Regression detection with statistical significance
- [ ] PR comments on benchmark changes
- [ ] CI fails on significant regression
- [ ] README badges added
- [ ] Release notes template includes metrics
- [ ] All test cases (TC-OBS-001 to TC-OBS-007) passing
- [ ] Documentation updated in CLAUDE.md

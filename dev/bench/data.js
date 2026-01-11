window.BENCHMARK_DATA = {
  "lastUpdate": 1768095900641,
  "repoUrl": "https://github.com/olegmukhin/thoughtgate",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "email": "oleg.v.mukhin@gmail.com",
            "name": "Oleg Mukhin",
            "username": "olegmukhin"
          },
          "committer": {
            "email": "oleg.v.mukhin@gmail.com",
            "name": "Oleg Mukhin",
            "username": "olegmukhin"
          },
          "distinct": true,
          "id": "6e84ea0b390d6248f735e1ac4441aaf77c024fec",
          "message": "fix: header redaction security bugs and benchmark CI initialization\n\nSecurity Fixes (Found by Fuzzer):\n- Fix case-sensitive header comparison allowing credential leaks\n  Headers like \"Cookie\", \"COOKIE\", \"Authorization\" were not being\n  redacted due to case-sensitive string matching. HTTP header names\n  are case-insensitive per RFC 7230 Section 3.2.\n\n- Fix memory allocation crash in header sanitization\n  The .to_lowercase() approach allocated a String for every header,\n  causing crashes with malformed input and potential DoS via memory\n  exhaustion.\n\n- Solution: Use zero-allocation .eq_ignore_ascii_case() for header\n  comparison. This is faster, safer, and prevents both bugs.\n\nCI Fix:\n- Add automatic gh-pages branch creation for benchmarks\n  The benchmark-action failed on first run because gh-pages branch\n  didn't exist. Added setup step to create orphan branch with README\n  if needed, allowing benchmark tracking to work from first run.\n\nDiscovered-by: cargo-fuzz (fuzz_header_redaction target)",
          "timestamp": "2026-01-04T10:19:20Z",
          "tree_id": "3289b7e920aee5073d2d85a594c624f210fd3747",
          "url": "https://github.com/olegmukhin/thoughtgate/commit/6e84ea0b390d6248f735e1ac4441aaf77c024fec"
        },
        "date": 1767522118637,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "ttfb/direct/baseline",
            "value": 131606.40457170393,
            "unit": "ns"
          },
          {
            "name": "ttfb/proxied/with_relay",
            "value": 11398889.061111115,
            "unit": "ns"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "oleg.v.mukhin@gmail.com",
            "name": "Oleg Mukhin",
            "username": "olegmukhin"
          },
          "committer": {
            "email": "oleg.v.mukhin@gmail.com",
            "name": "Oleg Mukhin",
            "username": "olegmukhin"
          },
          "distinct": true,
          "id": "ce3c78ca992b2e7e5fd3be8c77958bea1fd23200",
          "message": "Formatting fixes",
          "timestamp": "2026-01-04T10:34:42Z",
          "tree_id": "83cbecd8d145870c3f49bff2d51c3fb5f2fcad8c",
          "url": "https://github.com/olegmukhin/thoughtgate/commit/ce3c78ca992b2e7e5fd3be8c77958bea1fd23200"
        },
        "date": 1767523021118,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "ttfb/direct/baseline",
            "value": 95455.66009773948,
            "unit": "ns"
          },
          {
            "name": "ttfb/proxied/with_relay",
            "value": 11379650.08333333,
            "unit": "ns"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "oleg.v.mukhin@gmail.com",
            "name": "Oleg Mukhin",
            "username": "olegmukhin"
          },
          "committer": {
            "email": "oleg.v.mukhin@gmail.com",
            "name": "Oleg Mukhin",
            "username": "olegmukhin"
          },
          "distinct": true,
          "id": "33bdeaf5a2ef7a6cbc70c01fa561534db59c5056",
          "message": "refactor: streamline header sanitization logic in fuzz test\n\n- Consolidated header name and value creation to reduce redundancy.\n- Enhanced sensitive header detection to ensure proper redaction patterns are checked.\n- Improved assertions for verifying that sensitive headers are not leaked and are correctly redacted.\n\nThis refactor aims to improve code clarity and maintainability while ensuring robust security checks for sensitive headers.",
          "timestamp": "2026-01-04T11:39:37Z",
          "tree_id": "42297936c89bf09d3554d3f0cf1b13bf4793e710",
          "url": "https://github.com/olegmukhin/thoughtgate/commit/33bdeaf5a2ef7a6cbc70c01fa561534db59c5056"
        },
        "date": 1767526911878,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "ttfb/direct/baseline",
            "value": 128714.0174946416,
            "unit": "ns"
          },
          {
            "name": "ttfb/proxied/with_relay",
            "value": 11366306.38,
            "unit": "ns"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "oleg.v.mukhin@gmail.com",
            "name": "Oleg Mukhin",
            "username": "olegmukhin"
          },
          "committer": {
            "email": "oleg.v.mukhin@gmail.com",
            "name": "Oleg Mukhin",
            "username": "olegmukhin"
          },
          "distinct": true,
          "id": "ae76348d58c6a43f2a13766990e6594e2b777eb7",
          "message": "chore: update dependencies and remove unused packages\n\n- Removed obsolete dependencies from Cargo.lock and Cargo.toml, including aws-lc-rs, aws-lc-sys, cmake, dunce, fs_extra, and jobserver.\n- Updated hyper-rustls and rustls configurations to disable default features and include specific features for improved performance and security.\n\nThese changes help streamline the dependency tree and enhance the overall project configuration.",
          "timestamp": "2026-01-04T11:54:38Z",
          "tree_id": "4634c40fb0ca14e25073af9201a98680a546e9a3",
          "url": "https://github.com/olegmukhin/thoughtgate/commit/ae76348d58c6a43f2a13766990e6594e2b777eb7"
        },
        "date": 1767527850883,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "ttfb/direct/baseline",
            "value": 139242.0134312134,
            "unit": "ns"
          },
          {
            "name": "ttfb/proxied/with_relay",
            "value": 11345661.565555556,
            "unit": "ns"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "oleg.v.mukhin@gmail.com",
            "name": "Oleg Mukhin",
            "username": "olegmukhin"
          },
          "committer": {
            "email": "oleg.v.mukhin@gmail.com",
            "name": "Oleg Mukhin",
            "username": "olegmukhin"
          },
          "distinct": true,
          "id": "533021825a8ee950e6773d4f8665aef6a18c524d",
          "message": "fix(fuzz): handle header truncation in redaction test\n\nThe fuzz test was incorrectly failing when sensitive headers were\ntruncated due to MAX_HEADERS_TO_LOG=50 defense-in-depth limit.\n\nBefore: Test asserted ALL sensitive headers must appear as redacted\nAfter: Skip validation for headers that don't appear (truncated)\n\nOnly validate redaction for headers actually present in output.",
          "timestamp": "2026-01-06T10:43:39Z",
          "tree_id": "03e2d26f4ce5c4c68d8506d7a4213b45bd279b60",
          "url": "https://github.com/olegmukhin/thoughtgate/commit/533021825a8ee950e6773d4f8665aef6a18c524d"
        },
        "date": 1767696335959,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "ttfb/direct/baseline",
            "value": 130352.5170348292,
            "unit": "ns"
          },
          {
            "name": "ttfb/proxied/with_relay",
            "value": 11351371.344444443,
            "unit": "ns"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "oleg.v.mukhin@gmail.com",
            "name": "Oleg Mukhin",
            "username": "olegmukhin"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "957055731c178f0f2e68f22d544be13edf7e11cf",
          "message": "Merge pull request #13 from olegmukhin/traffic-termination\n\nfeat: implement zero-copy peeking and buffered termination strategies",
          "timestamp": "2026-01-09T22:13:22Z",
          "tree_id": "35ce71f4f8856a7d07f44fb0d6b804d101786595",
          "url": "https://github.com/olegmukhin/thoughtgate/commit/957055731c178f0f2e68f22d544be13edf7e11cf"
        },
        "date": 1767997036336,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "ttfb/direct/baseline",
            "value": 131415.99600925605,
            "unit": "ns"
          },
          {
            "name": "ttfb/proxied/with_relay",
            "value": 11448396.460000003,
            "unit": "ns"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "oleg.v.mukhin@gmail.com",
            "name": "Oleg Mukhin",
            "username": "olegmukhin"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8e6ee03e4f2d05b1d3235b29d941e50dffaaba1c",
          "message": "Merge pull request #14 from olegmukhin/refactor\n\nRemoved unused files and added CLAUDE.md",
          "timestamp": "2026-01-10T22:35:41Z",
          "tree_id": "0ea6adbfdfe17154359d6d04782f83426dd37890",
          "url": "https://github.com/olegmukhin/thoughtgate/commit/8e6ee03e4f2d05b1d3235b29d941e50dffaaba1c"
        },
        "date": 1768084699302,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "ttfb/direct/baseline",
            "value": 138279.4339354855,
            "unit": "ns"
          },
          {
            "name": "ttfb/proxied/with_relay",
            "value": 11375513.813333333,
            "unit": "ns"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "oleg.v.mukhin@gmail.com",
            "name": "Oleg Mukhin",
            "username": "olegmukhin"
          },
          "committer": {
            "email": "oleg.v.mukhin@gmail.com",
            "name": "Oleg Mukhin",
            "username": "olegmukhin"
          },
          "distinct": true,
          "id": "03b772f4a63227300d10232f3458c6ac4a4df43f",
          "message": "docs: merge pre-commit checklist",
          "timestamp": "2026-01-10T23:31:53Z",
          "tree_id": "6545ed3c245930dbb4b5724b3418a76e2a07d72c",
          "url": "https://github.com/olegmukhin/thoughtgate/commit/03b772f4a63227300d10232f3458c6ac4a4df43f"
        },
        "date": 1768088399866,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "ttfb/direct/baseline",
            "value": 141104.0656612643,
            "unit": "ns"
          },
          {
            "name": "ttfb/proxied/with_relay",
            "value": 11354554.626666663,
            "unit": "ns"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "oleg.v.mukhin@gmail.com",
            "name": "Oleg Mukhin",
            "username": "olegmukhin"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "0eef9941219be80be9975cb21e2fc975ce520e3d",
          "message": "feat(policy): implement Cedar policy engine (#15)\n\n* feat(policy): implement Cedar policy engine\n\nImplement comprehensive Cedar policy engine with 4-way traffic\nclassification (Green/Amber/Approval/Red paths) based on declarative\npolicies.\n\nKey features:\n- Policy evaluation with action priority (StreamRaw → Inspect → Approve)\n- Post-approval re-evaluation with policy drift detection\n- Policy loading priority: ConfigMap → Environment → Embedded defaults\n- Hot-reload with atomic swap (arc-swap) for zero-downtime updates\n- K8s identity inference from ServiceAccount mounts\n- Schema validation for all policies\n- Development mode override for local testing\n\nTechnical implementation:\n- Uses cedar-policy crate for sub-millisecond policy evaluation\n- Lock-free hot-reload using arc_swap::ArcSwap\n- Best-effort JWT parsing for ServiceAccount name extraction\n- Comprehensive error types (ParseError, SchemaValidation, IdentityError)\n- 31 unit tests covering all 16 edge cases (EC-POL-001 to EC-POL-016)\n\nFiles:\n- src/policy/mod.rs: Core types and public API\n- src/policy/engine.rs: Cedar engine with evaluate() and reload()\n- src/policy/loader.rs: Policy loading with fallback chain\n- src/policy/principal.rs: K8s identity inference\n- src/policy/schema.cedarschema: Cedar schema definition\n- src/policy/defaults.cedar: Embedded permissive policies for dev\n\nImplements: REQ-POL-001\nRefs: specs/REQ-POL-001_Cedar_Policy_Engine.md\n\n* chore(policy): update deps and add serial test annotations\n\nUpdate policy engine dependencies to latest stable versions:\n- cedar-policy: 4.2 → 4.8.0 (actual: 4.8.2)\n- arc-swap: 1.7 → 1.8.0\n\nAdd serial_test annotations to prevent race conditions in tests\nthat mutate global environment variables. Tests with #[serial]\nexecute sequentially, while pure-read tests remain parallel.\n\nFiles updated:\n- src/policy/engine.rs: 9 tests annotated with #[serial]\n- src/policy/loader.rs: 5 tests annotated with #[serial]\n- src/policy/principal.rs: 5 tests annotated with #[serial]\n\nVerified:\n- All 31 policy tests passing without --test-threads=1\n- cargo clippy clean (no warnings)\n- cargo build successful\n- No API changes required in existing code\n\n* fix(policy): add entity attributes, strict dev mode, and last_reload tracking\n\nFix critical issues in Cedar policy engine:\n\n1. Add principal entity attributes to Cedar evaluation\n   - Build entities with namespace, service_account, and roles\n   - Enables policies to match on principal.namespace and role hierarchy\n   - Previously used Entities::empty() causing silent policy failures\n\n2. Fix dev mode check to require explicit \"true\" value\n   - Changed from .is_ok() to .as_deref() == Ok(\"true\")\n   - Prevents accidental dev mode activation from THOUGHTGATE_DEV_MODE=false\n   - Security fix: operators can now safely disable dev mode\n\n3. Track last_reload timestamp in PolicyStats\n   - Add arc_swap::ArcSwap<Option<SystemTime>> to Stats struct\n   - Update timestamp on successful reload\n   - Improves observability for policy hot-reload operations\n\n4. Add serial test annotations for env-dependent tests\n   - Mark test_engine_creation, test_evaluate_with_default_policies,\n     test_stats with #[serial] to prevent race conditions\n\nTesting:\n- Added test_dev_mode_requires_true to verify strict checking\n- All 32 policy tests passing\n- cargo clippy clean\n\nFixes issues that would cause:\n- Policies using principal attributes to silently fail\n- Unintended dev mode activation (security vulnerability)\n- Missing observability data for policy reloads\n\n* Revert Cedar entity building to fix CI test failures\n\nReverts the build_entities() implementation that was causing 6 policy\nengine tests to fail in CI. The entity-building code had several issues:\n\n1. Role entities were created without required \"name\" attribute\n2. Resource entities were missing from entity store\n3. Cedar schema validation became stricter with entities present\n\nRoot cause: When entities are provided to Cedar's authorizer, it\nvalidates them against the schema. The incomplete entity construction\ncaused validation failures that manifested as policy denials.\n\nFor v0.1, we use Entities::empty() which works correctly with:\n- Entity UID-based policies (principal == ThoughtGate::App::\"name\")\n- All default embedded policies\n- All test policies\n\nThis does NOT support:\n- Attribute-based policies (principal.namespace == \"prod\")\n- Role hierarchy checks (principal in ThoughtGate::Role::\"admin\")\n\nFull entity store support will be added in a future version when\nneeded for production RBAC policies.\n\nFixes:\n- test_ec_pol_001_streamraw_permitted\n- test_ec_pol_002_inspect_only\n- test_ec_pol_006_post_approval_denied\n- test_ec_pol_010_invalid_syntax\n- test_ec_pol_011_schema_violation\n- test_ec_pol_012_reload_updates_stats (indirectly)\n\n* style(policy): remove extra blank line\n\n* fix(test): use BTreeMap for deterministic snapshot ordering\n\nChanged HashMap to BTreeMap in snapshot tests to ensure consistent\nkey ordering across test runs. This fixes CI failures where HashMap\niteration order is non-deterministic.\n\nFixes test_integrity_snapshot failures in CI.\n\n* docs(policy): add requirement traceability to public methods\n\nAdded doc comments with requirement links to policy_source() and\nstats() methods per coding guidelines. All public functions must\nlink to their implementing requirement.\n\n- policy_source(): Links to REQ-POL-001/F-003 (Policy Loading)\n- stats(): Links to REQ-POL-001/F-005 (Hot-Reload)",
          "timestamp": "2026-01-11T01:39:39Z",
          "tree_id": "2c29a0515d15a0f9eb475dd144a7ea18df068d76",
          "url": "https://github.com/olegmukhin/thoughtgate/commit/0eef9941219be80be9975cb21e2fc975ce520e3d"
        },
        "date": 1768095900242,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "ttfb/direct/baseline",
            "value": 137666.42281840704,
            "unit": "ns"
          },
          {
            "name": "ttfb/proxied/with_relay",
            "value": 11360106.245555554,
            "unit": "ns"
          }
        ]
      }
    ]
  }
}
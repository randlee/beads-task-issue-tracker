# B.P3 BTIT source handoff

This is the complete BTIT source-implementation record. It is not independent
source QA or acceptance, a destination copy, a merge decision, or a publication
decision. Those remain separately owned gates.

## Immutable inputs and source revision

| Item | Value |
| --- | --- |
| BTIT implementation SHA | `c31095326bde604557dd1aa51c7252db7b3d6284` |
| BTIT branch / target | `feature/sc-obs-B-P3-runtime-bridge` → `develop` |
| Draft review | [BTIT PR #86](https://github.com/randlee/beads-task-issue-tracker/pull/86) |
| Reviewed target and runtime contracts | `84b32e9d6718418371ffd25a3de52346278725ca` |
| B.P2 candidate | `1.3.0`, staged only; never published |
| B.P2 package source | `561f89923c7f4fdfa9cd0fafa929a5d6dc94dfe5` |
| B.P2 stage manifest SHA-256 | `822ff4494dcfbf92b3dcf47fa3fceac8df00b7e28baf7777b6a5aa1147078077` |
| B.P2 qualification | run `35179919596`, validation `31d47d3d62572dbf46fc31189638cfa38c3b19b8` |

`scripts/prepare_bp2_stage.py` verifies candidate identity, manifest, archive
digests, and declared file lists before extracting consumed packages under
ignored `.bp2-stage/`. The source never selects a sibling checkout or registry release.

| Archive | SHA-256 |
| --- | --- |
| `sc-observability-types-1.3.0.crate` | `352a993e75e7f0bf4fc2c5349d72eeb5792b1544e220d694e189e784a960aa8f` |
| `sc-observability-1.3.0.crate` | `e54941f5f8a365a71955561d239e440e2dc704d585f1efc9f2666305fd16e58c` |
| `sc-observe-1.3.0.crate` | `6834769a32ee2c741a6ee050cc10a9b1a4a20f5cc0284a158b522ca22d6d581b` |
| `sc-observability-otlp-1.3.0.crate` | `56bfd2a95dca677d4b173559070c17a1cdab6875a57904fe10e329cba4198a59` |

## Export and boundary disposition

The final root inventory was inspected from `lib.rs`, `control.rs`, `error.rs`,
`health.rs`, `error_codes.rs`, and macro roots at the implementation SHA.

| Family | Final disposition |
| --- | --- |
| Existing bridge root | `init`, `BridgeOptions`, tracing-style `Level`, `DroppedEvents`, `DropCause`, `LogGuard`, default timeout, and macros are preserved with target lifecycle/accounting behavior. `LogGuard` alone owns elevation/reset and shutdown. |
| Core/neutral values | `LoggerConfig`, core name/value types, `LogEvent`, query/snapshot, correlation/trace/outcome, `OperationDiagnostic`, `AdmissionOutcome` as `EmitOutcome`, and runtime level types/errors are re-exported. |
| Direct/control surface | `BridgeEvent`, cloneable non-owning `LogControl`, `try_log`, query, bounded flush, health, path, drop snapshot, and `wait_stopped` are native. Control has no elevation, reset, shutdown, or owner conversion. |
| Errors/completion | Typed `InitError`, `FlushError`, `ShutdownError`, `EmitError`, `ControlError`, `WaitError`, `FieldKeyError`, `LifecyclePhase`, `ShutdownOutcome`, `UnconfirmedShutdown`, and `ShutdownReport` replace opaque adapters. Operation errors are tagged serde data with stable code/remediation and no source chain. |
| Health/registry | `BridgeHealthReport` v1 retains core logging health, exact drop/lifecycle/path and coherent level state. The seven baseline codes plus nine target codes are in unique `ALL`. |
| Removed adapter surface | Deleted `report.rs`, `StructuredRecord`, `SubmitOutcome`, `SubmitError`, `FailureReport`, `CONTROL_SCHEMA_VERSION`, and `THRESHOLD`, `THRESHOLD_OFF`, `encode_threshold`, `level_enabled`, `to_log_level_filter`; final whole-crate scan is empty. |
| Hidden macro support | `__private` remains exact-version macro support: callsite/context/mapping, fields/labels, event assembly, guard/accounting dispatch, and parser expansion. External macro grammar and compile-fail ownership fixtures are the boundary. |

The staged-core `LevelOwner` is the single direct/facade/macro filter authority.
The fixed facade ceiling is Trace; unsupported levels fail before mutation/global
installation. Lifecycle, bounded flush, one shutdown coordinator, retained
completion, panic/reentrancy containment, and exact-once rejection accounting
are covered by the source fixtures.

## Final validation and retained evidence

Local commands at `c31095326bde604557dd1aa51c7252db7b3d6284`:

```text
cargo test --workspace                                      PASS
cargo fmt --check -p sc-observability-log -p sc-observability-log-macros -p sc-observability-log-consumer-check  PASS
cargo clippy --locked --no-deps -p sc-observability-log -p sc-observability-log-macros -p sc-observability-log-consumer-check --all-targets --all-features -- -D warnings  PASS
cargo test --release --locked -p sc-observability-log --test runtime_level_bridge  PASS
cargo test --release --locked -p sc-observability-log --features static_level_cap_test --test static_level_cap  PASS
cargo test --locked -p sc-observability-log --features test_hooks --test init_runtime_start  PASS
cargo tree --locked -p sc-observability-log -e normal --target all … | diff - crates/runtime-deps.txt  PASS
test-isolation contract                                      PASS
cargo rustdoc --locked -p sc-observability-log -- -D missing-docs  PASS
cargo rustdoc --locked -p sc-observability-log-macros -- -D missing-docs  PASS
```

The workspace run includes bridge/macro/API/UI/consumer checks, direct/facade/
macro accounting, health retention, bounded flush/shutdown races, repeated
waiters, reinstallation, static-cap, startup failure, native error serde/clone,
and compile-fail control-ownership fixtures. Lead-focused artifacts are:

- `~/.config/atm/share/sc-obs/bp3-evidence/d7a26ded/lead-focused-fixtures.log`
- `~/.config/atm/share/sc-obs/bp3-evidence/c3109532/lead-native-data.log`

Final platform qualification is [CI run 35185604331](https://github.com/randlee/beads-task-issue-tracker/actions/runs/35185604331), explicitly at the implementation SHA and after downloading/verifying the B.P2 stage:

| Required job | Result |
| --- | --- |
| `crates (ubuntu-latest)` | PASS, including static-cap, dependency, isolation, rustdoc, and MSRV gates |
| `crates (macos-latest)` | PASS |
| `crates (windows-latest)` | PASS |
| `rust quality` on Ubuntu/macOS/Windows | PASS |

The unrelated `backend (windows-latest)` repository job was still running when
this handoff was written; it is not substituted for, or claimed as, B.P3 bridge
qualification. Retain its final state with the CI record.

## Handoff boundary

Implementation completeness is recorded. Independent critical review/re-review
and source QA, sc-observability acceptance, B.1 destination copy, merge, and
B.7 publication remain pending and are not implied by this record.

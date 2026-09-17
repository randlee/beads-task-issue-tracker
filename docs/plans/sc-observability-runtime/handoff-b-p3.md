# B.P3 BTIT source and evidence handoff

This is the complete BTIT source-implementation record. It is not independent
source QA or acceptance, a destination copy, a merge decision, or a publication
decision. Those remain separately owned gates.

## Immutable inputs and source revision

| Item | Value |
| --- | --- |
| BTIT implementation SHA | `c31095326bde604557dd1aa51c7252db7b3d6284` |
| BTIT branch / target | `feature/sc-obs-B-P3-runtime-bridge` → `develop` |
| Draft review | [BTIT PR #86](https://github.com/randlee/beads-task-issue-tracker/pull/86) |
| Accepted target design / reviewed runtime contract | `84b32e9d6718418371ffd25a3de52346278725ca`; runtime public-source acceptance remains owner-deferred |
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

The complete, reproducible root/hidden-support inventory is
[`bp3-implementation-inventory.md`](bp3-implementation-inventory.md). It is
generated from `git show c31095326bde604557dd1aa51c7252db7b3d6284:path`, not
the current checkout, by:

```text
python3 scripts/generate_bp3_handoff_inventory.py \
  --revision c31095326bde604557dd1aa51c7252db7b3d6284 \
  --output docs/plans/sc-observability-runtime/bp3-implementation-inventory.md
```

That artifact maps every root export/re-export, public native field/method,
enum payload, derive/impl, error-code member, hidden macro-support item, and
the `test_hooks` export to its target disposition. This table is a concise
boundary summary only; it does not narrow that generated inventory.

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

The retained [CI run 35185604331](https://github.com/randlee/beads-task-issue-tracker/actions/runs/35185604331)
is completed/successful and records `headSha`
`c31095326bde604557dd1aa51c7252db7b3d6284`. It is the authoritative
three-platform execution evidence. The immutable local evidence bundle is:

| Evidence | SHA-256 | Source revision it proves |
| --- | --- | --- |
| `~/.config/atm/share/sc-obs/bp3-evidence/ci-35185604331/run.json` | `e73b13cfee91e588a14b937b4b8372422cef959836f2d9bcfc57c444d6c1a21b` | The CI metadata, including the implementation `headSha` and all 13 successful jobs, at `c31095326bde604557dd1aa51c7252db7b3d6284`. |
| `~/.config/atm/share/sc-obs/bp3-evidence/ci-35185604331/logs.zip` | `ca1520851fde90b3fa5db4c3b341617d60b2c7b84275eb6ce716dd3f132018f0` | Raw GitHub job/step logs for that same implementation SHA. |
| `~/.config/atm/share/sc-obs/bp3-evidence/d7a26ded/lead-focused-fixtures.log` | `03533cd0d614e188f82d65a83b261d739e5ef73f60d2c7d7ab1c49549bb933d4` | The selected one-writer and late-shutdown fixture pass at `d7a26ded`; supporting pre-final evidence only, not a substitute for the final CI SHA. |
| `~/.config/atm/share/sc-obs/bp3-evidence/c3109532/lead-native-data.log` | `bb56db31edb748d8eac489e8af6c82486ce86689a2882c8299f195bfb2d03c15` | Native error/health serde and clone data fixtures at `c31095326bde604557dd1aa51c7252db7b3d6284`. |

Reproduce the retained-bundle integrity checks with:

```text
shasum -a 256 ~/.config/atm/share/sc-obs/bp3-evidence/ci-35185604331/run.json
shasum -a 256 ~/.config/atm/share/sc-obs/bp3-evidence/ci-35185604331/logs.zip
shasum -a 256 ~/.config/atm/share/sc-obs/bp3-evidence/d7a26ded/lead-focused-fixtures.log
shasum -a 256 ~/.config/atm/share/sc-obs/bp3-evidence/c3109532/lead-native-data.log
jq -r '.headSha, .conclusion, (.jobs[] | "\(.name)\\t\(.conclusion)")' \
  ~/.config/atm/share/sc-obs/bp3-evidence/ci-35185604331/run.json
unzip -t ~/.config/atm/share/sc-obs/bp3-evidence/ci-35185604331/logs.zip
```

The exact CI commands are preserved below rather than abbreviated. All ran
after `python3 scripts/prepare_bp2_stage.py --stage .bp2-download`, which
verifies/reconstructs the B.P2 input set before Cargo starts. Their raw output
is in `logs.zip`; paths name the enclosing CI job and step.

```text
cargo fmt --check -p sc-observability-log -p sc-observability-log-macros -p sc-observability-log-consumer-check
cargo clippy --locked --no-deps -p sc-observability-log -p sc-observability-log-macros -p sc-observability-log-consumer-check --all-targets --all-features -- -D warnings
cargo test --locked -p sc-observability-log -p sc-observability-log-macros -p sc-observability-log-consumer-check
cargo test --release --locked -p sc-observability-log --test runtime_level_bridge
cargo test --release --locked -p sc-observability-log --features static_level_cap_test --test static_level_cap
cargo test --locked -p sc-observability-log --features test_hooks --test init_runtime_start
CARGO_TERM_COLOR=never cargo tree --locked -p sc-observability-log -e normal --target all --prefix none --format '{p}' | sed -E 's/ \(.*$//' | LC_ALL=C sort -u | diff - crates/runtime-deps.txt
for f in crates/sc-observability-log*/tests/*.rs; do if grep -qE '\binit\(' "$f"; then n=$(grep -cE '#\[([A-Za-z_]+::)*test\b' "$f"); [ "$n" -eq 1 ] || { echo "isolation violation: $f has $n test fns"; exit 1; }; fi; done
cargo rustdoc --locked -p sc-observability-log -- -D missing-docs
cargo rustdoc --locked -p sc-observability-log-macros -- -D missing-docs
rustup toolchain install 1.94.1 --profile minimal --no-self-update
cargo +1.94.1 check --locked -p sc-observability-log -p sc-observability-log-macros -p sc-observability-log-consumer-check --all-targets
```

The matching raw member paths include `crates (ubuntu-latest)/7_Format.txt`,
`8_Clippy.txt`, `9_Test.txt`, `10_Release runtime-level bridge fixture.txt`,
`11_Capped startup fixture.txt`, `12_Lifecycle startup-failure fixture.txt`,
`13_Runtime dependency graph.txt`, `14_Test-isolation contract.txt`,
`15_Rustdoc missing-docs.txt`, and `16_MSRV check (1.94.1).txt`; the macOS and
Windows `crates` logs retain their platform equivalents. Rust-quality logs
retain the three-platform format/clippy/rustdoc results.

The retained CI metadata names all thirteen successful jobs:

| Required job | Result |
| --- | --- |
| `crates (ubuntu-latest)` | PASS, including static-cap, dependency, isolation, rustdoc, and MSRV gates |
| `crates (macos-latest)` | PASS |
| `crates (windows-latest)` | PASS |
| `rust quality` on Ubuntu/macOS/Windows | PASS |
| `version sync`; frontend and backend on Ubuntu/macOS/Windows | PASS |

## Handoff boundary

Implementation completeness is recorded. Independent critical review/re-review
and source QA, sc-observability acceptance, B.1 destination copy, merge, and
B.7 publication remain pending and are not implied by this record.

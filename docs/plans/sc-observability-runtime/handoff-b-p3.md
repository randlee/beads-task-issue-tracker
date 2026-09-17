# B.P3 BTIT source implementation handoff

This record identifies the immutable implementation commit for independent
critical review. It is source completion evidence only: it does not approve
the destination copy, runtime public API, registry publication, merge, or QA.

## Revisions and staged package provenance

| Item | Value |
| --- | --- |
| BTIT implementation source | `8c8e2565ab44a222bb1c64b8f855f597cf2086db` |
| Reviewed target bridge contract | `84b32e9d6718418371ffd25a3de52346278725ca` |
| B.P2 candidate version | `1.3.0` (staged; never published) |
| B.P2 package source | `561f89923c7f4fdfa9cd0fafa929a5d6dc94dfe5` |
| B.P2 stage manifest SHA-256 | `822ff4494dcfbf92b3dcf47fa3fceac8df00b7e28baf7777b6a5aa1147078077` |
| B.P2 retained platform qualification | run `35179919596`, validation head `31d47d3d62572dbf46fc31189638cfa38c3b19b8` |

The workspace never selects a sibling source checkout or a presumed registry
release. `scripts/prepare_bp2_stage.py` validates the exact final manifest,
candidate identity, every consumed archive digest and its declared package file
list before extracting the two Cargo dependencies to ignored `.bp2-stage/`.
The CI workflow downloads the same `bp2-candidate-stage` artifact before every
Rust job and runs that verifier.

| Archive | SHA-256 |
| --- | --- |
| `sc-observability-types-1.3.0.crate` | `352a993e75e7f0bf4fc2c5349d72eeb5792b1544e220d694e189e784a960aa8f` |
| `sc-observability-1.3.0.crate` | `e54941f5f8a365a71955561d239e440e2dc704d585f1efc9f2666305fd16e58c` |
| `sc-observe-1.3.0.crate` | `6834769a32ee2c741a6ee050cc10a9b1a4a20f5cc0284a158b522ca22d6d581b` |
| `sc-observability-otlp-1.3.0.crate` | `56bfd2a95dca677d4b173559070c17a1cdab6875a57904fe10e329cba4198a59` |

## Contract disposition and removals

- Removed the independent bridge `THRESHOLD`, `THRESHOLD_OFF`,
  `encode_threshold`, `level_enabled`, and `to_log_level_filter` policy path.
  `LevelOwner` from the staged core is the one mutable filter state; the fixed
  `log` facade ceiling remains Trace so compiled Debug/Trace callsites survive.
- Added owner-only `LogGuard::elevate_level` and `reset_level`; each rejects an
  unavailable static cap before mutating core state. `LogControl` remains
  cloneable, non-owning, and has no owner/shutdown/mutation conversion.
- Added typed `BridgeEvent`, `EmitOutcome` (the core `AdmissionOutcome` alias),
  direct `try_log`, query, non-owning drop snapshot and `wait_stopped` surface.
  Direct, facade, and macro paths use staged `try_log_with_outcome` and the
  same reentrancy/containment/drop accounting boundary.
- Added the reviewed direct/control errors and `LifecyclePhase`, tagged Serde
  forms, stable code/remediation methods, typed field-key rejection and
  unconfirmed-shutdown result. Retained control observation does not assert a
  stopped writer when completion is unconfirmed.
- Extended bridge health with the staged core’s configured/effective level and
  revision snapshot; existing compatibility projections remain available while
  B.1 owns final destination export acceptance.

## Source validation and raw evidence

Performed locally after reconstructing exact archives from:

`/Users/randlee/.config/atm/share/sc-obs/bp2-evidence/ci-35179919596/bp2-candidate-stage/`

```text
python3 scripts/prepare_bp2_stage.py --stage …/bp2-candidate-stage
verified B.P2 stage …/stage-manifest.json (822ff449…7078077)
cargo test --workspace
all workspace tests passed
cargo clippy --no-deps -p beads-issue-tracker -p btit-types -p btit-beads -p btit-cli -p btit-bd -p btit-br -p sc-observability-log -p sc-observability-log-macros -p sc-observability-log-consumer-check --all-targets --all-features -- -D warnings
Finished …
cargo fmt --check --all
python3 scripts/check_version_sync.py
version sync OK: app 1.24.5, rust toolchain 1.98.1
```

`crates/sc-observability-log/tests/runtime_level_bridge.rs` is the direct,
facade, macro, owner, level-revision and shutdown-observation fixture. Existing
macro grammar, external consumer and compile-fail ownership fixtures remain in
the package suite. The required three-platform source qualification is prepared
by `.github/workflows/ci.yml` on the final source branch; its raw run URLs and
results must be appended by independent QA rather than inferred from B.P2.

Independent critical review/re-review and final source acceptance remain
pending. No B.1 copy or B.7 publication was performed.

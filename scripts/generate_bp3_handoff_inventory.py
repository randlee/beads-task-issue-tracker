#!/usr/bin/env python3
"""Generate the B.P3 public/hidden-support inventory from a pinned Git revision.

The handoff is deliberately generated with ``git show REV:path`` instead of the
checkout.  That makes the review artifact continue to describe the frozen
implementation even when this evidence branch accumulates documentation-only
changes.
"""

from __future__ import annotations

import argparse
import re
import subprocess
from pathlib import Path


IMPLEMENTATION_REVISION = "c31095326bde604557dd1aa51c7252db7b3d6284"


def git_show(root: Path, revision: str, path: str) -> str:
    return subprocess.check_output(
        ["git", "-C", str(root), "show", f"{revision}:{path}"], text=True
    )


def resolved_revision(root: Path, revision: str) -> str:
    return subprocess.check_output(
        ["git", "-C", str(root), "rev-parse", revision], text=True
    ).strip()


LEDGER_GROUPS = {
    "Root contract declarations": [
        "crates/sc-observability-log/src/lib.rs",
        "crates/sc-observability-log/src/control.rs",
        "crates/sc-observability-log/src/error.rs",
        "crates/sc-observability-log/src/health.rs",
        "crates/sc-observability-log/src/error_codes.rs",
    ],
    "Hidden exact-version macro support": [
        "crates/sc-observability-log/src/callsite.rs",
        "crates/sc-observability-log/src/context.rs",
        "crates/sc-observability-log/src/mapping.rs",
    ],
    "Macro facade": ["crates/sc-observability-log-macros/src/lib.rs"],
}


def compact(lines: list[str]) -> str:
    """Keep signatures readable while preserving their source spelling."""
    return re.sub(r"\s+", " ", " ".join(line.strip() for line in lines)).strip()


def balanced_end(lines: list[str], start: int) -> int:
    """Return the end of the braced source item beginning at ``start``."""
    depth = 0
    seen_open = False
    for index in range(start, len(lines)):
        line = lines[index]
        depth += line.count("{") - line.count("}")
        seen_open = seen_open or "{" in line
        if seen_open and depth == 0:
            return index
    raise SystemExit(f"unclosed source item beginning on line {start + 1}")


def signature_end(lines: list[str], start: int) -> int:
    """Return the end of a declaration header or semicolon-delimited item."""
    for index in range(start, len(lines)):
        if "{" in lines[index] or ";" in lines[index]:
            return index
    raise SystemExit(f"unterminated source item beginning on line {start + 1}")


def semicolon_end(lines: list[str], start: int) -> int:
    """Return the semicolon ending a use/type/const declaration.

    A multiline ``pub use foo::{...};`` contains an opening brace before its
    terminating semicolon, so it must not use ``signature_end``.
    """
    for index in range(start, len(lines)):
        if ";" in lines[index]:
            return index
    raise SystemExit(f"unterminated semicolon item beginning on line {start + 1}")


def declarations(path: str, source: str) -> tuple[list[str], int, int, int]:
    """Extract source declarations instead of maintaining a hand-written API list.

    Rust's visibility is lexical; this ledger intentionally records every public
    declaration, derive, and impl in the nominated root or `__private` source
    files. Enum/struct/trait blocks retain their actual fields, payloads and
    trait methods. Impl headers plus public methods retain inherent/trait/Drop
    behavior without serializing implementation bodies.
    """
    lines = source.splitlines()
    entries: list[str] = []
    derive_count = impl_count = public_count = 0
    index = 0
    while index < len(lines):
        line = lines[index]
        stripped = line.strip()
        if (
            stripped == "#[cfg(test)]"
            and index + 1 < len(lines)
            and re.match(r"\s*mod\s+tests\s*\{", lines[index + 1])
        ):
            index = balanced_end(lines, index + 1) + 1
            continue
        if stripped.startswith("#[derive("):
            entries.append(f"- L{index + 1} derive: `{compact([line])}`")
            derive_count += 1
            index += 1
            continue
        if re.match(r"(?:pub\s+)?impl(?:<|\s)", stripped):
            end = signature_end(lines, index)
            entries.append(f"- L{index + 1} impl: `{compact(lines[index : end + 1])}`")
            impl_count += 1
            index += 1
            continue
        if not stripped.startswith("pub "):
            index += 1
            continue
        public_count += 1
        if re.match(r"pub\s+use\b", stripped):
            end = semicolon_end(lines, index)
            entries.append(
                f"- L{index + 1} public declaration: `{compact(lines[index : end + 1])}`"
            )
            index = end + 1
            continue
        if re.match(r"pub\s+(?:struct|enum|trait|mod)\b", stripped) and "{" in line:
            end = balanced_end(lines, index)
            block = "\n".join(lines[index : end + 1])
            entries.extend(
                [
                    f"- L{index + 1} public declaration:",
                    "```rust",
                    block,
                    "```",
                ]
            )
            index = end + 1
            continue
        end = signature_end(lines, index)
        entries.append(
            f"- L{index + 1} public declaration: `{compact(lines[index : end + 1])}`"
        )
        index = end + 1
    return entries, public_count, impl_count, derive_count


def extracted_ledger(sources: dict[str, str]) -> str:
    sections = [
        "\n## Source-derived declaration ledger\n",
        "This ledger is generated directly from the pinned source, not from the",
        "summary tables above. It captures public declarations (including complete",
        "struct/enum/trait blocks for fields, payloads and trait methods), every",
        "`#[derive(...)]`, every `impl` header (including trait and `Drop` impls),",
        "and the public signatures in the designated source files. The grouped",
        "disposition applies to every ledger entry in that section: root entries are",
        "the retained/revised native target contract unless explicitly internal;",
        "hidden entries are retained solely as exact-version macro support; macro",
        "entries are preserved tracing-compatible façade exports. This over-includes",
        "private-module `pub` implementation helpers so review can prove they are not",
        "silently mistaken for a root export.",
    ]
    for group, paths in LEDGER_GROUPS.items():
        sections.extend([f"\n### {group}\n"])
        for path in paths:
            entries, public_count, impl_count, derive_count = declarations(path, sources[path])
            sections.extend(
                [
                    f"#### `{path}`\n",
                    f"Source-derived counts: {public_count} public declarations, {impl_count} impl headers, {derive_count} derive lists.\n",
                    *entries,
                ]
            )
    return "\n".join(sections) + "\n"


def markdown(revision: str, sources: dict[str, str]) -> str:
    return f"""# B.P3 implementation API and macro-support inventory

Generated from implementation revision `{revision}`. Do not edit this file by
hand; regenerate it with:

```text
python3 scripts/generate_bp3_handoff_inventory.py \\
  --revision {IMPLEMENTATION_REVISION} \\
  --output docs/plans/sc-observability-runtime/bp3-implementation-inventory.md
```

The generator reads each listed file with `git show REV:path`, so this artifact
does not accidentally describe the handoff branch. The accepted target design
and reviewed runtime contract are `84b32e9d6718418371ffd25a3de52346278725ca`.
Source acceptance is pending independent QA; only runtime public-API acceptance
remains owner-deferred.

## Root contract and re-exports

Every root export/re-export below is retained as the listed target disposition.
`error_codes` is the sole public module; all other implementation modules are
private apart from the exact-version `__private` macro boundary documented
below.

| Root symbol(s) | Source | Target disposition |
| --- | --- | --- |
| `error_codes` | `lib.rs` | Preserve public stable-code registry module. |
| `BridgeEvent`, `EmitOutcome`, `LogControl` | `control.rs` via `lib.rs` | Native direct/control surface; retain non-owning control boundary. |
| `ControlError`, `DropCause`, `EmitError`, `FieldKeyError`, `FlushError`, `InitError`, `LifecyclePhase`, `ShutdownError`, `WaitError`, `ShutdownOutcome`, `ShutdownReport`, `UnconfirmedShutdown` | `error.rs` via `lib.rs` | Retain native typed, serde-tagged errors and retained completion observation. |
| `BRIDGE_HEALTH_SCHEMA_VERSION`, `BridgeHealthReport` | `health.rs` via `lib.rs` | Retain native bridge health v1. |
| `LoggerConfig` | `sc_observability` via `lib.rs` | Re-export the staged-core configuration rather than an adapter. |
| `ActionName`, `CorrelationId`, `ErrorCode`, `LevelFilter`, `LoggingHealthReport`, `OutcomeLabel`, `ProcessIdentityPolicy`, `Remediation`, `ServiceName`, `TargetCategory`, `Timestamp`, `TraceContext` | `sc_observability_types` via `lib.rs` | Re-export core-neutral contract values. |
| `AdmissionOutcome`, `EventLevel`, `LevelChange`, `LevelChangeError`, `LevelChangeSource`, `LevelState`, `LogEvent`, `LogQuery`, `LogSnapshot`, `OperationDiagnostic` | `sc_observability_types` via `lib.rs` | Re-export staged-core event/query/level contracts; `AdmissionOutcome` is also spelled `EmitOutcome`. |
| `trace!`, `debug!`, `info!`, `warn!`, `error!`, `event!`, `#[instrument]` | `sc-observability-log-macros` via `lib.rs` | Preserve tracing-compatible grammar and exact-version expansion boundary. |
| `init` | `lib.rs` | Retain process-global install returning the sole `LogGuard` lifecycle owner. |
| `Level` (`TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`) | `lib.rs` | Retain tracing-style level API; conversion targets staged-core level. |
| `BridgeOptions` | `lib.rs` | Retain bridge configuration with target-owned action parsing. |
| `DroppedEvents`, `DEFAULT_DROP_SHUTDOWN_TIMEOUT` | `lib.rs` | Retain exact-once drop accounting and bounded implicit teardown contract. |
| `LogGuard` | `lib.rs` | Retain the non-`Clone` lifecycle owner and exclusive level/shutdown authority. |

## Public native data, payloads, derives, and methods

| Symbol | Complete public shape at `{revision}` | Target disposition |
| --- | --- | --- |
| `BridgeEvent` | `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]`; public fields `level: EventLevel`, `target: TargetCategory`, `action: Option<ActionName>`, `message: Option<String>`, `outcome: Option<OutcomeLabel>`, `fields: serde_json::Map<String, serde_json::Value>`, `request_id: Option<CorrelationId>`, `correlation_id: Option<CorrelationId>`, `trace: Option<TraceContext>` | Retain typed direct producer input; bridge-owned identity/timestamp remain absent. |
| `LogControl` | `#[derive(Debug, Clone)]`; `flush(&self, Duration) -> Result<(), FlushError>`, `health(&self) -> Result<BridgeHealthReport, ControlError>`, `active_log_path(&self) -> Result<Option<PathBuf>, ControlError>`, `dropped_events(&self) -> DroppedEvents`, `wait_stopped(&self, Duration) -> Result<ShutdownReport, WaitError>`, `try_log(&self, BridgeEvent) -> Result<EmitOutcome, EmitError>`, `query(&self, &LogQuery) -> Result<LogSnapshot, ControlError>` | Retain cloneable, non-owning read/admission control; no elevate, reset, shutdown, owner conversion, or mutable logger access. |
| `Level` | `#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]`; associated constants `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`; `From<Level> for sc_observability_types::Level` | Retain fixed tracing-style API and staged-core conversion. |
| `BridgeOptions` | `#[derive(Debug, Clone)]`; public fields `default_action: ActionName`, `parse_bracket_action: bool` | Retain target action/default behavior. |
| `DroppedEvents` | `#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]`; `get(&self, DropCause) -> u64`, `total(&self) -> u64` | Retain exact-once counters; fields intentionally remain private. |
| `LogGuard` | `#[derive(Debug)]`, deliberately not `Clone`; `control(&self) -> LogControl`, `elevate_level(&mut self, LevelFilter, LevelChangeSource) -> Result<LevelChange, LevelChangeError>`, `reset_level(&mut self, LevelChangeSource) -> Result<LevelChange, LevelChangeError>`, `flush(&self, Duration) -> Result<(), FlushError>`, `shutdown(self, Duration) -> Result<(), ShutdownError>`, `dropped_events(&self) -> DroppedEvents`, `active_log_path(&self) -> Option<&Path>`, `health(&self) -> Result<BridgeHealthReport, ControlError>`; `Drop` is the bounded fallback | Retain sole ownership of runtime-level mutation and shutdown. |
| `BridgeHealthReport` | `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]`; public fields `schema_version: u32`, `logging: LoggingHealthReport`, `dropped: DroppedEvents`, `lifecycle: LifecyclePhase`, `active_log_path: Option<PathBuf>`, `configured_level: LevelFilter`, `effective_level: LevelFilter`, `level_revision: u64` | Retain native health v1 with the staged core report and bridge lifecycle/accounting fields. |
| `ShutdownReport` | `#[derive(Debug, Clone, Serialize, Deserialize)]`; public fields `outcome: ShutdownOutcome`, `health: BridgeHealthReport` | Retain final/pending shutdown observation without claiming an unconfirmed stop. |
| `DropCause` | `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]`; variants `QueueFull`, `InvalidEvent`, `WriterDegraded`, `ShutdownTimedOut`, `NotInstalled`, `LoggerPanicked`, `ReentrantEmit`; `ALL` | Retain seven exact accounting categories and declaration order. |
| `LifecyclePhase` | `#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]`; variants `Running`, `Stopping`, `Stopped`, `Failed` | Retain observable lifecycle projection. |
| `FieldKeyError` | `#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]`; variants `Empty`, `ReservedPrefix`, `Collision {{ other_raw_key: String }}` | Retain typed field-label rejection. |
| `EmitError` | `#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]`; `InvalidField {{ raw_key: String, reason: FieldKeyError }}`, `InvalidEvent {{ diagnostic: OperationDiagnostic }}`, `QueueFull {{ diagnostic: OperationDiagnostic }}`, `WriterDegraded {{ diagnostic: OperationDiagnostic }}`, `ShutdownTimedOut {{ diagnostic: OperationDiagnostic }}`, `NotRunning {{ phase: LifecyclePhase }}`, `Reentrant`, `Panicked`; `code(&self) -> ErrorCode`, `remediation(&self) -> Remediation` | Retain typed direct-admission rejection and exact-once disposition. |
| `ControlError` | `#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]`; `NotRunning {{ phase: LifecyclePhase }}`, `Query {{ diagnostic: OperationDiagnostic }}`, `Unavailable {{ diagnostic: OperationDiagnostic }}`; `code(&self) -> ErrorCode`, `remediation(&self) -> Remediation` | Retain typed read-only control failure. |
| `WaitError` | `#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]`; `NotStarted`, `TimedOut {{ timeout: Duration }}`, `Unavailable {{ diagnostic: OperationDiagnostic }}`; `code(&self) -> ErrorCode`, `remediation(&self) -> Remediation` | Retain owner-shutdown observation failure. |
| `UnconfirmedShutdown` | `#[derive(Debug, Clone, Serialize, Deserialize)]`; `HelperSpawn {{ diagnostic: OperationDiagnostic }}`, `HelperLost {{ diagnostic: OperationDiagnostic }}` | Retain explicit non-confirmation states. |
| `ShutdownOutcome` | `#[derive(Debug, Clone, Serialize, Deserialize)]`; `Stopped`, `StoppedWithFlushError {{ diagnostic: OperationDiagnostic }}`, `Unconfirmed {{ cause: UnconfirmedShutdown }}` | Retain completion truthfulness: unconfirmed is never stopped. |
| `InitError` | `#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]`; `AlreadyInitialized`, `ForeignLoggerInstalled`, `UnsupportedLevel {{ configured: LevelFilter, available: LevelFilter }}`, `IdentityResolution {{ diagnostic: OperationDiagnostic }}`, `Logger {{ diagnostic: OperationDiagnostic }}`, `RuntimeStart {{ diagnostic: OperationDiagnostic }}`; `code(&self) -> ErrorCode`, `remediation(&self) -> Remediation` | Retain typed installation/lifecycle failure. |
| `FlushError` | `#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]`; `TimedOut {{ timeout: Duration }}`, `Logger {{ diagnostic: OperationDiagnostic }}`, `HelperSpawn {{ diagnostic: OperationDiagnostic }}`, `HelperLost {{ diagnostic: OperationDiagnostic }}`, `NotRunning {{ phase: LifecyclePhase }}`, `InProgress`; `code(&self) -> ErrorCode`, `remediation(&self) -> Remediation` | Retain bounded helper-flush semantics. |
| `ShutdownError` | `#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]`; `TimedOut {{ timeout: Duration }}`, `FinalFlush {{ diagnostic: OperationDiagnostic }}`, `HelperSpawn {{ diagnostic: OperationDiagnostic }}`, `HelperLost {{ diagnostic: OperationDiagnostic }}`; `code(&self) -> ErrorCode`, `remediation(&self) -> Remediation` | Retain sole-owner bounded shutdown semantics. |

Every enum above that has `Serialize`/`Deserialize` uses its source-declared
snake_case tagged representation (`kind`/`value`) except `LifecyclePhase`,
which uses snake_case unit variants. The API-freeze test at
`crates/sc-observability-log/tests/api_freeze.rs` additionally compiles the
public methods and the promised data/error trait bounds.

## Stable error-code registry

`error_codes::ALL` retains exactly these sixteen codes, in source declaration
order: `SC_OBSERVABILITY_LOG_ALREADY_INITIALIZED`,
`SC_OBSERVABILITY_LOG_FOREIGN_LOGGER_INSTALLED`,
`SC_OBSERVABILITY_LOG_IDENTITY_RESOLUTION_FAILED`,
`SC_OBSERVABILITY_LOG_FLUSH_TIMED_OUT`,
`SC_OBSERVABILITY_LOG_SHUTDOWN_TIMED_OUT`,
`SC_OBSERVABILITY_LOG_HELPER_SPAWN_FAILED`,
`SC_OBSERVABILITY_LOG_HELPER_LOST`, `SC_OBSERVABILITY_LOG_UNSUPPORTED_LEVEL`,
`SC_OBSERVABILITY_LOG_RUNTIME_START_FAILED`, `SC_OBSERVABILITY_LOG_NOT_RUNNING`,
`SC_OBSERVABILITY_LOG_INVALID_FIELD`, `SC_OBSERVABILITY_LOG_REENTRANT_EMIT`,
`SC_OBSERVABILITY_LOG_LOGGER_PANICKED`, `SC_OBSERVABILITY_LOG_STATUS_UNAVAILABLE`,
`SC_OBSERVABILITY_LOG_SHUTDOWN_NOT_STARTED`, and
`SC_OBSERVABILITY_LOG_FLUSH_IN_PROGRESS`. Their disposition is retain: each is
the stable code returned by a native public error rather than an adapter error.

## Hidden exact-version macro support

`__private` is `#[doc(hidden)]` but externally reachable by macro expansion.
It is not a semver API: the root crate pins `sc-observability-log-macros` to an
exact `=` version. Its complete export set at this revision is:

| Hidden symbol(s) | Target disposition |
| --- | --- |
| `serde_json::Map`, `serde_json::Value`, staged-core `Level` | Retain only for generated expansion data assembly. |
| `Callsite`, `DebugKind`, `DebugKindTag`, `DynamicKey`, `FieldDebug`, `FieldRecord`, `FieldValue`, `SerializeKind`, `SerializeKindTag`, `debug_value`, `display_value`, `emit_callsite`, `record_dynamic_field`, `record_field` | Retain exact-version callsite caching and field dispatch. |
| `CallLevels`, `CallOutcome`, `CallSpan`, `Entered`, `current_trace` | Retain exact-version `#[instrument]` context/completion expansion support. |
| `LabelError`, `LabelKind`, `RESERVED_FIELD_PREFIX`, `action_label`, `field_key_label`, `sanitize_label`, `target_label` | Retain one label sanitizer and reserved-prefix rule for macros. |
| `EventParts` (`#[derive(Debug)]`; public fields `level`, `target`, `action`, `message`, `outcome`, `fields`) | Retain generated event assembly only. |
| `enabled(level)`, `emit(parts)`, `record_drop(cause)` | Retain guarded filter/submit/accounting dispatch only. |

## Feature-gated test-only export

`#[cfg(feature = "test_hooks")] #[doc(hidden)]`
`fail_next_shutdown_coordinator_reservation` is the only feature-gated root
item. Its disposition is retain as test-only deterministic lifecycle fault
injection; it is excluded from normal production feature resolution.

## Removed surface confirmation

The source task's required removals remain absent at this revision:
`report.rs`, `StructuredRecord`, `SubmitOutcome`, `SubmitError`,
`FailureReport`, `CONTROL_SCHEMA_VERSION`, `THRESHOLD`, `THRESHOLD_OFF`,
`encode_threshold`, `level_enabled`, and `to_log_level_filter`. The target
disposition for every listed spelling is removed; the source handoff's
whole-crate scan is recorded separately in the validation evidence.
""" + extracted_ledger(sources)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--revision", default=IMPLEMENTATION_REVISION)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    revision = resolved_revision(root, args.revision)
    if revision != IMPLEMENTATION_REVISION:
        raise SystemExit(
            f"expected implementation revision {IMPLEMENTATION_REVISION}, got {revision}"
        )
    paths = [
        "crates/sc-observability-log/src/lib.rs",
        "crates/sc-observability-log/src/control.rs",
        "crates/sc-observability-log/src/error.rs",
        "crates/sc-observability-log/src/health.rs",
        "crates/sc-observability-log/src/error_codes.rs",
        "crates/sc-observability-log/src/callsite.rs",
        "crates/sc-observability-log/src/context.rs",
        "crates/sc-observability-log/src/mapping.rs",
        "crates/sc-observability-log-macros/src/lib.rs",
    ]
    sources = {path: git_show(root, revision, path) for path in paths}
    output = markdown(revision, sources)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(output)


if __name__ == "__main__":
    main()

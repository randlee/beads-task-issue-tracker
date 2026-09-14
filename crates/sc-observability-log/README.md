# sc-observability-log

A `log` facade bridge for [sc-observability](https://github.com/randlee/sc-observability)
structured JSONL logging.

`sc_observability_log::init` installs a `log::Log` implementation. It maps every
`log` record to a `sc_observability_types::LogEvent` and writes it through one
process-wide `sc_observability::Logger`. Existing `log::info!` (and friends) call
sites keep working unchanged, including `target:` and the `kv` syntax
`key = value; "msg"`.

Tracing-compatible event macros (sprint a-2) and `#[instrument]` (sprint a-3)
build on the same emit path.

## Quick start

```rust,no_run
use std::time::Duration;
use sc_observability_log::{ActionName, BridgeOptions, LevelFilter, LoggerConfig, ServiceName};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = LoggerConfig::default_for(ServiceName::new("my-app")?, "/var/log/my-app".into());
    config.level = LevelFilter::Debug; // the only level setting
    let options = BridgeOptions {
        default_action: ActionName::new("log.record")?,
        parse_bracket_action: true, // "[sync.start] msg" -> action "sync.start"
    };

    // Keep the guard alive for the whole process.
    let guard = sc_observability_log::init(config, options)?;
    println!("logging to {:?}", guard.active_log_path());

    log::info!(target: "my_app::sync", "[sync.start] syncing {} items", 3);
    log::warn!(count = 3, retry = true; "kv fields become LogEvent.fields");

    guard.flush(Duration::from_secs(1))?;     // bounded flush
    guard.shutdown(Duration::from_secs(5))?;  // bounded shutdown
    Ok(())
}
```

The record mapping, the label sanitizer, the emit semantics and the full error
inventory are documented in [`docs/mapping.md`](docs/mapping.md).

## Install once

`log::set_boxed_logger` can succeed once per process and the `log` crate has no
uninstall API. A second `init` returns `InitError::AlreadyInitialized`, whether
the first guard is alive or already shut down. If another `log::Log` (for example
`tauri-plugin-log`) is installed first, `init` returns
`InitError::ForeignLoggerInstalled`.

In tests, each integration test file that calls `init` is its own process and
contains exactly one `#[test]` fn.

## Flushing

`log::logger().flush()` does nothing. `log::Log::flush` has no timeout and no
error channel, and sc-observability's flush is unbounded, so delegating to it
would let any caller hang. Call `LogGuard::flush(timeout)` instead.
`LogGuard::shutdown(timeout)` and `Drop for LogGuard` (with
`DEFAULT_DROP_SHUTDOWN_TIMEOUT`) are bounded the same way.

`Drop for LogGuard` has no `Result` to hand back to a caller, so it discards
the outcome of its flush-and-shutdown sequence (`let _ = ..`). An implicit
teardown failure (timeout, final-flush error or a lost helper thread) is
therefore silent. Call `LogGuard::shutdown(timeout)` explicitly, and act on
its `Result`, whenever the outcome matters (for example at a controlled
process exit).

## Health

`LogGuard::health()` returns a read-only `BridgeHealth` snapshot: bridge
lifecycle (`running` / `shutting_down` / `stopped`), aggregate state, writer
state, queue depth, capacity and high-water mark, the last writer error and
last error (each with its stable `ErrorCode` and a `Remediation`), file-sink
status with the active JSONL path, console-sink status, and the dropped-event
counters. It never blocks on I/O and never panics, and it can be called from
any thread.

`LogGuard::handle()` returns a `LogHandle`: `Copy`, non-owning, and unable to
shut the logger down. `LogHandle::health()` keeps working after shutdown and
then reports `lifecycle: stopped`, `state: unavailable` and the final health of
the stopped logger. The `LogGuard` stays the single lifecycle owner.

Every health type is `serde`-serializable. States are `snake_case` string
enums and the shape is versioned by `BRIDGE_HEALTH_SCHEMA_VERSION` (carried as
`schema_version`), so generated bindings never parse free-form text. Field
semantics and the code-to-remediation table are in
[`docs/mapping.md`](docs/mapping.md#health).

## Lockstep and `__private` policy

`sc-observability-log` depends on `sc-observability-log-macros` through an exact
`=` version pin, following the `serde` → `serde_core = "=1.0.228"` precedent.
Macro expansions call into the `#[doc(hidden)] sc_observability_log::__private`
module, which is **outside semver**: it may change in any release. The exact pin
keeps the two crates in lockstep. Never call `__private` directly and never
depend on `sc-observability-log-macros` on its own.

## No-panic policy

Production code in both crates contains no `unwrap`, `expect`, `panic!`,
`unreachable!`, `todo!`, `unimplemented!` or panicking indexing. This is enforced
by `[workspace.lints.clippy]` at `deny`. Poisoned locks are recovered with
`PoisonError::into_inner`.

The emit path never panics and never blocks on I/O or queue capacity. Every
dropped event is counted under exactly one `DropCause`, readable with
`LogGuard::dropped_events()`:

- `QueueFull`, `InvalidEvent`, `WriterDegraded`, `ShutdownTimedOut`: the matching
  sc-observability `TryLogError`.
- `NotInstalled`: a record before `init` or after shutdown.
- `LoggerPanicked`: a panic inside the emit guard, contained with
  `std::panic::catch_unwind`: in sc-observability `try_log`, or in a `log`
  record's own `Display`/`Debug` formatting (message arguments or a key-value).
- `ReentrantEmit`: a record logged while the emit guard is active on the same
  thread: from a formatter of the record being emitted, a panic hook, a sink or
  a redactor.

For a `log` record, the guard is entered exactly once and covers the slot read,
target/action labelling, message and key-value rendering, event assembly,
redaction and `try_log`; only the lock-free level check runs before it.

### Residual: `panic = "abort"`

`catch_unwind` has no effect when the final binary is built with
`panic = "abort"`. A panic inside sc-observability then aborts the process
instead of being counted as `DropCause::LoggerPanicked`. The helper threads used
by `flush` and `shutdown` likewise cannot contain such a panic.

## No message or field size cap

`record_to_parts` copies `log::Record::args()` and every `kv` field into the
`LogEvent` unbounded; sc-observability 1.2.0 has no per-event or per-field
byte-size limit either (`rotation_max_bytes` on `LoggerConfig` bounds the
active JSONL *file*, not a single event, and `Logger::try_log`'s queue is
bounded by event *count*, via `queue_capacity`, not by bytes). An unusually
large message or field set is therefore emitted as-is; it is bounded only by
available memory and by the sink's own write path. No truncation is
implemented in this crate: adding one would need a documented, callable
policy (which limit, which fields, silent vs. counted truncation) and is left
for a future sprint if event or field sizes prove to be a real-world problem.

## Process identity

sc-observability 1.2.0 stores `LoggerConfig.process_identity` but never applies
it, so `init` resolves it once: `Auto` records the current pid (no hostname),
`Fixed` is used as given, and `Resolver` failures fail `init` with
`InitError::IdentityResolution`.

## License

MIT

//! One `init` per test binary, in a child process: the bridge cannot be replaced after shutdown.
//!
//! R-A4-005 design evidence 4 (install-once). The single test fn re-executes this
//! test binary with `CHILD_ENV` set. The child installs the bridge, shuts it
//! down, then tries to reinstall it (`init` again) and to replace it with a
//! foreign `log::Log`, and reports every outcome as one JSON line built only from
//! the serializable contracts (`FailureReport`, `BridgeHealthReport`). The parent
//! never calls `init`; it checks the child's exit status, the reported codes and
//! the JSONL file from outside the child process.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration test: helper fns are not covered by clippy.toml allow-*-in-tests"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use sc_observability_log::{
    ActionName, BridgeOptions, Level, LevelFilter, LoggerConfig, ServiceName, StructuredRecord,
};
use serde_json::{Value, json};

const CHILD_ENV: &str = "SC_OBSERVABILITY_LOG_REINSTALL_CHILD_ROOT";
const RESULT_PREFIX: &str = "REINSTALL_CHILD_RESULT ";
const TEST_NAME: &str = "bridge_cannot_be_reinstalled_or_replaced_after_shutdown";
const CHILD_DEADLINE: Duration = Duration::from_secs(60);

struct ForeignLogger;

impl log::Log for ForeignLogger {
    fn enabled(&self, _: &log::Metadata<'_>) -> bool {
        true
    }
    fn log(&self, _: &log::Record<'_>) {}
    fn flush(&self) {}
}

fn config(root: &Path) -> (LoggerConfig, BridgeOptions) {
    let mut config = LoggerConfig::default_for(
        ServiceName::new("reinstall-child").unwrap(),
        root.to_path_buf(),
    );
    config.level = LevelFilter::Info;
    config.enable_console_sink = false;
    let options = BridgeOptions {
        default_action: ActionName::new("log.record").unwrap(),
        parse_bracket_action: false,
    };
    (config, options)
}

/// The child process: install, shut down, then try to reinstall and replace.
fn run_child(root: &Path) {
    let (first_config, first_options) = config(root);
    let guard = sc_observability_log::init(first_config, first_options).unwrap();
    let control = guard.control();
    log::info!(target: "reinstall", "record before shutdown");
    let before = control
        .submit(
            StructuredRecord::new(Level::INFO, "reinstall").with_message("submit before shutdown"),
        )
        .unwrap();
    guard.shutdown(Duration::from_secs(5)).unwrap();

    let (second_config, second_options) = config(root);
    let reinit = sc_observability_log::init(second_config, second_options)
        .map(|_guard| ())
        .unwrap_err()
        .report();
    let foreign_rejected = log::set_boxed_logger(Box::new(ForeignLogger)).is_err();
    log::error!(target: "reinstall", "record after reinstall attempt");
    let submit_after = control
        .submit(
            StructuredRecord::new(Level::ERROR, "reinstall").with_message("submit after shutdown"),
        )
        .unwrap_err()
        .report();
    let flush_after = control.flush(Duration::from_secs(1)).unwrap_err().report();
    let result = json!({
        "submit_before": before,
        "reinit": reinit,
        "foreign_logger_rejected": foreign_rejected,
        "submit_after": submit_after,
        "flush_after": flush_after,
        "health": control.health().unwrap(),
        "active_log_path": control.active_log_path().unwrap(),
    });
    // libtest may print "test <name> ... " on the same line first.
    println!("{RESULT_PREFIX}{result}");
}

fn run_parent() {
    let root = tempfile::tempdir().unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", TEST_NAME, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, root.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > CHILD_DEADLINE {
            let _ = child.kill();
            panic!("child did not exit within {CHILD_DEADLINE:?}");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let output = child.wait_with_output().unwrap();
    assert!(status.success(), "child failed: {status}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let line = stdout
        .lines()
        .find_map(|line| line.split_once(RESULT_PREFIX).map(|(_, json)| json))
        .unwrap_or_else(|| panic!("no child result in:\n{stdout}"));
    let result: Value = serde_json::from_str(line).unwrap();

    assert_eq!(result["submit_before"], "accepted");
    assert_eq!(result["reinit"]["failure"]["operation"], "init");
    assert_eq!(result["reinit"]["failure"]["kind"], "already_initialized");
    assert_eq!(
        result["reinit"]["code"],
        "SC_OBSERVABILITY_LOG_ALREADY_INITIALIZED"
    );
    assert_eq!(result["reinit"]["remediation"]["kind"], "not_recoverable");
    assert_eq!(result["foreign_logger_rejected"], true);
    assert_eq!(
        result["submit_after"]["failure"],
        json!({"operation": "submit", "kind": "stopped", "lifecycle": "stopped"})
    );
    assert_eq!(
        result["submit_after"]["code"],
        "SC_OBSERVABILITY_LOG_SUBMIT_STOPPED"
    );
    assert_eq!(result["flush_after"]["failure"]["kind"], "shut_down");
    assert_eq!(result["health"]["lifecycle"], "stopped");
    assert_eq!(result["health"]["logging"]["writer_state"], "Stopped");
    assert_eq!(result["health"]["dropped"]["not_installed"], 1);

    let path = PathBuf::from(result["active_log_path"].as_str().unwrap());
    assert!(path.starts_with(root.path()));
    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(contents.contains("record before shutdown"));
    assert!(contents.contains("submit before shutdown"));
    assert!(!contents.contains("after reinstall attempt"));
    assert!(!contents.contains("submit after shutdown"));
}

#[test]
fn bridge_cannot_be_reinstalled_or_replaced_after_shutdown() {
    match std::env::var_os(CHILD_ENV) {
        Some(root) => run_child(Path::new(&root)),
        None => run_parent(),
    }
}

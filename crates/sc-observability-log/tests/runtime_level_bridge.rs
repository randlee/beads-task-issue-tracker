//! B.P3 direct-path and shared-core runtime-level fixture.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration fixture keeps one process-global bridge installation"
)]

use std::time::Duration;

use sc_observability_log::{
    ActionName, AdmissionOutcome, BridgeEvent, BridgeOptions, LevelChange, LevelChangeSource,
    LevelFilter, LoggerConfig, ServiceName, TargetCategory,
};

struct Scratch(std::path::PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn direct_facade_and_macro_admission_share_the_core_level_owner() {
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "sc-observability-log-bp3-runtime-{}",
        std::process::id()
    )));
    let mut config =
        LoggerConfig::default_for(ServiceName::new("bp3-runtime").unwrap(), scratch.0.clone());
    config.level = LevelFilter::Info;
    config.enable_console_sink = false;
    let mut guard = sc_observability_log::init(
        config,
        BridgeOptions {
            default_action: ActionName::new("log.record").unwrap(),
            parse_bracket_action: false,
        },
    )
    .unwrap();
    let control = guard.control();
    let direct = || BridgeEvent {
        level: sc_observability_log::EventLevel::Debug,
        target: TargetCategory::new("bp3.direct").unwrap(),
        action: Some(ActionName::new("runtime.level").unwrap()),
        message: Some("direct debug event".to_owned()),
        outcome: None,
        fields: serde_json::Map::new(),
        request_id: None,
        correlation_id: None,
        trace: None,
    };

    assert_eq!(
        control.try_log(direct()).unwrap(),
        AdmissionOutcome::Filtered
    );
    assert!(matches!(
        guard
            .elevate_level(LevelFilter::Debug, LevelChangeSource::DiagnosticSession)
            .unwrap(),
        LevelChange::Changed { .. }
    ));
    assert_eq!(
        control.try_log(direct()).unwrap(),
        AdmissionOutcome::Accepted
    );
    sc_observability_log::debug!(target: "bp3.macro", "macro debug event");
    log::debug!(target: "bp3.facade", "facade debug event");

    let health = guard.health();
    assert_eq!(health.configured_level, LevelFilter::Info);
    assert_eq!(health.effective_level, LevelFilter::Debug);
    assert!(health.level_revision >= 1);
    guard.shutdown(Duration::from_secs(5)).unwrap();
    assert!(matches!(
        control
            .wait_stopped(Duration::from_secs(1))
            .unwrap()
            .outcome,
        sc_observability_log::ShutdownOutcome::Stopped
    ));
}

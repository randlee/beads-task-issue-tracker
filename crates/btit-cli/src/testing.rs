//! [`RecordingInvoker`]: a scripted [`CliInvoker`] for tests (feature `test-support`).
//!
//! No process is spawned. Every call records its full argv, and replies are taken
//! from per-form queues in the order they were scripted. Used by this crate's tests
//! and, through `btit-cli = { features = ["test-support"] }` dev-dependencies, by
//! `btit-bd`, `btit-br` and the app.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};

use btit_beads::error::BeadsError;
use btit_types::{CliOutput, CliProbe, ProjectRef};

use crate::runner::CliInvoker;

/// Scripted `CliInvoker` for unit tests in btit-cli, btit-bd, btit-br and the app: no process is spawned.
///
/// `client_info()` returns the seeded probe unchanged (`None` is the "no probe"
/// case: client `Unknown`, all capabilities `false`). `run_json` records the argv
/// [`run::json_invocation`](crate::run::json_invocation) assembles from that probe, the
/// same step [`CliRunner`](crate::CliRunner) spawns; `run_raw` records its `args`
/// verbatim. With an empty queue, `run_json` replies `Ok("")` and `run_raw` replies a
/// successful, empty [`CliOutput`].
#[derive(Debug)]
pub struct RecordingInvoker {
    binary: String,
    probe: Option<CliProbe>,
    json_replies: Mutex<VecDeque<Result<String, BeadsError>>>,
    raw_replies: Mutex<VecDeque<Result<CliOutput, BeadsError>>>,
    calls: Mutex<Vec<Vec<String>>>,
    probe_calls: AtomicUsize,
}

impl RecordingInvoker {
    /// An invoker for binary `"bd"` seeded with `probe` (`None` = no probe).
    #[must_use]
    pub fn new(probe: Option<CliProbe>) -> Self {
        Self {
            binary: "bd".to_string(),
            probe,
            json_replies: Mutex::new(VecDeque::new()),
            raw_replies: Mutex::new(VecDeque::new()),
            calls: Mutex::new(Vec::new()),
            probe_calls: AtomicUsize::new(0),
        }
    }

    /// Sets the name [`CliInvoker::binary`] reports.
    #[must_use]
    pub fn with_binary(mut self, binary: impl Into<String>) -> Self {
        self.binary = binary.into();
        self
    }

    /// Queues the reply of the next unanswered `run_json` call.
    #[must_use]
    pub fn reply_json(mut self, reply: Result<String, BeadsError>) -> Self {
        self.json_replies
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .push_back(reply);
        self
    }

    /// Queues the reply of the next unanswered `run_raw` call.
    #[must_use]
    pub fn reply_raw(mut self, reply: Result<CliOutput, BeadsError>) -> Self {
        self.raw_replies
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .push_back(reply);
        self
    }

    /// The full argv of every call so far, in call order, e.g. `["list", "--limit=0", "--json"]`.
    #[must_use]
    pub fn calls(&self) -> Vec<Vec<String>> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// How many times [`CliInvoker::probe`] was called.
    #[must_use]
    pub fn probe_calls(&self) -> usize {
        self.probe_calls.load(Ordering::SeqCst)
    }

    fn record(&self, argv: Vec<String>) {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(argv);
    }
}

impl CliInvoker for RecordingInvoker {
    fn binary(&self) -> String {
        self.binary.clone()
    }

    fn client_info(&self) -> Option<CliProbe> {
        self.probe.clone()
    }

    fn probe(&self) -> Option<CliProbe> {
        self.probe_calls.fetch_add(1, Ordering::SeqCst);
        self.probe.clone()
    }

    fn run_json(
        &self,
        _project: &ProjectRef,
        command: &str,
        args: &[String],
    ) -> Result<String, BeadsError> {
        // the same step CliRunner spawns
        let (_client, full_args) = crate::run::json_invocation(self.probe.as_ref(), command, args);
        self.record(full_args);
        self.json_replies
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front()
            .unwrap_or_else(|| Ok(String::new()))
    }

    fn run_raw(&self, _project: &ProjectRef, args: &[&str]) -> Result<CliOutput, BeadsError> {
        self.record(args.iter().map(ToString::to_string).collect());
        self.raw_replies
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front()
            .unwrap_or_else(|| {
                Ok(CliOutput {
                    status: Some(0),
                    success: true,
                    stdout: String::new(),
                    stderr: String::new(),
                })
            })
    }
}

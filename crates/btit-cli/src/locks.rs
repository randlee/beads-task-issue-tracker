//! [`ProjectLocks`]: one mutex per project working directory.
//!
//! bd 0.55 uses embedded Dolt, which crashes (SIGSEGV) when two bd processes access
//! the same database simultaneously, so every `--json` invocation holds the lock of
//! its working directory for the lifetime of the child process. Entries are never
//! removed: the map grows by one entry per distinct working directory.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

/// Per-project mutexes, keyed by working directory.
///
/// Shared by every runner of one app through an `Arc`, so two runners for the same
/// project serialize their calls.
#[derive(Debug, Default)]
pub struct ProjectLocks {
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl ProjectLocks {
    /// An empty lock map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The mutex for `working_dir`, created on first use.
    ///
    /// A poisoned map (a thread panicked while inserting) is recovered rather than
    /// propagated: the map holds no invariant a panic could break.
    #[must_use]
    pub fn guard(&self, working_dir: &str) -> Arc<Mutex<()>> {
        let mut locks = self.locks.lock().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(
            locks
                .entry(working_dir.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(()))),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_working_dir_shares_one_mutex() {
        let locks = ProjectLocks::new();
        let a = locks.guard("/p/one");
        let b = locks.guard("/p/one");
        let c = locks.guard("/p/two");
        assert!(Arc::ptr_eq(&a, &b));
        assert!(!Arc::ptr_eq(&a, &c));
    }

    #[test]
    fn poisoned_map_is_recovered() {
        let locks = Arc::new(ProjectLocks::new());
        let first = locks.guard("/p");
        let poisoner = Arc::clone(&locks);
        let joined = std::thread::spawn(move || {
            let _held = poisoner.locks.lock().unwrap();
            panic!("poison the map");
        })
        .join();
        assert!(joined.is_err());
        assert!(locks.locks.is_poisoned());
        assert!(Arc::ptr_eq(&first, &locks.guard("/p")));
    }
}

#[cfg(test)]
use std::sync::{Mutex, MutexGuard, OnceLock};

#[cfg(test)]
static TEST_ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

#[cfg(test)]
pub(crate) fn lock_test_env() -> MutexGuard<'static, ()> {
    // 2026-04-17 CST: Added because multiple lib tests mutate process-wide env
    // variables and can race under the default Rust test runner.
    // Purpose: serialize test-only env overrides so fallback-path assertions stay deterministic.
    TEST_ENV_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("test env mutex should not be poisoned")
}

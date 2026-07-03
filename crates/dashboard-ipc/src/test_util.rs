//! Serialize tests that mutate `ANYCODE_DASHBOARD_STATE_DIR`.

use std::sync::Mutex;

static STATE_DIR_TEST_LOCK: Mutex<()> = Mutex::new(());

pub fn lock_state_dir_env() -> std::sync::MutexGuard<'static, ()> {
    STATE_DIR_TEST_LOCK.lock().expect("state dir test lock")
}

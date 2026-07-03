//! File-based IPC shared by dashboard, bootstrap, and channel bridges.

pub mod approval_ipc;
pub mod cancel_ipc;
pub mod host_guard;
pub mod question_ipc;

#[cfg(test)]
mod test_util;

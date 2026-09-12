//! Pi-inspired, independent Rust execution kernel. No UI, cloud or device authority.
//! Host adapters retain provider-native messages and must enforce their own sandbox.
#![forbid(unsafe_code)]
pub mod approval;
pub mod budget;
pub mod context;
pub mod digest;
pub mod error;
pub mod events;
pub mod journal;
pub mod kernel;
pub mod run_store;
pub mod session;
pub mod types;
pub use context::{Capabilities, RunContext, Scope};
pub use error::{Error, Result};

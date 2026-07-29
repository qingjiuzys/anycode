//! A2A cloud team handoff — presence, task metadata, in-memory stream relay (no OSS).

pub mod handlers;
pub mod models;
pub mod relay;
pub mod store;

pub use relay::StreamRelay;

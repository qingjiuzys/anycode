//! Agent crate inline tests (split from legacy `agent_test_mod.inc`).

mod graph_engine_run;
#[cfg(feature = "harness-v1")]
mod harness_pilot;
#[cfg(feature = "harness-v1")]
mod harness_unified;
mod integration;
mod support;
mod tool_pairing;
mod unit;

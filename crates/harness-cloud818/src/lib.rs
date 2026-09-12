//! 818cloud SSO v2 adapter. SERVER-SIDE confidential client only, never bundle
//! PRODUCT_SSO_CLIENT_SECRET in Tauri, browser code, a skill or a distributed CLI.
#![forbid(unsafe_code)]
pub mod desktop_pairing;
pub mod identity;
pub mod metering;

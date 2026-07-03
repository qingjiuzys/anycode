//! Local relay gateway: OpenAI-compatible proxy over `~/.anycode/relay.json`.

pub mod proxy;
pub mod router;
pub mod server;
pub mod store;

pub use server::{spawn_gateway, GatewayConfig};
pub use store::{
    load_relay_store, merge_accounts_preserving_keys, normalize_relay_config, public_account,
    relay_path, save_relay_store, RelayAccount, RelayConfig, RelayModel, RelayStore,
};

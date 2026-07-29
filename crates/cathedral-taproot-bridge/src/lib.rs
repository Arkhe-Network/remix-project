pub mod auth;
pub mod client;
pub mod error;
pub mod identity;

pub use auth::Macaroon;
pub use client::TaprootClient;
pub use error::BridgeError;
pub use identity::asset_ref_to_did;

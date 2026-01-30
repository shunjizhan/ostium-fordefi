//! Ostium SDK for Rust
//!
//! A Rust SDK for interacting with Ostium perpetuals protocol on Arbitrum One mainnet
//! using Fordefi MPC wallet for secure transaction signing.
//!
//! # Features
//!
//! - **Trading**: Open/close BTC perpetual positions with configurable leverage
//! - **OLP Vault**: Deposit USDC, request withdrawals, approve auto-withdraw
//! - **Fordefi MPC**: Secure institutional-grade signing via Fordefi API
//!
//! # Example
//!
//! ```rust,ignore
//! use ostium_sdk::{OstiumClient, NetworkConfig, FordefiSigner, get_btc_price, PlaceOrderParams};
//!
//! #[tokio::main]
//! async fn main() -> eyre::Result<()> {
//!     let config = NetworkConfig::default();
//!     let jwt_token = std::env::var("FORDEFI_JWT_TOKEN")?;
//!     let private_key_pem = std::fs::read_to_string("keys/pk.pem")?;
//!
//!     let signer = FordefiSigner::discover(&jwt_token, &private_key_pem, &config.rpc_url).await?;
//!     let client = OstiumClient::new(signer, config).await?;
//!
//!     // Get current BTC price and place a trade
//!     let btc_price = get_btc_price().await?;
//!     let tx_hash = client.place_order(
//!         PlaceOrderParams::market(0, 10.0, 10.0, true) // BTC/USD, 10 USDC, 10x, long
//!             .with_open_price(btc_price)
//!             .with_slippage(2.0),
//!         None
//!     ).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod config;
pub mod constants;
pub mod contracts;
pub mod error;
pub mod price;
pub mod signer;
pub mod types;

// Re-export main types for convenience
pub use client::OstiumClient;
pub use config::NetworkConfig;
pub use error::{eyre, Context, Report, Result};
pub use price::{get_btc_price, get_eth_price, get_price};
pub use signer::{FordefiSigner, TransactionSigner, TxRequest};
pub use types::{CloseTradeParams, DepositParams, PlaceOrderParams, Position, VaultEpoch, VaultPosition};

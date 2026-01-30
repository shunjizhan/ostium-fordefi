//! Ostium SDK for Rust
//!
//! A Rust SDK for interacting with Ostium perpetuals protocol on Arbitrum One mainnet.
//!
//! # Features
//!
//! - Place trades (market/limit/stop orders)
//! - Deposit to OLP vault
//! - Withdraw from OLP vault
//!
//! # Example
//!
//! ```rust,ignore
//! use ostium_sdk::{OstiumClient, NetworkConfig, LocalSigner, get_btc_price, PlaceOrderParams};
//!
//! #[tokio::main]
//! async fn main() -> eyre::Result<()> {
//!     let config = NetworkConfig::default();
//!     let signer = LocalSigner::from_private_key("0x...", &config.rpc_url).await?;
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
pub mod subgraph;
pub mod types;

// Re-export main types for convenience
pub use client::OstiumClient;
pub use config::{FordefiConfig, NetworkConfig};
pub use error::{eyre, Context, Report, Result};
pub use price::{get_btc_price, get_eth_price, get_price};
pub use signer::{FordefiSigner, LocalSigner, TransactionSigner, TxRequest};
pub use subgraph::{OpenTrade, SubgraphClient};
pub use types::{
    BuilderFeeParams, CloseTradeParams, DepositParams, PlaceOrderParams, Position, RedeemParams,
    VaultEpoch, VaultPosition, WithdrawParams,
};

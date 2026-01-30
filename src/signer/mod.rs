//! Transaction signer abstraction for Ostium SDK
//!
//! This module provides a trait-based abstraction for signing and sending transactions,
//! allowing the SDK to work with both local private keys (Phase 1) and
//! Fordefi MPC wallets (Phase 2).

mod fordefi;
mod local;

pub use fordefi::FordefiSigner;
pub use local::LocalSigner;

use alloy::primitives::{Address, Bytes, TxHash, U256};
use alloy::rpc::types::TransactionReceipt;
use eyre::Result;

/// Transaction request parameters
#[derive(Debug, Clone)]
pub struct TxRequest {
    /// Target contract address
    pub to: Address,
    /// Transaction value in wei
    pub value: U256,
    /// Encoded calldata
    pub data: Bytes,
    /// Optional gas limit override
    pub gas_limit: Option<u64>,
}

impl TxRequest {
    /// Create a new transaction request
    pub fn new(to: Address, data: impl Into<Bytes>) -> Self {
        Self {
            to,
            value: U256::ZERO,
            data: data.into(),
            gas_limit: None,
        }
    }

    /// Set transaction value
    pub fn with_value(mut self, value: U256) -> Self {
        self.value = value;
        self
    }

    /// Set gas limit
    pub fn with_gas_limit(mut self, gas_limit: u64) -> Self {
        self.gas_limit = Some(gas_limit);
        self
    }
}

/// Trait for signing and sending EVM transactions
///
/// This abstraction allows the SDK to work with different signing mechanisms:
/// - `LocalSigner`: Uses a local private key (Phase 1)
/// - `FordefiSigner`: Uses Fordefi MPC API (Phase 2)
pub trait TransactionSigner: Send + Sync {
    /// Returns the signer's EVM address
    fn address(&self) -> Address;

    /// Signs and sends a transaction, returning the transaction hash
    fn sign_and_send(
        &self,
        tx: TxRequest,
    ) -> impl std::future::Future<Output = Result<TxHash>> + Send;

    /// Waits for a transaction to be confirmed and returns the receipt
    fn wait_for_receipt(
        &self,
        tx_hash: TxHash,
    ) -> impl std::future::Future<Output = Result<TransactionReceipt>> + Send;

    /// Gets the native token balance (ETH on Arbitrum)
    fn get_balance(&self) -> impl std::future::Future<Output = Result<U256>> + Send;
}

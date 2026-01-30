//! Vault types for user-facing API

use crate::constants::{scale_usdc, unscale_from_decimals, USDC_DECIMALS};
use alloy::primitives::{Address, U256};

/// Parameters for depositing to OLP vault
#[derive(Debug, Clone)]
pub struct DepositParams {
    /// Amount of USDC to deposit
    pub amount: f64,
    /// Receiver address for OLP shares (defaults to sender)
    pub receiver: Option<Address>,
}

impl DepositParams {
    /// Create deposit params with amount
    pub fn new(amount: f64) -> Self {
        Self {
            amount,
            receiver: None,
        }
    }

    /// Get scaled USDC amount
    pub fn scaled_amount(&self) -> U256 {
        scale_usdc(self.amount)
    }
}

/// User's OLP vault position
#[derive(Debug, Clone)]
pub struct VaultPosition {
    /// OLP share balance
    pub shares: U256,
    /// Equivalent USDC value
    pub value: f64,
}

impl VaultPosition {
    /// Create from raw values
    pub fn new(shares: U256, assets: U256) -> Self {
        Self {
            shares,
            value: unscale_from_decimals(assets, USDC_DECIMALS),
        }
    }

    /// Get shares as f64 (with 6 decimals)
    pub fn shares_f64(&self) -> f64 {
        unscale_from_decimals(self.shares, USDC_DECIMALS)
    }
}

/// Vault epoch information
#[derive(Debug, Clone)]
pub struct VaultEpoch {
    /// Current epoch number
    pub current_epoch: u64,
    /// Epoch start timestamp (Unix timestamp)
    pub epoch_start_timestamp: u64,
    /// Epoch end timestamp (Unix timestamp)
    pub epoch_end_timestamp: u64,
    /// Whether withdrawals are currently open (first 48h of epoch)
    pub withdrawals_open: bool,
}

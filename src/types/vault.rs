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

    /// Set receiver address
    pub fn with_receiver(mut self, receiver: Address) -> Self {
        self.receiver = Some(receiver);
        self
    }

    /// Get scaled USDC amount
    pub fn scaled_amount(&self) -> U256 {
        scale_usdc(self.amount)
    }
}

/// Parameters for withdrawing from OLP vault
#[derive(Debug, Clone)]
pub struct WithdrawParams {
    /// Amount of USDC to withdraw
    pub amount: f64,
    /// Receiver address for USDC (defaults to sender)
    pub receiver: Option<Address>,
}

impl WithdrawParams {
    /// Create withdraw params with amount
    pub fn new(amount: f64) -> Self {
        Self {
            amount,
            receiver: None,
        }
    }

    /// Set receiver address
    pub fn with_receiver(mut self, receiver: Address) -> Self {
        self.receiver = Some(receiver);
        self
    }

    /// Get scaled USDC amount
    pub fn scaled_amount(&self) -> U256 {
        scale_usdc(self.amount)
    }
}

/// Parameters for redeeming OLP shares
#[derive(Debug, Clone)]
pub struct RedeemParams {
    /// Amount of OLP shares to redeem
    pub shares: U256,
    /// Receiver address for USDC (defaults to sender)
    pub receiver: Option<Address>,
}

impl RedeemParams {
    /// Create redeem params with shares
    pub fn new(shares: U256) -> Self {
        Self {
            shares,
            receiver: None,
        }
    }

    /// Set receiver address
    pub fn with_receiver(mut self, receiver: Address) -> Self {
        self.receiver = Some(receiver);
        self
    }
}

/// OLP vault information
#[derive(Debug, Clone)]
pub struct VaultInfo {
    /// Total assets (USDC) managed by vault
    pub total_assets: f64,
    /// Total OLP shares issued
    pub total_shares: U256,
    /// Current share price (assets per share)
    pub share_price: f64,
    /// Current epoch number
    pub current_epoch: u64,
    /// Whether withdrawals are open
    pub withdrawals_open: bool,
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
    /// Epoch end timestamp (Unix timestamp)
    pub epoch_end_timestamp: u64,
    /// Whether withdrawals are currently open
    pub withdrawals_open: bool,
}

impl VaultEpoch {
    /// Get the next withdrawal epoch (current + 1)
    pub fn next_withdraw_epoch(&self) -> u64 {
        self.current_epoch + 1
    }
}

/// Locked deposit information
#[derive(Debug, Clone)]
pub struct LockedDeposit {
    /// Owner address
    pub owner: Address,
    /// Locked shares
    pub shares: U256,
    /// Assets deposited
    pub assets_deposited: f64,
    /// Assets discount
    pub assets_discount: f64,
    /// Timestamp when locked
    pub locked_at: u64,
    /// Lock duration in seconds
    pub lock_duration: u64,
}

impl LockedDeposit {
    /// Check if deposit is unlocked
    pub fn is_unlocked(&self, current_timestamp: u64) -> bool {
        current_timestamp >= self.locked_at + self.lock_duration
    }
}

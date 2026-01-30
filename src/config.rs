//! Network configuration for Ostium SDK

use alloy::primitives::Address;

/// Network configuration containing RPC URLs and contract addresses (Arbitrum One mainnet)
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Chain ID (42161 for Arbitrum One)
    pub chain_id: u64,
    /// RPC endpoint URL
    pub rpc_url: String,
    /// USDC token address
    pub usdc: Address,
    /// Trading contract address
    pub trading: Address,
    /// TradingStorage contract address
    pub trading_storage: Address,
    /// OLP Vault contract address (optional, for vault operations)
    pub vault: Option<Address>,
    /// Auto-withdraw contract address (approves OLP for automatic withdrawals)
    pub auto_withdraw: Option<Address>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkConfig {
    /// Create Arbitrum One mainnet configuration (default)
    pub fn new() -> Self {
        let alchemy_key = std::env::var("ALCHEMY_API_KEY")
            .expect("ALCHEMY_API_KEY environment variable must be set");

        Self {
            chain_id: 42161,
            rpc_url: format!("https://arb-mainnet.g.alchemy.com/v2/{}", alchemy_key),
            usdc: "0xaf88d065e77c8cC2239327C5EDb3A432268e5831"
                .parse()
                .unwrap(),
            trading: "0x6D0bA1f9996DBD8885827e1b2e8f6593e7702411"
                .parse()
                .unwrap(),
            trading_storage: "0xcCd5891083A8acD2074690F65d3024E7D13d66E7"
                .parse()
                .unwrap(),
            vault: Some(
                "0x20d419a8e12c45f88fda7c5760bb6923cee27f98"
                    .parse()
                    .unwrap(),
            ),
            auto_withdraw: Some(
                "0x6297ce1a61c2c8a72bfb0de957f6b1cf0413141e"
                    .parse()
                    .unwrap(),
            ),
        }
    }

    /// Alias for new() - Arbitrum One mainnet configuration
    pub fn mainnet() -> Self {
        Self::new()
    }

    /// Create custom configuration with specific RPC URL
    pub fn with_rpc_url(mut self, rpc_url: impl Into<String>) -> Self {
        self.rpc_url = rpc_url.into();
        self
    }

    /// Set the vault address
    pub fn with_vault(mut self, vault: Address) -> Self {
        self.vault = Some(vault);
        self
    }

    /// Set the auto-withdraw address
    pub fn with_auto_withdraw(mut self, auto_withdraw: Address) -> Self {
        self.auto_withdraw = Some(auto_withdraw);
        self
    }
}

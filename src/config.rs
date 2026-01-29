//! Network configuration for Ostium SDK

use alloy::primitives::Address;

/// Network configuration containing RPC URLs and contract addresses (Arbitrum One mainnet)
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Chain ID (42161 for Arbitrum One)
    pub chain_id: u64,
    /// RPC endpoint URL
    pub rpc_url: String,
    /// Subgraph URL for querying positions
    pub subgraph_url: String,
    /// USDC token address
    pub usdc: Address,
    /// Trading contract address
    pub trading: Address,
    /// TradingStorage contract address
    pub trading_storage: Address,
    /// OLP Vault contract address (optional, for vault operations)
    pub vault: Option<Address>,
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
            subgraph_url: "https://subgraph.satsuma-prod.com/391a61815d32/ostium/ost-prod/api"
                .to_string(),
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
}

/// Fordefi API configuration for Phase 2
#[derive(Debug, Clone)]
pub struct FordefiConfig {
    /// Fordefi API base URL
    pub api_base_url: String,
    /// JWT access token
    pub access_token: String,
    /// PEM-encoded P-256 private key for request signing
    pub api_private_key_pem: String,
    /// Fordefi vault ID containing the EVM wallet
    pub vault_id: String,
    /// EVM address of the wallet in the vault
    pub address: Address,
}

impl FordefiConfig {
    /// Create new Fordefi configuration
    pub fn new(
        access_token: impl Into<String>,
        api_private_key_pem: impl Into<String>,
        vault_id: impl Into<String>,
        address: Address,
    ) -> Self {
        Self {
            api_base_url: "https://api.fordefi.com".to_string(),
            access_token: access_token.into(),
            api_private_key_pem: api_private_key_pem.into(),
            vault_id: vault_id.into(),
            address,
        }
    }
}

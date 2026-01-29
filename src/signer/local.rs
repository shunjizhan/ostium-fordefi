//! Local private key signer implementation (Phase 1)

use super::{TransactionSigner, TxRequest};
use alloy::network::{Ethereum, EthereumWallet, TransactionBuilder};
use alloy::primitives::{Address, TxHash, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::TransactionReceipt;
use alloy::signers::local::PrivateKeySigner;
use alloy::transports::http::reqwest::Url;
use eyre::{Context, Result};
use std::sync::Arc;

/// Local signer using a private key
///
/// This is the Phase 1 implementation that signs transactions locally
/// using a raw EVM private key.
pub struct LocalSigner {
    /// Provider with wallet filler - handles nonce, gas, chain_id, and signing
    provider: Arc<dyn Provider<Ethereum>>,
    address: Address,
}

impl LocalSigner {
    /// Create a new LocalSigner from a private key hex string
    ///
    /// # Arguments
    ///
    /// * `private_key` - Hex-encoded private key (with or without 0x prefix)
    /// * `rpc_url` - RPC endpoint URL
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let signer = LocalSigner::from_private_key(
    ///     "0x...",
    ///     "https://arb1.arbitrum.io/rpc"
    /// ).await?;
    /// ```
    pub async fn from_private_key(
        private_key: impl AsRef<str>,
        rpc_url: impl AsRef<str>,
    ) -> Result<Self> {
        let key = private_key.as_ref();
        let key = key.strip_prefix("0x").unwrap_or(key);

        let signer: PrivateKeySigner = key.parse().context("Failed to parse private key")?;

        let address = signer.address();
        let wallet = EthereumWallet::from(signer);

        let url: Url = rpc_url.as_ref().parse().context("Invalid RPC URL")?;

        // Build provider with wallet filler - this handles nonce, gas, and signing
        let provider = ProviderBuilder::new()
            .wallet(wallet)
            .connect_http(url);

        Ok(Self {
            provider: Arc::new(provider),
            address,
        })
    }
}

impl TransactionSigner for LocalSigner {
    fn address(&self) -> Address {
        self.address
    }

    async fn sign_and_send(&self, tx: TxRequest) -> Result<TxHash> {
        let mut tx_request = alloy::rpc::types::TransactionRequest::default()
            .with_to(tx.to)
            .with_value(tx.value)
            .with_input(tx.data);

        // Set gas limit if provided
        if let Some(gas_limit) = tx.gas_limit {
            tx_request = tx_request.with_gas_limit(gas_limit);
        }

        // Send transaction - provider will fill nonce, gas, chain_id and sign
        let pending_tx = self
            .provider
            .send_transaction(tx_request)
            .await
            .context("Failed to send transaction")?;

        Ok(*pending_tx.tx_hash())
    }

    async fn wait_for_receipt(&self, tx_hash: TxHash) -> Result<TransactionReceipt> {
        // Poll for receipt with timeout
        let max_attempts = 60; // 60 attempts * 2 seconds = 2 minutes timeout
        let poll_interval = std::time::Duration::from_secs(2);

        for _ in 0..max_attempts {
            let receipt: Option<TransactionReceipt> = self
                .provider
                .get_transaction_receipt(tx_hash)
                .await
                .context("Failed to get transaction receipt")?;

            if let Some(receipt) = receipt {
                return Ok(receipt);
            }

            tokio::time::sleep(poll_interval).await;
        }

        eyre::bail!("Transaction receipt not found after timeout: {}", tx_hash)
    }

    async fn get_balance(&self) -> Result<U256> {
        let balance: U256 = self
            .provider
            .get_balance(self.address)
            .await
            .context("Failed to get balance")?;

        Ok(balance)
    }
}

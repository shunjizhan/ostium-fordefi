//! Fordefi MPC wallet signer implementation (Phase 2)
//!
//! This signer uses Fordefi's API to sign and submit transactions via their MPC wallet.

use super::{TransactionSigner, TxRequest};
use alloy::primitives::{Address, TxHash, U256};
use alloy::rpc::types::TransactionReceipt;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use eyre::{Context, Result};
use p256::ecdsa::{signature::Signer, SigningKey};
use p256::pkcs8::DecodePrivateKey;
use reqwest::Client;
use sec1::DecodeEcPrivateKey;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const FORDEFI_API_BASE: &str = "https://api.fordefi.com/api/v1";
const ARBITRUM_CHAIN_NAME: &str = "arbitrum_mainnet";

/// Fordefi MPC wallet signer
///
/// This implementation uses Fordefi's REST API to create and sign transactions
/// via their MPC infrastructure.
pub struct FordefiSigner {
    /// Vault ID for the EVM wallet
    vault_id: String,
    /// JWT access token for API authentication
    access_token: String,
    /// P-256 signing key for request authentication
    signing_key: SigningKey,
    /// HTTP client
    client: Client,
    /// Wallet address
    address: Address,
    /// RPC URL for reading receipts
    rpc_url: String,
}

// ========== API Request/Response Types ==========

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct CreateTransactionRequest {
    #[serde(rename = "type")]
    tx_type: String,
    vault_id: String,
    signer_type: String,
    details: EvmTransactionDetails,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct EvmTransactionDetails {
    #[serde(rename = "type")]
    detail_type: String,
    chain: String,
    to: String,
    value: String,
    data: HexData,
    gas: GasConfig,
    push_mode: String,
    skip_prediction: bool,
}

// Fordefi expects just the chain name as a string, not an object

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct HexData {
    #[serde(rename = "type")]
    data_type: String,
    hex_data: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct GasConfig {
    #[serde(rename = "type")]
    gas_type: String,
    priority_level: String,
}

#[derive(Debug, Deserialize)]
struct CreateTransactionResponse {
    id: String,
}

#[derive(Debug, Deserialize)]
struct TransactionStatusResponse {
    #[allow(dead_code)]
    id: String,
    state: String,
    #[serde(default)]
    hash: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VaultsResponse {
    vaults: Vec<VaultInfo>,
}

#[derive(Debug, Deserialize)]
struct VaultInfo {
    id: String,
    #[serde(default)]
    address: Option<String>,
}

impl FordefiSigner {
    /// Create a new FordefiSigner with a specific address
    ///
    /// # Arguments
    ///
    /// * `access_token` - JWT access token from Fordefi
    /// * `private_key_pem` - P-256 private key in PEM format for request signing
    /// * `address` - The EVM wallet address to use
    /// * `rpc_url` - RPC URL for reading transaction receipts
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let signer = FordefiSigner::new(
    ///     "eyJ...",
    ///     "-----BEGIN EC PRIVATE KEY-----\n...\n-----END EC PRIVATE KEY-----",
    ///     "0x1234...".parse()?,
    ///     "https://arb-mainnet.g.alchemy.com/v2/...",
    /// ).await?;
    /// ```
    pub async fn new(
        access_token: impl Into<String>,
        private_key_pem: impl AsRef<str>,
        address: Address,
        rpc_url: impl Into<String>,
    ) -> Result<Self> {
        let access_token = access_token.into();
        let rpc_url = rpc_url.into();

        // Parse the P-256 private key from PEM
        let signing_key = parse_pem_private_key(private_key_pem.as_ref())
            .context("Failed to parse Fordefi private key")?;

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        // Get vault ID for this address
        let vault_id = Self::fetch_vault_id(&client, &access_token, address).await?;

        Ok(Self {
            vault_id,
            access_token,
            signing_key,
            client,
            address,
            rpc_url,
        })
    }

    /// Create a new FordefiSigner, auto-discovering the first EVM vault
    ///
    /// This method will fetch all EVM vaults from Fordefi and use the first one.
    /// Useful when you only have one EVM wallet in your Fordefi account.
    ///
    /// # Arguments
    ///
    /// * `access_token` - JWT access token from Fordefi
    /// * `private_key_pem` - P-256 private key in PEM format for request signing
    /// * `rpc_url` - RPC URL for reading transaction receipts
    pub async fn discover(
        access_token: impl Into<String>,
        private_key_pem: impl AsRef<str>,
        rpc_url: impl Into<String>,
    ) -> Result<Self> {
        let access_token = access_token.into();
        let rpc_url = rpc_url.into();

        // Parse the P-256 private key from PEM
        let signing_key = parse_pem_private_key(private_key_pem.as_ref())
            .context("Failed to parse Fordefi private key")?;

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        // Discover vault and address
        let (vault_id, address) = Self::discover_vault(&client, &access_token).await?;

        Ok(Self {
            vault_id,
            access_token,
            signing_key,
            client,
            address,
            rpc_url,
        })
    }

    /// Discover the first EVM vault and its address
    async fn discover_vault(client: &Client, access_token: &str) -> Result<(String, Address)> {
        let url = format!("{}/vaults?vault_types=evm", FORDEFI_API_BASE);

        let resp = client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .context("Failed to fetch vaults")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            eyre::bail!("Failed to fetch vaults: {} - {}", status, body);
        }

        let vaults: VaultsResponse = resp.json().await.context("Failed to parse vaults response")?;

        // Use the first vault with an address
        for vault in &vaults.vaults {
            if let Some(addr_str) = &vault.address {
                let address: Address = addr_str.parse().context("Invalid vault address")?;
                tracing::info!("Discovered Fordefi vault: {} at {}", vault.id, address);
                return Ok((vault.id.clone(), address));
            }
        }

        eyre::bail!("No EVM vault found in Fordefi account")
    }

    /// Fetch vault ID for an address
    async fn fetch_vault_id(
        client: &Client,
        access_token: &str,
        address: Address,
    ) -> Result<String> {
        let url = format!(
            "{}/vaults?vault_types=evm&search={}",
            FORDEFI_API_BASE, address
        );

        let resp = client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .context("Failed to fetch vaults")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            eyre::bail!("Failed to fetch vaults: {} - {}", status, body);
        }

        let vaults: VaultsResponse =
            resp.json().await.context("Failed to parse vaults response")?;

        // Find vault matching the address
        let address_str = format!("{:?}", address).to_lowercase();
        for vault in &vaults.vaults {
            if let Some(vault_addr) = &vault.address {
                if vault_addr.to_lowercase() == address_str {
                    return Ok(vault.id.clone());
                }
            }
        }

        eyre::bail!("No vault found for address {}", address)
    }

    /// Sign the API request body for POST /api/v1/transactions
    fn sign_request_body(&self, body: &str) -> Result<(String, String)> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("System time error")?
            .as_millis()
            .to_string();

        // Format: /api/v1/transactions|{timestamp}|{body}
        let payload = format!("/api/v1/transactions|{}|{}", timestamp, body);

        // Sign with ECDSA SHA-256
        let signature: p256::ecdsa::Signature = self.signing_key.sign(payload.as_bytes());
        let sig_der = signature.to_der();
        let sig_base64 = BASE64.encode(sig_der.as_bytes());

        Ok((timestamp, sig_base64))
    }

    /// Create a transaction via Fordefi API
    async fn create_transaction(&self, tx: &TxRequest) -> Result<String> {
        let request = CreateTransactionRequest {
            tx_type: "evm_transaction".to_string(),
            vault_id: self.vault_id.clone(),
            signer_type: "api_signer".to_string(),
            details: EvmTransactionDetails {
                detail_type: "evm_raw_transaction".to_string(),
                chain: ARBITRUM_CHAIN_NAME.to_string(),
                to: format!("{:?}", tx.to),
                value: tx.value.to_string(),
                data: HexData {
                    data_type: "hex".to_string(),
                    hex_data: format!("0x{}", hex::encode(&tx.data)),
                },
                gas: GasConfig {
                    gas_type: "priority".to_string(),
                    priority_level: "medium".to_string(),
                },
                push_mode: "auto".to_string(),
                skip_prediction: true,
            },
        };

        let body = serde_json::to_string(&request).context("Failed to serialize request")?;
        let (timestamp, signature) = self.sign_request_body(&body)?;

        let url = format!("{}/transactions", FORDEFI_API_BASE);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.access_token)
            .header("X-Timestamp", &timestamp)
            .header("X-Signature", &signature)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .context("Failed to create transaction")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            eyre::bail!("Failed to create transaction: {} - {}", status, body);
        }

        let result: CreateTransactionResponse = resp
            .json()
            .await
            .context("Failed to parse transaction response")?;

        Ok(result.id)
    }

    /// Poll transaction status until it's signed and pushed
    async fn poll_transaction_status(&self, tx_id: &str) -> Result<TxHash> {
        let url = format!("{}/transactions/{}", FORDEFI_API_BASE, tx_id);
        let poll_interval = Duration::from_secs(2);
        let max_attempts = 90; // 3 minutes timeout

        for attempt in 0..max_attempts {
            let resp = self
                .client
                .get(&url)
                .bearer_auth(&self.access_token)
                .send()
                .await
                .context("Failed to get transaction status")?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                eyre::bail!("Failed to get transaction status: {} - {}", status, body);
            }

            let status: TransactionStatusResponse = resp
                .json()
                .await
                .context("Failed to parse transaction status")?;

            tracing::debug!(
                "Fordefi tx {} state: {} (attempt {}/{})",
                tx_id,
                status.state,
                attempt + 1,
                max_attempts
            );

            match status.state.as_str() {
                // Success states - transaction has been pushed to blockchain
                "mined" | "completed" | "pushed_to_blockchain" | "signed" => {
                    if let Some(hash) = status.hash {
                        let hash = hash.strip_prefix("0x").unwrap_or(&hash);
                        let bytes: [u8; 32] = hex::decode(hash)
                            .context("Invalid tx hash hex")?
                            .try_into()
                            .map_err(|_| eyre::eyre!("Invalid tx hash length"))?;
                        return Ok(TxHash::from(bytes));
                    }
                    // If signed but no hash yet, keep polling
                    if status.state == "signed" {
                        tokio::time::sleep(poll_interval).await;
                        continue;
                    }
                    eyre::bail!("Transaction completed but no hash returned");
                }

                // Error states
                "error_signing" | "error_pushing_to_blockchain" => {
                    eyre::bail!("Transaction failed: {}", status.state);
                }
                "aborted" | "cancelled" => {
                    eyre::bail!("Transaction was {}", status.state);
                }

                // Pending states - keep polling
                "waiting_for_approval" | "approved" | "queued" | "stuck" => {
                    tokio::time::sleep(poll_interval).await;
                }

                // Unknown state
                other => {
                    tracing::warn!("Unknown transaction state: {}", other);
                    tokio::time::sleep(poll_interval).await;
                }
            }
        }

        eyre::bail!("Transaction polling timed out after {} attempts", max_attempts)
    }
}

impl TransactionSigner for FordefiSigner {
    fn address(&self) -> Address {
        self.address
    }

    async fn sign_and_send(&self, tx: TxRequest) -> Result<TxHash> {
        // Create transaction via Fordefi API
        let tx_id = self.create_transaction(&tx).await?;
        tracing::info!("Created Fordefi transaction: {}", tx_id);

        // Poll until we get the transaction hash
        self.poll_transaction_status(&tx_id).await
    }

    async fn wait_for_receipt(&self, tx_hash: TxHash) -> Result<TransactionReceipt> {
        use alloy::providers::{Provider, ProviderBuilder};
        use alloy::transports::http::reqwest::Url;

        let url: Url = self.rpc_url.parse().context("Invalid RPC URL")?;
        let provider = ProviderBuilder::new()
            .disable_recommended_fillers()
            .connect_http(url);

        // Poll for receipt
        let max_attempts = 60;
        let poll_interval = Duration::from_secs(2);

        for _ in 0..max_attempts {
            let receipt: Option<TransactionReceipt> = provider
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
        use alloy::providers::{Provider, ProviderBuilder};
        use alloy::transports::http::reqwest::Url;

        let url: Url = self.rpc_url.parse().context("Invalid RPC URL")?;
        let provider = ProviderBuilder::new()
            .disable_recommended_fillers()
            .connect_http(url);

        let balance: U256 = provider
            .get_balance(self.address)
            .await
            .context("Failed to get balance")?;

        Ok(balance)
    }
}

/// Parse a P-256 private key from PEM format
fn parse_pem_private_key(pem: &str) -> Result<SigningKey> {
    // Normalize PEM format - ensure proper line breaks
    let normalized = normalize_pem(pem);

    // Try parsing as PKCS#8 first, then SEC1
    if let Ok(key) = SigningKey::from_pkcs8_pem(&normalized) {
        return Ok(key);
    }

    if let Ok(key) = SigningKey::from_sec1_pem(&normalized) {
        return Ok(key);
    }

    eyre::bail!("Failed to parse private key - not a valid P-256 key in PEM format")
}

/// Normalize PEM format by ensuring proper headers and line breaks
fn normalize_pem(pem: &str) -> String {
    // First, replace escaped newlines with actual newlines
    let pem = pem.replace("\\n", "\n").replace("\\r", "");

    // Trim each line to remove trailing whitespace
    let lines: Vec<&str> = pem.lines().map(|l| l.trim()).collect();
    let pem = lines.join("\n");
    let pem = pem.trim();

    // If it already has proper headers, return as-is
    if pem.contains("-----BEGIN") && pem.contains("-----END") {
        return pem.to_string();
    }

    // Otherwise, try to wrap it in EC PRIVATE KEY headers
    let base64_content = pem
        .replace("-----BEGIN EC PRIVATE KEY-----", "")
        .replace("-----END EC PRIVATE KEY-----", "")
        .replace("-----BEGIN PRIVATE KEY-----", "")
        .replace("-----END PRIVATE KEY-----", "")
        .replace('\n', "")
        .replace('\r', "")
        .replace(' ', "");

    format!(
        "-----BEGIN EC PRIVATE KEY-----\n{}\n-----END EC PRIVATE KEY-----",
        base64_content
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_pem() {
        let raw = "MHQCAQEEIGsomething...base64...oAcGBSuBBAAKoUQDQgAE...";
        let normalized = normalize_pem(raw);
        assert!(normalized.contains("-----BEGIN EC PRIVATE KEY-----"));
        assert!(normalized.contains("-----END EC PRIVATE KEY-----"));
    }
}

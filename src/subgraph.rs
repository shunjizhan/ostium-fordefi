//! Subgraph client for querying Ostium positions

use eyre::{Context, Result};
use serde::{Deserialize, Serialize};

/// Open trade position from subgraph
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenTrade {
    pub trade_id: Option<String>,
    pub collateral: String,
    pub leverage: String,
    pub open_price: String,
    pub stop_loss_price: String,
    pub take_profit_price: String,
    pub is_open: bool,
    pub timestamp: String,
    pub is_buy: bool,
    pub index: String,
    pub pair: TradePair,
}

/// Trading pair info
#[derive(Debug, Clone, Deserialize)]
pub struct TradePair {
    pub id: String,
    pub from: String,
    pub to: String,
}

impl OpenTrade {
    /// Get collateral as f64 (from 6 decimals)
    pub fn collateral_f64(&self) -> f64 {
        self.collateral.parse::<f64>().unwrap_or(0.0) / 1e6
    }

    /// Get leverage as f64 (from 2 decimals)
    pub fn leverage_f64(&self) -> f64 {
        self.leverage.parse::<f64>().unwrap_or(0.0) / 100.0
    }

    /// Get open price as f64 (from 18 decimals)
    pub fn open_price_f64(&self) -> f64 {
        self.open_price.parse::<f64>().unwrap_or(0.0) / 1e18
    }

    /// Get position size (collateral * leverage)
    pub fn position_size(&self) -> f64 {
        self.collateral_f64() * self.leverage_f64()
    }

    /// Get pair index
    pub fn pair_index(&self) -> u16 {
        self.pair.id.parse().unwrap_or(0)
    }

    /// Get trade index
    pub fn trade_index(&self) -> u8 {
        self.index.parse().unwrap_or(0)
    }

    /// Get direction as string
    pub fn direction(&self) -> &str {
        if self.is_buy { "LONG" } else { "SHORT" }
    }

    /// Get pair name (e.g., "BTC/USD")
    pub fn pair_name(&self) -> String {
        format!("{}/{}", self.pair.from, self.pair.to)
    }
}

#[derive(Serialize)]
struct GraphQLRequest {
    query: String,
    variables: serde_json::Value,
}

#[derive(Deserialize)]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Deserialize, Debug)]
struct GraphQLError {
    message: String,
}

#[derive(Deserialize)]
struct TradesData {
    trades: Vec<OpenTrade>,
}

/// Subgraph client for querying positions
pub struct SubgraphClient {
    url: String,
    client: reqwest::Client,
}

impl SubgraphClient {
    /// Create a new subgraph client
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
                .timeout(std::time::Duration::from_secs(30))
                .connect_timeout(std::time::Duration::from_secs(10))
                .use_rustls_tls()
                .build()
                .unwrap(),
        }
    }

    /// Get all open trades for an address
    pub async fn get_open_trades(&self, address: &str) -> Result<Vec<OpenTrade>> {
        let query = r#"
            query trades($trader: Bytes!) {
                trades(where: { isOpen: true, trader: $trader }) {
                    tradeID
                    collateral
                    leverage
                    openPrice
                    stopLossPrice
                    takeProfitPrice
                    isOpen
                    timestamp
                    isBuy
                    index
                    pair {
                        id
                        from
                        to
                    }
                }
            }
        "#;

        let request = GraphQLRequest {
            query: query.to_string(),
            variables: serde_json::json!({
                "trader": address.to_lowercase()
            }),
        };

        let response = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to query subgraph")?;

        let result: GraphQLResponse<TradesData> = response
            .json()
            .await
            .context("Failed to parse subgraph response")?;

        if let Some(errors) = result.errors {
            let error_msgs: Vec<_> = errors.iter().map(|e| e.message.clone()).collect();
            eyre::bail!("Subgraph errors: {:?}", error_msgs);
        }

        Ok(result.data.map(|d| d.trades).unwrap_or_default())
    }
}

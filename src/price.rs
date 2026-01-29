//! Price fetching from Ostium metadata backend

use eyre::{Context, Result};
use serde::Deserialize;

const OSTIUM_PRICE_API: &str = "https://metadata-backend.ostium.io/PricePublish/latest-prices";

/// Price data from Ostium API
#[derive(Debug, Deserialize)]
pub struct PriceData {
    pub from: String,
    pub to: String,
    pub bid: f64,
    pub mid: f64,
    pub ask: f64,
    #[serde(rename = "isMarketOpen")]
    pub is_market_open: bool,
    #[serde(rename = "isDayTradingClosed")]
    pub is_day_trading_closed: bool,
}

/// Fetch the current price for a trading pair
pub async fn get_price(from: &str, to: &str) -> Result<f64> {
    let client = reqwest::Client::builder()
        .user_agent("OstiumRustSDK/0.1.0")
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(OSTIUM_PRICE_API)
        .send()
        .await
        .context("Failed to fetch prices")?;

    let text = response.text().await.context("Failed to read response body")?;

    let prices: Vec<PriceData> = serde_json::from_str(&text)
        .with_context(|| format!("Failed to parse price response: {}", &text[..text.len().min(200)]))?;

    for price in prices {
        if price.from == from && price.to == to {
            return Ok(price.mid);
        }
    }

    eyre::bail!("No price found for {}/{}", from, to)
}

/// Get BTC/USD price
pub async fn get_btc_price() -> Result<f64> {
    get_price("BTC", "USD").await
}

/// Get ETH/USD price
pub async fn get_eth_price() -> Result<f64> {
    get_price("ETH", "USD").await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_btc_price() {
        let price = get_btc_price().await.unwrap();
        assert!(price > 0.0);
        println!("BTC price: ${:.2}", price);
    }
}

# Ostium Rust SDK

Rust SDK for interacting with the Ostium perpetuals protocol on Arbitrum One.

## Features

- **Trading**: Open/close BTC long positions with configurable leverage
- **OLP Vault**: Deposit USDC and withdraw from the OLP vault (ERC-4626)
- **Position Management**: View and close open positions
- **Dual Signer Support**: Local private key or Fordefi MPC wallet

## Prerequisites

- Rust 1.70+
- An Alchemy API key (for Arbitrum RPC)
- A wallet with:
  - Some ETH on Arbitrum for gas
  - USDC for trading/depositing

## Setup

1. Clone the repository:
```bash
git clone https://github.com/hypersignals/ostium-fordefi
cd ostium-fordefi
```

2. Copy the environment example and fill in your values:
```bash
cp .env.example .env
```

3. Configure your signer (choose one):

### Option 1: Local Private Key
```
ALCHEMY_API_KEY=your_alchemy_api_key
PRIVATE_KEY=your_private_key_without_0x_prefix
```

### Option 2: Fordefi MPC Wallet
```
ALCHEMY_API_KEY=your_alchemy_api_key
FORDEFI_JWT_TOKEN=your_fordefi_jwt_token
```

Then place your Fordefi API user's P-256 private key in `keys/pk.pem`:
```
-----BEGIN EC PRIVATE KEY-----
MHcCAQEEI...
-----END EC PRIVATE KEY-----
```

Optional environment variables for Fordefi:
- `FORDEFI_PRIVATE_KEY_PATH` - Custom path to PEM file (default: `keys/pk.pem`)
- `FORDEFI_ADDRESS` - Wallet address (auto-discovered if not set)

## Running the Example Flow

The `flow` example provides an interactive CLI for all SDK operations:

```bash
cargo run --example flow
```

The SDK automatically detects which signer to use based on environment variables:
- If `FORDEFI_JWT_TOKEN` is set → Fordefi MPC mode
- Otherwise → Local signer mode

### Menu Options

1. **Long BTC** - Open a $20 BTC long position ($2 collateral, 10x leverage)
2. **Close position** - Close an existing position by index
3. **Deposit to OLP vault** - Deposit USDC to earn yield (default: 0.02 USDC)
4. **Withdraw from OLP vault** - Request withdrawal from OLP (default: 0.01 OLP shares)
5. **View info** - Display balances, positions, and pending withdrawals

### OLP Vault Withdrawal Process

The OLP vault uses an epoch-based withdrawal system:

1. **Request Window**: Withdrawal requests can only be made in the first 48 hours of any epoch
2. **Cooling-Off Period**: 1-3 epochs depending on vault collateralization:
   - Above 120%: 1 epoch (3 days)
   - 110-120%: 2 epochs (6 days)
   - Below 110%: 3 epochs (9 days)
3. **Redemption Window**: After cooling-off, you have 48 hours to complete the withdrawal
4. If the window is missed, the request cancels and must be resubmitted

## Contract Addresses (Arbitrum One)

| Contract | Address |
|----------|---------|
| USDC | `0xaf88d065e77c8cC2239327C5EDb3A432268e5831` |
| Trading | `0x6D0bA1f9996DBD8885827e1b2e8f6593e7702411` |
| TradingStorage | `0xcCd5891083A8acD2074690F65d3024E7D13d66E7` |
| OLP Vault | `0x20d419a8e12c45f88fda7c5760bb6923cee27f98` |

## SDK Usage

### With Local Signer

```rust
use ostium_sdk::{OstiumClient, NetworkConfig, LocalSigner, PlaceOrderParams, get_btc_price};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    let config = NetworkConfig::default();
    let private_key = std::env::var("PRIVATE_KEY")?;
    let signer = LocalSigner::from_private_key(&private_key, &config.rpc_url).await?;
    let client = OstiumClient::new(signer, config).await?;

    // Check balances
    let usdc = client.get_usdc_balance().await?;
    let olp = client.get_olp_balance().await?;

    // Place a BTC long trade
    let btc_price = get_btc_price().await?;
    let params = PlaceOrderParams::market(0, 2.0, 10.0, true) // $2 collateral, 10x, long
        .with_open_price(btc_price)
        .with_slippage(2.0);
    let tx_hash = client.place_order(params, None).await?;

    // Get open positions
    let positions = client.get_positions(None).await?;

    Ok(())
}
```

### With Fordefi MPC Signer

```rust
use ostium_sdk::{OstiumClient, NetworkConfig, FordefiSigner, PlaceOrderParams, get_btc_price};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    let config = NetworkConfig::default();
    let jwt_token = std::env::var("FORDEFI_JWT_TOKEN")?;
    let private_key_pem = std::fs::read_to_string("keys/pk.pem")?;

    // Auto-discover vault address from Fordefi
    let signer = FordefiSigner::discover(&jwt_token, &private_key_pem, &config.rpc_url).await?;
    let client = OstiumClient::new(signer, config).await?;

    // Place a BTC long trade via Fordefi MPC
    let btc_price = get_btc_price().await?;
    let params = PlaceOrderParams::market(0, 2.0, 10.0, true)
        .with_open_price(btc_price)
        .with_slippage(2.0);
    let tx_hash = client.place_order(params, None).await?;

    println!("Trade placed: {}", tx_hash);
    Ok(())
}
```

## License

MIT

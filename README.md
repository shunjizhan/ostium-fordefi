# Ostium Rust SDK

Rust SDK for interacting with the [Ostium](https://ostium.io) perpetuals protocol on Arbitrum One, using [Fordefi](https://fordefi.com) MPC wallet for secure institutional-grade transaction signing.

## Features

- **Trading**: Open/close BTC perpetual positions with configurable leverage (up to 100x)
- **OLP Vault**: Deposit USDC to earn yield, request withdrawals, approve auto-withdraw
- **Fordefi MPC**: Secure transaction signing via Fordefi's MPC infrastructure
- **Real-time Prices**: Fetch live BTC/ETH prices from Ostium's price feed

## Prerequisites

- Rust 1.70+
- [Alchemy](https://www.alchemy.com/) API key (for Arbitrum RPC)
- [Fordefi](https://fordefi.com) account with:
  - API User configured with P-256 key pair
  - EVM vault on Arbitrum One with ETH for gas and USDC for trading

## Setup

### 1. Clone and configure

```bash
git clone https://github.com/hypersignals/ostium-fordefi
cd ostium-fordefi
cp .env.example .env
```

### 2. Configure environment variables

Edit `.env` with your credentials:

```bash
# Alchemy API key for Arbitrum One RPC
ALCHEMY_API_KEY=your_alchemy_api_key

# Fordefi API JWT token (from Fordefi dashboard)
FORDEFI_JWT_TOKEN=your_jwt_token

# Optional: Wallet address (auto-discovered if not set)
# FORDEFI_ADDRESS=0x...
```

### 3. Set up Fordefi API signing key

Place your Fordefi API User's P-256 private key in `keys/pk.pem`:

```
-----BEGIN EC PRIVATE KEY-----
MHcCAQEEI...your_key_here...
-----END EC PRIVATE KEY-----
```

## Running the Interactive CLI

```bash
cargo run --example flow
```

### Menu Options

| Option | Description |
|--------|-------------|
| **1. Long BTC** | Open a $20 BTC long position ($2 collateral, 10x leverage) |
| **2. Close position** | Close an existing position by trade index |
| **3. Deposit to OLP vault** | Deposit USDC to OLP vault (default: 0.02 USDC) |
| **4. Withdraw (auto)** | Approve OLP tokens for auto-withdraw contract |
| **5. Withdraw (manual)** | Request manual withdrawal from OLP vault |
| **6. View info** | Display balances, positions, and pending withdrawals |
| **q. Quit** | Exit the CLI |

## OLP Vault Withdrawal System

The OLP vault uses an epoch-based withdrawal system (each epoch = 3 days):

### Withdrawal Methods

**Auto-Withdraw (Option 4)**
- Approve your OLP tokens to the auto-withdraw contract
- The protocol automatically processes your withdrawal when eligible
- Simpler but requires trusting the auto-withdraw contract

**Manual Withdraw (Option 5)**
- Request withdrawal during the first 48 hours of an epoch
- Wait for cooling-off period (1-3 epochs based on vault collateralization)
- Complete redemption within 48-hour window after cooling-off
- If window is missed, request must be resubmitted

### Cooling-Off Periods

| Vault Collateralization | Cooling-Off Period |
|------------------------|-------------------|
| Above 120% | 1 epoch (3 days) |
| 110% - 120% | 2 epochs (6 days) |
| Below 110% | 3 epochs (9 days) |

## SDK Usage

```rust
use ostium_sdk::{
    OstiumClient, NetworkConfig, FordefiSigner,
    PlaceOrderParams, DepositParams, CloseTradeParams,
    get_btc_price,
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    // Initialize
    let config = NetworkConfig::default();
    let jwt_token = std::env::var("FORDEFI_JWT_TOKEN")?;
    let private_key_pem = std::fs::read_to_string("keys/pk.pem")?;

    // Auto-discover wallet from Fordefi
    let signer = FordefiSigner::discover(&jwt_token, &private_key_pem, &config.rpc_url).await?;
    let client = OstiumClient::new(signer, config).await?;

    // Check balances
    let usdc_balance = client.get_usdc_balance().await?;
    let olp_position = client.get_olp_balance().await?;
    println!("USDC: ${:.2}", usdc_balance);
    println!("OLP: {:.6} shares (${:.2})", olp_position.shares_f64(), olp_position.value);

    // Place a BTC long trade
    let btc_price = get_btc_price().await?;
    let params = PlaceOrderParams::market(0, 2.0, 10.0, true) // pair 0 = BTC, $2 collateral, 10x, long
        .with_open_price(btc_price)
        .with_slippage(2.0);
    let tx_hash = client.place_order(params, None).await?;
    println!("Trade tx: {}", tx_hash);

    // Get open positions
    let positions = client.get_positions(None).await?;
    for pos in &positions {
        println!(
            "Position: {} {} {:.1}x @ ${:.2}",
            if pos.is_long { "LONG" } else { "SHORT" },
            match pos.pair_index { 0 => "BTC", 1 => "ETH", _ => "?" },
            pos.leverage,
            pos.open_price
        );
    }

    // Close a position
    if let Some(pos) = positions.first() {
        let close_params = CloseTradeParams::close_all(pos.pair_index, pos.trade_index, btc_price);
        let tx_hash = client.close_trade(close_params).await?;
        println!("Close tx: {}", tx_hash);
    }

    // Deposit to OLP vault
    let deposit_params = DepositParams::new(10.0); // 10 USDC
    let tx_hash = client.deposit_olp(deposit_params).await?;
    println!("Deposit tx: {}", tx_hash);

    Ok(())
}
```

## API Reference

### OstiumClient Methods

| Method | Description |
|--------|-------------|
| `get_usdc_balance()` | Get USDC balance |
| `get_eth_balance()` | Get ETH balance (for gas) |
| `get_olp_balance()` | Get OLP vault position (shares + value) |
| `get_positions(pair_index)` | Get open trading positions |
| `get_vault_epoch()` | Get current vault epoch info |
| `get_pending_withdrawal(epoch)` | Get pending withdrawal for epoch |
| `get_auto_withdraw_allowance()` | Get OLP allowance for auto-withdraw |
| `place_order(params, trade_index)` | Open a new trade |
| `close_trade(params)` | Close an existing trade |
| `deposit_olp(params)` | Deposit USDC to OLP vault |
| `request_olp_withdrawal(shares)` | Request manual withdrawal |
| `approve_auto_withdraw(shares)` | Approve OLP for auto-withdraw |

### Trading Pairs

| Index | Pair |
|-------|------|
| 0 | BTC/USD |
| 1 | ETH/USD |

## Contract Addresses (Arbitrum One)

| Contract | Address |
|----------|---------|
| USDC | `0xaf88d065e77c8cC2239327C5EDb3A432268e5831` |
| Trading | `0x6D0bA1f9996DBD8885827e1b2e8f6593e7702411` |
| TradingStorage | `0xcCd5891083A8acD2074690F65d3024E7D13d66E7` |
| OLP Vault | `0x20d419a8e12c45f88fda7c5760bb6923cee27f98` |
| Auto-Withdraw | `0x6297ce1a61c2c8a72bfb0de957f6b1cf0413141e` |

## Project Structure

```
ostium-fordefi/
├── src/
│   ├── lib.rs              # Public exports
│   ├── client.rs           # OstiumClient - main entry point
│   ├── config.rs           # Network configuration
│   ├── constants.rs        # Precision levels, limits
│   ├── error.rs            # Error types
│   ├── price.rs            # Price feed utilities
│   ├── signer/
│   │   ├── mod.rs          # TransactionSigner trait
│   │   └── fordefi.rs      # Fordefi MPC signer
│   ├── contracts/
│   │   ├── trading.rs      # Trading contract bindings
│   │   ├── vault.rs        # OLP Vault bindings
│   │   └── usdc.rs         # USDC token bindings
│   └── types/
│       ├── trade.rs        # Trade types (PlaceOrderParams, etc.)
│       └── vault.rs        # Vault types (DepositParams, etc.)
├── examples/
│   └── flow.rs             # Interactive CLI example
├── keys/
│   └── pk.pem              # Fordefi P-256 private key (gitignored)
└── .env                    # Environment variables (gitignored)
```

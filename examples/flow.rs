//! Interactive CLI for Ostium SDK
//!
//! Run with: cargo run --example interactive
//!
//! Requires PRIVATE_KEY environment variable

use std::io::{self, Write};

use ostium_sdk::{
    get_btc_price, get_eth_price, CloseTradeParams, DepositParams, LocalSigner, NetworkConfig,
    OstiumClient, PlaceOrderParams, Position,
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Load environment
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    // Get private key from env
    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY must be set");

    // Initialize client
    let config = NetworkConfig::default();
    let signer = LocalSigner::from_private_key(&private_key, &config.rpc_url).await?;
    let client = OstiumClient::new(signer, config.clone()).await?;

    println!("\n========================================");
    println!("       Ostium SDK Interactive CLI");
    println!("========================================");
    println!("Connected wallet: {}", client.address());

    // Main loop
    loop {
        println!("\n----------------------------------------");
        println!("Select an option:");
        println!("  1. Long BTC");
        println!("  2. Close position");
        println!("  3. Deposit to OLP vault");
        println!("  4. Withdraw from OLP vault");
        println!("  5. View info");
        println!("  q. Quit");
        println!("----------------------------------------");

        print!("Enter choice: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim();

        match choice {
            "1" => long_btc_flow(&client).await?,
            "2" => close_position_flow(&client).await?,
            "3" => deposit_olp_flow(&client).await?,
            "4" => withdraw_olp_flow(&client).await?,
            "5" => view_info(&client).await?,
            "q" | "Q" => {
                println!("\nGoodbye!");
                break;
            }
            _ => println!("\nInvalid choice. Please try again."),
        }
    }

    Ok(())
}

/// Long BTC with $2 collateral, 10x leverage
async fn long_btc_flow<S: ostium_sdk::TransactionSigner>(
    client: &OstiumClient<S>,
) -> eyre::Result<()> {
    println!("\n=== LONG BTC ===");

    // Fetch positions and BTC price in parallel
    let (positions_before, current_price) = tokio::join!(
        client.get_positions(None),
        get_btc_price()
    );
    let positions_before = positions_before?;
    let current_price = current_price?;

    println!("Positions BEFORE: {}", positions_before.len());
    if !positions_before.is_empty() {
        print_positions(&positions_before);
    }

    println!("\nCurrent BTC price: ${:.2}", current_price);

    // $2 collateral, 10x leverage = $20 position
    let collateral = 2.0;
    let leverage = 10.0;

    println!("Placing LONG ${:.0} position...", collateral * leverage);

    let params = PlaceOrderParams::market(0, collateral, leverage, true) // pair_index 0 = BTC
        .with_open_price(current_price)
        .with_slippage(2.0);

    let tx_hash = client.place_order(params, None).await?;
    println!("Transaction: {}", tx_hash);

    let receipt = client.wait_for_receipt(tx_hash).await?;
    if receipt.status() {
        println!("LONG trade placed successfully!");

        // Show positions after
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        let positions_after = client.get_positions(None).await?;
        println!("\nPositions AFTER: {}", positions_after.len());
        if !positions_after.is_empty() {
            print_positions(&positions_after);
        }
    } else {
        println!("Trade transaction reverted!");
    }

    Ok(())
}

/// View all info (positions + balances)
async fn view_info<S: ostium_sdk::TransactionSigner>(
    client: &OstiumClient<S>,
) -> eyre::Result<()> {
    println!("\n=== Account Info ===");

    // Fetch all data in parallel
    let (usdc_result, eth_result, olp_result, epoch_result, positions_result) = tokio::join!(
        client.get_usdc_balance(),
        client.get_eth_balance(),
        client.get_olp_balance(),
        client.get_vault_epoch(),
        client.get_positions(None)
    );

    // Display USDC balance
    if let Ok(usdc_balance) = usdc_result {
        println!("USDC Balance: ${:.2}", usdc_balance);
    }

    // Display ETH balance
    if let Ok(eth_balance) = eth_result {
        let eth_f64 = eth_balance.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
        println!("ETH Balance: {:.6} ETH", eth_f64);
    }

    // Display OLP balance
    if let Ok(olp_pos) = olp_result {
        let shares_f64 = olp_pos.shares.to_string().parse::<f64>().unwrap_or(0.0) / 1e6;
        println!("OLP Shares: {:.6} (${:.2})", shares_f64, olp_pos.value);
    }

    // Fetch and display pending withdrawals in parallel
    if client.config().vault.is_some() {
        if let Ok(epoch_info) = epoch_result {
            let current = epoch_info.current_epoch as u16;
            let start_epoch = current.saturating_sub(10);

            // Fetch all pending withdrawals in parallel
            let futures: Vec<_> = (start_epoch..=current + 1)
                .map(|epoch| {
                    let client = client;
                    async move { (epoch, client.get_pending_withdrawal(epoch).await) }
                })
                .collect();

            let results = futures::future::join_all(futures).await;

            for (epoch, result) in results {
                if let Ok(pending) = result {
                    let pending_f64: f64 = pending.to_string().parse().unwrap_or(0.0) / 1e6;
                    if pending_f64 > 0.0 {
                        println!("Pending Withdrawal (Epoch {}): {:.6} OLP", epoch, pending_f64);
                    }
                }
            }
        }
    }

    // Display positions
    println!("\n--- Open Positions ---");
    if let Ok(positions) = positions_result {
        if positions.is_empty() {
            println!("No open positions.");
        } else {
            print_positions(&positions);
        }
    }

    Ok(())
}

/// Print positions in a formatted table
fn print_positions(positions: &[Position]) {
    println!("\n{:<6} {:<10} {:<6} {:<8} {:<12} {:<12}",
        "Index", "Pair", "Dir", "Lev", "Collateral", "Open Price");
    println!("{}", "-".repeat(60));

    for pos in positions {
        let pair_name = match pos.pair_index {
            0 => "BTC/USD",
            1 => "ETH/USD",
            _ => "Unknown",
        };
        let direction = if pos.is_long { "LONG" } else { "SHORT" };

        println!(
            "{:<6} {:<10} {:<6} {:<8.1}x ${:<11.2} ${:<12.2}",
            pos.trade_index,
            pair_name,
            direction,
            pos.leverage,
            pos.collateral,
            pos.open_price
        );
    }
}

/// Close a position
async fn close_position_flow<S: ostium_sdk::TransactionSigner>(
    client: &OstiumClient<S>,
) -> eyre::Result<()> {
    println!("\n=== Close Position ===");
    println!("Querying positions...");

    let positions = client.get_positions(None).await?;

    if positions.is_empty() {
        println!("No open positions to close.");
        return Ok(());
    }

    print_positions(&positions);

    // Get position to close
    print!("\nEnter trade index to close [0]: ");
    io::stdout().flush()?;
    let mut index_input = String::new();
    io::stdin().read_line(&mut index_input)?;
    let trade_index: u8 = index_input.trim().parse().unwrap_or(0);

    // Find the position
    let position = positions.iter().find(|p| p.trade_index == trade_index);
    let position = match position {
        Some(p) => p,
        None => {
            println!("Position with index {} not found.", trade_index);
            return Ok(());
        }
    };

    // Get current price for the pair
    let market_price = match position.pair_index {
        0 => get_btc_price().await?,
        1 => get_eth_price().await?,
        _ => position.open_price,
    };

    let pair_name = match position.pair_index {
        0 => "BTC/USD",
        1 => "ETH/USD",
        _ => "Unknown",
    };

    println!(
        "\nClosing {} {} position at index {}...",
        pair_name,
        if position.is_long { "LONG" } else { "SHORT" },
        trade_index
    );
    println!("Current price: ${:.2}", market_price);

    let params = CloseTradeParams::close_all(position.pair_index, trade_index, market_price);
    let tx_hash = client.close_trade(params).await?;
    println!("Transaction: {}", tx_hash);

    let receipt = client.wait_for_receipt(tx_hash).await?;
    if receipt.status() {
        println!("Position closed successfully!");

        // Show positions after
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        let positions_after = client.get_positions(None).await?;
        println!("\nPositions AFTER: {}", positions_after.len());
        if !positions_after.is_empty() {
            print_positions(&positions_after);
        } else {
            println!("No open positions.");
        }
    } else {
        println!("Close transaction reverted!");
    }

    Ok(())
}

/// Deposit to OLP vault flow
async fn deposit_olp_flow<S: ostium_sdk::TransactionSigner>(
    client: &OstiumClient<S>,
) -> eyre::Result<()> {
    println!("\n=== Deposit to OLP Vault ===");

    // Check if vault is configured
    if client.config().vault.is_none() {
        println!("OLP Vault is not configured for this network.");
        return Ok(());
    }

    // Fetch OLP and USDC balance in parallel
    let (olp_result, usdc_result) = tokio::join!(
        client.get_olp_balance(),
        client.get_usdc_balance()
    );

    let balance_before = match olp_result {
        Ok(b) => b,
        Err(e) => {
            println!("Error getting OLP balance: {}", e);
            return Ok(());
        }
    };
    let usdc_balance = usdc_result?;

    let shares_before = balance_before.shares.to_string().parse::<f64>().unwrap_or(0.0) / 1e6;
    println!("\nOLP Position BEFORE deposit:");
    println!("  Shares: {:.6}", shares_before);
    println!("  Value: ${:.2}", balance_before.value);
    println!("\nAvailable USDC: ${:.2}", usdc_balance);

    // Get deposit amount
    print!("Amount to deposit (USDC) [0.02]: ");
    io::stdout().flush()?;
    let mut amount_input = String::new();
    io::stdin().read_line(&mut amount_input)?;
    let amount: f64 = amount_input.trim().parse().unwrap_or(0.02);

    if amount > usdc_balance {
        println!("Insufficient USDC balance!");
        return Ok(());
    }

    println!("\nDepositing ${:.2} USDC...", amount);
    let params = DepositParams::new(amount);
    let tx_hash = client.deposit_olp(params).await?;
    println!("Transaction: {}", tx_hash);

    let receipt = client.wait_for_receipt(tx_hash).await?;
    if receipt.status() {
        println!("Deposit successful!");
    } else {
        println!("Deposit transaction reverted!");
        return Ok(());
    }

    // Show balance after
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    let balance_after = client.get_olp_balance().await?;
    let shares_after = balance_after.shares.to_string().parse::<f64>().unwrap_or(0.0) / 1e6;

    println!("\nOLP Position AFTER deposit:");
    println!("  Shares: {:.6} (+{:.6})", shares_after, shares_after - shares_before);
    println!("  Value: ${:.2}", balance_after.value);

    Ok(())
}

/// Withdraw from OLP vault flow (Initialize withdrawal request)
async fn withdraw_olp_flow<S: ostium_sdk::TransactionSigner>(
    client: &OstiumClient<S>,
) -> eyre::Result<()> {
    println!("\n=== Initialize OLP Withdrawal Request ===");

    // Check if vault is configured
    if client.config().vault.is_none() {
        println!("OLP Vault is not configured for this network.");
        return Ok(());
    }

    // Fetch epoch info and OLP balance in parallel
    let (epoch_result, balance_result) = tokio::join!(
        client.get_vault_epoch(),
        client.get_olp_balance()
    );

    let epoch_info = match epoch_result {
        Ok(e) => e,
        Err(e) => {
            println!("Error getting vault epoch: {}", e);
            return Ok(());
        }
    };

    let balance = match balance_result {
        Ok(b) => b,
        Err(e) => {
            println!("Error getting OLP balance: {}", e);
            return Ok(());
        }
    };

    println!("\n--- Vault Epoch Info ---");
    println!("  Current Epoch: {}", epoch_info.current_epoch);
    println!("  Withdrawals Open: {}", if epoch_info.withdrawals_open { "YES" } else { "NO" });

    let shares_f64 = balance.shares_f64();
    println!("\n--- Current OLP Position ---");
    println!("  Shares: {:.6} OLP", shares_f64);
    println!("  Value: ${:.2} USDC", balance.value);

    // Fetch pending withdrawals in parallel
    let current = epoch_info.current_epoch as u16;
    let start_epoch = current.saturating_sub(10);

    let futures: Vec<_> = (start_epoch..=current + 1)
        .map(|epoch| {
            let client = client;
            async move { (epoch, client.get_pending_withdrawal(epoch).await) }
        })
        .collect();

    let results = futures::future::join_all(futures).await;

    println!("\n--- Pending Withdrawals ---");
    let mut found_pending = false;
    for (epoch, result) in &results {
        if let Ok(pending) = result {
            let pending_f64: f64 = pending.to_string().parse().unwrap_or(0.0) / 1e6;
            if pending_f64 > 0.0 {
                println!("  Epoch {}: {:.6} OLP shares pending", epoch, pending_f64);
                found_pending = true;
            }
        }
    }
    if !found_pending {
        println!("  No pending withdrawal requests");
    }

    if shares_f64 < 0.000001 {
        println!("\nNo OLP balance to withdraw.");
        return Ok(());
    }

    // Get withdrawal amount in shares
    println!("\nHow many OLP shares to request withdrawal for?");
    print!("Amount (OLP shares) [0.01]: ");
    io::stdout().flush()?;
    let mut amount_input = String::new();
    io::stdin().read_line(&mut amount_input)?;

    let shares_to_withdraw: f64 = if amount_input.trim().is_empty() {
        0.01
    } else if amount_input.trim().to_lowercase() == "all" {
        shares_f64
    } else {
        amount_input.trim().parse().unwrap_or(0.01)
    };

    if shares_to_withdraw > shares_f64 {
        println!("Withdrawal amount exceeds available balance!");
        return Ok(());
    }

    // Convert to raw shares (6 decimals)
    let shares_raw = alloy::primitives::U256::from((shares_to_withdraw * 1e6) as u128);

    println!("\nInitiating withdrawal request for {:.6} OLP...", shares_to_withdraw);
    let tx_hash = client.request_olp_withdrawal(shares_raw).await?;
    println!("Transaction: {}", tx_hash);

    let receipt = client.wait_for_receipt(tx_hash).await?;
    if receipt.status() {
        println!("Withdrawal request initiated successfully!");
    } else {
        println!("Withdrawal request transaction reverted!");
        return Ok(());
    }

    // Show updated pending withdrawals in parallel
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let futures: Vec<_> = (start_epoch..=current + 1)
        .map(|epoch| {
            let client = client;
            async move { (epoch, client.get_pending_withdrawal(epoch).await) }
        })
        .collect();

    let results = futures::future::join_all(futures).await;

    println!("\n--- Updated Pending Withdrawals ---");
    let mut found_any = false;
    for (epoch, result) in results {
        if let Ok(pending) = result {
            let pending_f64: f64 = pending.to_string().parse().unwrap_or(0.0) / 1e6;
            if pending_f64 > 0.0 {
                println!("  Epoch {}: {:.6} OLP shares pending", epoch, pending_f64);
                found_any = true;
            }
        }
    }
    if !found_any {
        println!("  No pending withdrawal requests");
    }

    // Show remaining balance
    let balance_after = client.get_olp_balance().await?;
    println!("\n--- Remaining OLP Position ---");
    println!("  Shares: {:.6} OLP", balance_after.shares_f64());
    println!("  Value: ${:.2} USDC", balance_after.value);

    Ok(())
}

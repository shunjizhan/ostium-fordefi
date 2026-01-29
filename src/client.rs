//! OstiumClient - main entry point for the SDK

use crate::config::NetworkConfig;
use crate::constants::scale_usdc;
use crate::contracts::{IERC20, IOstiumVault, ITrading, ITradingStorage};
use crate::signer::{TransactionSigner, TxRequest};
use crate::types::{
    BuilderFeeParams, CloseTradeParams, DepositParams, PlaceOrderParams, Position, RedeemParams,
    VaultEpoch, VaultPosition, WithdrawParams, trade::u256_to_u192,
};
use alloy::network::{Ethereum, TransactionBuilder};
use alloy::primitives::{Address, Bytes, TxHash, U256};
use alloy::providers::{Provider, ProviderBuilder, RootProvider};
use alloy::rpc::types::TransactionReceipt;
use alloy::sol_types::SolCall;
use alloy::transports::http::reqwest::Url;
use eyre::{Context, Result};
use std::sync::Arc;

/// Type alias for read-only provider
type ReadProvider = Arc<RootProvider<Ethereum>>;

/// Main client for interacting with Ostium protocol
pub struct OstiumClient<S: TransactionSigner> {
    signer: S,
    config: NetworkConfig,
    provider: ReadProvider,
}

impl<S: TransactionSigner> OstiumClient<S> {
    /// Create a new OstiumClient
    pub async fn new(signer: S, config: NetworkConfig) -> Result<Self> {
        let url: Url = config.rpc_url.parse().context("Invalid RPC URL")?;
        // Read-only provider without fillers (we only do eth_call operations)
        let provider = ProviderBuilder::new()
            .disable_recommended_fillers()
            .network::<Ethereum>()
            .connect_http(url);

        Ok(Self {
            signer,
            config,
            provider: Arc::new(provider),
        })
    }

    /// Get the signer's address
    pub fn address(&self) -> Address {
        self.signer.address()
    }

    /// Get the network configuration
    pub fn config(&self) -> &NetworkConfig {
        &self.config
    }

    // ========== Token Operations ==========

    /// Get USDC balance
    pub async fn get_usdc_balance(&self) -> Result<f64> {
        let balance = self.get_token_balance(self.config.usdc).await?;
        Ok(crate::constants::unscale_from_decimals(
            balance,
            crate::constants::USDC_DECIMALS,
        ))
    }

    /// Get token balance
    async fn get_token_balance(&self, token: Address) -> Result<U256> {
        let call = IERC20::balanceOfCall {
            account: self.address(),
        };
        let data = call.abi_encode();

        let result: Bytes = self
            .provider
            .call(
                alloy::rpc::types::TransactionRequest::default()
                    .with_to(token)
                    .with_input(data),
            )
            .await
            .context("Failed to call balanceOf")?;

        let decoded = IERC20::balanceOfCall::abi_decode_returns(&result)
            .context("Failed to decode balance")?;

        Ok(decoded)
    }

    /// Approve token spending
    pub async fn approve_usdc(&self, spender: Address, amount: f64) -> Result<TxHash> {
        self.approve_token(self.config.usdc, spender, scale_usdc(amount))
            .await
    }

    /// Approve token spending (raw amount)
    async fn approve_token(
        &self,
        token: Address,
        spender: Address,
        amount: U256,
    ) -> Result<TxHash> {
        let call = IERC20::approveCall { spender, amount };
        let data = Bytes::from(call.abi_encode());

        let tx = TxRequest::new(token, data);
        self.signer
            .sign_and_send(tx)
            .await
            .context("Failed to approve token")
    }

    /// Check and ensure USDC allowance
    async fn ensure_usdc_allowance(&self, spender: Address, amount: U256) -> Result<()> {
        let call = IERC20::allowanceCall {
            owner: self.address(),
            spender,
        };
        let data = call.abi_encode();

        let result: Bytes = self
            .provider
            .call(
                alloy::rpc::types::TransactionRequest::default()
                    .with_to(self.config.usdc)
                    .with_input(data),
            )
            .await
            .context("Failed to check allowance")?;

        let decoded = IERC20::allowanceCall::abi_decode_returns(&result)
            .context("Failed to decode allowance")?;

        if decoded < amount {
            // Approve max uint256 for convenience
            self.approve_token(self.config.usdc, spender, U256::MAX)
                .await?;
        }

        Ok(())
    }

    // ========== Trading Operations ==========

    /// Place a new order
    ///
    /// # Arguments
    ///
    /// * `params` - Order parameters including pair, collateral, leverage, etc.
    /// * `builder_fee` - Optional builder/referral fee parameters
    ///
    /// # Returns
    ///
    /// Transaction hash of the submitted order
    pub async fn place_order(
        &self,
        params: PlaceOrderParams,
        builder_fee: Option<BuilderFeeParams>,
    ) -> Result<TxHash> {
        // Validate parameters
        params.validate()?;

        // Ensure USDC allowance to TradingStorage
        let collateral = scale_usdc(params.collateral);
        self.ensure_usdc_allowance(self.config.trading_storage, collateral)
            .await?;

        // Build trade struct
        let trade_index = params.trade_index.unwrap_or(0);
        let trade = params.to_trade(self.address(), trade_index);
        let builder_fee = builder_fee.unwrap_or_default().to_builder_fee();
        let slippage = params.scaled_slippage();

        // Encode call
        let call = ITrading::openTradeCall {
            t: trade,
            bf: builder_fee,
            orderType: params.order_type.into(),
            slippageP: slippage,
        };
        let data = Bytes::from(call.abi_encode());

        // Send transaction
        let tx = TxRequest::new(self.config.trading, data);
        self.signer
            .sign_and_send(tx)
            .await
            .context("Failed to place order")
    }

    /// Close a trade at market price
    ///
    /// # Arguments
    ///
    /// * `params` - Close trade parameters
    ///
    /// # Returns
    ///
    /// Transaction hash of the close order
    pub async fn close_trade(&self, params: CloseTradeParams) -> Result<TxHash> {
        let call = ITrading::closeTradeMarketCall {
            pairIndex: params.pair_index,
            index: params.trade_index,
            closePercentage: params.scaled_close_percentage(),
            marketPrice: params.scaled_market_price(),
            slippageP: params.scaled_slippage(),
        };
        let data = Bytes::from(call.abi_encode());

        let tx = TxRequest::new(self.config.trading, data);
        self.signer
            .sign_and_send(tx)
            .await
            .context("Failed to close trade")
    }

    /// Cancel an open limit order
    pub async fn cancel_order(&self, pair_index: u16, trade_index: u8) -> Result<TxHash> {
        let call = ITrading::cancelOpenLimitOrderCall {
            pairIndex: pair_index,
            index: trade_index,
        };
        let data = Bytes::from(call.abi_encode());

        let tx = TxRequest::new(self.config.trading, data);
        self.signer
            .sign_and_send(tx)
            .await
            .context("Failed to cancel order")
    }

    /// Update take profit price
    pub async fn update_take_profit(
        &self,
        pair_index: u16,
        trade_index: u8,
        new_tp: f64,
    ) -> Result<TxHash> {
        let tp_scaled = u256_to_u192(crate::constants::scale_price(new_tp));

        let call = ITrading::updateTpCall {
            pairIndex: pair_index,
            index: trade_index,
            newTp: tp_scaled,
        };
        let data = Bytes::from(call.abi_encode());

        let tx = TxRequest::new(self.config.trading, data);
        self.signer
            .sign_and_send(tx)
            .await
            .context("Failed to update take profit")
    }

    /// Update stop loss price
    pub async fn update_stop_loss(
        &self,
        pair_index: u16,
        trade_index: u8,
        new_sl: f64,
    ) -> Result<TxHash> {
        let sl_scaled = u256_to_u192(crate::constants::scale_price(new_sl));

        let call = ITrading::updateSlCall {
            pairIndex: pair_index,
            index: trade_index,
            newSl: sl_scaled,
        };
        let data = Bytes::from(call.abi_encode());

        let tx = TxRequest::new(self.config.trading, data);
        self.signer
            .sign_and_send(tx)
            .await
            .context("Failed to update stop loss")
    }

    // ========== Position Queries (Direct Contract Calls) ==========

    /// Get all open positions for an address directly from TradingStorage contract
    ///
    /// This is an alternative to subgraph queries when the subgraph is unavailable.
    /// It iterates through all trading pairs to find open positions.
    ///
    /// # Arguments
    ///
    /// * `trader` - Optional address to query. Defaults to the signer's address.
    ///
    /// # Returns
    ///
    /// Vector of Position structs representing open trades
    pub async fn get_positions(&self, trader: Option<Address>) -> Result<Vec<Position>> {
        let trader = trader.unwrap_or_else(|| self.address());
        let mut positions = Vec::new();

        // Query positions for the most common pairs (0-49)
        // Could be expanded based on pairsCount() if needed
        let max_pairs: u16 = 50;
        let max_trades_per_pair: u8 = 3; // Ostium allows up to 3 trades per pair

        for pair_index in 0..max_pairs {
            // Check open trades count for this pair
            let count = self.get_open_trades_count(trader, pair_index).await?;
            if count == 0 {
                continue;
            }

            // Query each possible trade index
            for trade_index in 0..max_trades_per_pair {
                if let Some(position) = self.get_position(trader, pair_index, trade_index).await? {
                    positions.push(position);
                }
            }
        }

        Ok(positions)
    }

    /// Get open trades count for a specific pair
    async fn get_open_trades_count(&self, trader: Address, pair_index: u16) -> Result<u32> {
        let call = ITradingStorage::openTradesCountCall {
            trader,
            pairIndex: pair_index,
        };

        let result: Bytes = self
            .provider
            .call(
                alloy::rpc::types::TransactionRequest::default()
                    .with_to(self.config.trading_storage)
                    .with_input(call.abi_encode()),
            )
            .await
            .context("Failed to get open trades count")?;

        let decoded = ITradingStorage::openTradesCountCall::abi_decode_returns(&result)
            .context("Failed to decode open trades count")?;

        Ok(decoded)
    }

    /// Get a single position from contract
    async fn get_position(
        &self,
        trader: Address,
        pair_index: u16,
        trade_index: u8,
    ) -> Result<Option<Position>> {
        let call = ITradingStorage::getOpenTradeCall {
            trader,
            pairIndex: pair_index,
            index: trade_index,
        };

        let result: Bytes = self
            .provider
            .call(
                alloy::rpc::types::TransactionRequest::default()
                    .with_to(self.config.trading_storage)
                    .with_input(call.abi_encode()),
            )
            .await
            .context("Failed to get open trade")?;

        let trade = ITradingStorage::getOpenTradeCall::abi_decode_returns(&result)
            .context("Failed to decode open trade")?;

        // Check if position is open (collateral > 0)
        if trade.collateral == U256::ZERO {
            return Ok(None);
        }

        // Convert to Position struct
        let collateral = crate::constants::unscale_from_decimals(
            trade.collateral,
            crate::constants::USDC_DECIMALS,
        );
        let leverage = trade.leverage as f64 / 100.0;
        let open_price = crate::constants::unscale_from_decimals(
            U256::from(trade.openPrice),
            crate::constants::PRICE_DECIMALS,
        );

        // Convert tp and sl (0 means not set)
        let take_profit = if trade.tp != crate::types::U192::ZERO {
            Some(crate::constants::unscale_from_decimals(
                U256::from(trade.tp),
                crate::constants::PRICE_DECIMALS,
            ))
        } else {
            None
        };

        let stop_loss = if trade.sl != crate::types::U192::ZERO {
            Some(crate::constants::unscale_from_decimals(
                U256::from(trade.sl),
                crate::constants::PRICE_DECIMALS,
            ))
        } else {
            None
        };

        Ok(Some(Position {
            trader: trade.trader,
            pair_index: trade.pairIndex,
            trade_index: trade.index,
            collateral,
            leverage,
            is_long: trade.buy,
            open_price,
            take_profit,
            stop_loss,
            unrealized_pnl: None, // PnL requires current price, not available from contract
        }))
    }

    // ========== Vault Operations ==========

    /// Deposit USDC to OLP vault
    ///
    /// # Arguments
    ///
    /// * `params` - Deposit parameters
    ///
    /// # Returns
    ///
    /// Transaction hash of the deposit
    pub async fn deposit_olp(&self, params: DepositParams) -> Result<TxHash> {
        let vault = self
            .config
            .vault
            .ok_or_else(|| eyre::eyre!("Vault address not configured"))?;

        let amount = params.scaled_amount();
        let receiver = params.receiver.unwrap_or_else(|| self.address());

        // Ensure USDC allowance to vault
        self.ensure_usdc_allowance(vault, amount).await?;

        // Encode deposit call
        let call = IOstiumVault::depositCall {
            assets: amount,
            receiver,
        };
        let data = Bytes::from(call.abi_encode());

        let tx = TxRequest::new(vault, data);
        self.signer
            .sign_and_send(tx)
            .await
            .context("Failed to deposit to vault")
    }

    /// Withdraw USDC from OLP vault
    ///
    /// # Arguments
    ///
    /// * `params` - Withdraw parameters
    ///
    /// # Returns
    ///
    /// Transaction hash of the withdrawal
    pub async fn withdraw_olp(&self, params: WithdrawParams) -> Result<TxHash> {
        let vault = self
            .config
            .vault
            .ok_or_else(|| eyre::eyre!("Vault address not configured"))?;

        let amount = params.scaled_amount();
        let receiver = params.receiver.unwrap_or_else(|| self.address());
        let owner = self.address();

        let call = IOstiumVault::withdrawCall {
            assets: amount,
            receiver,
            owner,
        };
        let data = Bytes::from(call.abi_encode());

        let tx = TxRequest::new(vault, data);
        self.signer
            .sign_and_send(tx)
            .await
            .context("Failed to withdraw from vault")
    }

    /// Redeem OLP shares for USDC
    pub async fn redeem_olp(&self, params: RedeemParams) -> Result<TxHash> {
        let vault = self
            .config
            .vault
            .ok_or_else(|| eyre::eyre!("Vault address not configured"))?;

        let receiver = params.receiver.unwrap_or_else(|| self.address());
        let owner = self.address();

        let call = IOstiumVault::redeemCall {
            shares: params.shares,
            receiver,
            owner,
        };
        let data = Bytes::from(call.abi_encode());

        let tx = TxRequest::new(vault, data);
        self.signer
            .sign_and_send(tx)
            .await
            .context("Failed to redeem from vault")
    }

    /// Get OLP share balance
    pub async fn get_olp_balance(&self) -> Result<VaultPosition> {
        let vault = self
            .config
            .vault
            .ok_or_else(|| eyre::eyre!("Vault address not configured"))?;

        // Get share balance
        let balance_call = IOstiumVault::balanceOfCall {
            account: self.address(),
        };
        let balance_result: Bytes = self
            .provider
            .call(
                alloy::rpc::types::TransactionRequest::default()
                    .with_to(vault)
                    .with_input(balance_call.abi_encode()),
            )
            .await
            .context("Failed to get OLP balance")?;

        let shares = IOstiumVault::balanceOfCall::abi_decode_returns(&balance_result)?;

        // Convert shares to assets
        let convert_call = IOstiumVault::convertToAssetsCall { shares };
        let convert_result: Bytes = self
            .provider
            .call(
                alloy::rpc::types::TransactionRequest::default()
                    .with_to(vault)
                    .with_input(convert_call.abi_encode()),
            )
            .await
            .context("Failed to convert shares to assets")?;

        let assets = IOstiumVault::convertToAssetsCall::abi_decode_returns(&convert_result)?;

        Ok(VaultPosition::new(shares, assets))
    }

    /// Initialize a withdrawal request for OLP shares
    ///
    /// This initiates a withdrawal that will be processed in a future epoch.
    /// The shares will be locked until the withdrawal epoch opens.
    ///
    /// # Arguments
    ///
    /// * `shares` - Amount of OLP shares to withdraw (raw value with 6 decimals)
    ///
    /// # Returns
    ///
    /// Transaction hash of the withdrawal request
    pub async fn request_olp_withdrawal(&self, shares: U256) -> Result<TxHash> {
        let vault = self
            .config
            .vault
            .ok_or_else(|| eyre::eyre!("Vault address not configured"))?;

        let call = IOstiumVault::makeWithdrawRequestCall {
            shares,
            owner: self.address(),
        };
        let data = Bytes::from(call.abi_encode());

        let tx = TxRequest::new(vault, data);
        self.signer
            .sign_and_send(tx)
            .await
            .context("Failed to request withdrawal")
    }

    /// Get current vault epoch information
    pub async fn get_vault_epoch(&self) -> Result<VaultEpoch> {
        let vault = self
            .config
            .vault
            .ok_or_else(|| eyre::eyre!("Vault address not configured"))?;

        // Get current epoch
        let epoch_call = IOstiumVault::currentEpochCall {};
        let epoch_result: Bytes = self
            .provider
            .call(
                alloy::rpc::types::TransactionRequest::default()
                    .with_to(vault)
                    .with_input(epoch_call.abi_encode()),
            )
            .await
            .context("Failed to get current epoch")?;
        let current_epoch = IOstiumVault::currentEpochCall::abi_decode_returns(&epoch_result)?;

        // Get epoch end
        let end_call = IOstiumVault::currentEpochEndCall {};
        let end_result: Bytes = self
            .provider
            .call(
                alloy::rpc::types::TransactionRequest::default()
                    .with_to(vault)
                    .with_input(end_call.abi_encode()),
            )
            .await
            .context("Failed to get epoch end")?;
        let epoch_end = IOstiumVault::currentEpochEndCall::abi_decode_returns(&end_result)?;

        // Check if withdrawals are open
        let open_call = IOstiumVault::withdrawalsOpenCall {};
        let open_result: Bytes = self
            .provider
            .call(
                alloy::rpc::types::TransactionRequest::default()
                    .with_to(vault)
                    .with_input(open_call.abi_encode()),
            )
            .await
            .context("Failed to check withdrawals open")?;
        let withdrawals_open = IOstiumVault::withdrawalsOpenCall::abi_decode_returns(&open_result)?;

        Ok(VaultEpoch {
            current_epoch: current_epoch.try_into().unwrap_or(0),
            epoch_end_timestamp: epoch_end.try_into().unwrap_or(0),
            withdrawals_open,
        })
    }

    /// Get pending withdrawal request for the current user
    ///
    /// # Arguments
    ///
    /// * `epoch` - The epoch to check for pending withdrawals
    ///
    /// # Returns
    ///
    /// Amount of shares pending withdrawal for the given epoch
    pub async fn get_pending_withdrawal(&self, epoch: u16) -> Result<U256> {
        let vault = self
            .config
            .vault
            .ok_or_else(|| eyre::eyre!("Vault address not configured"))?;

        let call = IOstiumVault::withdrawRequestsCall {
            owner: self.address(),
            withdrawEpoch: epoch,
        };
        let result: Bytes = self
            .provider
            .call(
                alloy::rpc::types::TransactionRequest::default()
                    .with_to(vault)
                    .with_input(call.abi_encode()),
            )
            .await
            .context("Failed to get pending withdrawal")?;

        let shares = IOstiumVault::withdrawRequestsCall::abi_decode_returns(&result)?;
        Ok(shares)
    }

    // ========== Utility Methods ==========

    /// Wait for transaction confirmation
    pub async fn wait_for_receipt(&self, tx_hash: TxHash) -> Result<TransactionReceipt> {
        self.signer.wait_for_receipt(tx_hash).await
    }

    /// Get native token (ETH) balance
    pub async fn get_eth_balance(&self) -> Result<U256> {
        self.signer.get_balance().await
    }
}

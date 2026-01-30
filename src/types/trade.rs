//! Trading types for user-facing API

use crate::constants::{
    scale_leverage, scale_price, scale_slippage, scale_usdc, DEFAULT_SLIPPAGE, MAX_LEVERAGE,
    MAX_SLIPPAGE, MIN_LEVERAGE,
};
use crate::contracts::{BuilderFee, OrderType, Trade};
use alloy::primitives::{Address, Uint, U256};
use eyre::{ensure, Result};

/// Type alias for U192 (used for prices in Ostium)
pub type U192 = Uint<192, 3>;

/// Helper to convert U256 to U192 (truncating if necessary)
pub fn u256_to_u192(value: U256) -> U192 {
    // Create U192 from the lower 192 bits
    let limbs = value.as_limbs();
    U192::from_limbs([limbs[0], limbs[1], limbs[2]])
}

/// Parameters for placing a new order
#[derive(Debug, Clone)]
pub struct PlaceOrderParams {
    /// Trading pair index (e.g., 0 = BTC/USD)
    pub pair_index: u16,
    /// Collateral amount in USDC (e.g., 100.0 for 100 USDC)
    pub collateral: f64,
    /// Leverage multiplier (e.g., 10.0 for 10x)
    pub leverage: f64,
    /// True for long, false for short
    pub is_long: bool,
    /// Order type (Market, LimitOpen, StopOpen)
    pub order_type: OrderType,
    /// Open price for limit/stop orders (ignored for market orders)
    pub open_price: Option<f64>,
    /// Take profit price (optional)
    pub take_profit: Option<f64>,
    /// Stop loss price (optional)
    pub stop_loss: Option<f64>,
    /// Slippage tolerance in percentage (default: 2%)
    pub slippage: Option<f64>,
    /// Trade index (0-2, auto-selected if None)
    pub trade_index: Option<u8>,
}

impl Default for PlaceOrderParams {
    fn default() -> Self {
        Self {
            pair_index: 0,
            collateral: 0.0,
            leverage: 10.0,
            is_long: true,
            order_type: OrderType::Market,
            open_price: None,
            take_profit: None,
            stop_loss: None,
            slippage: Some(DEFAULT_SLIPPAGE),
            trade_index: None,
        }
    }
}

impl PlaceOrderParams {
    /// Create a new market order
    pub fn market(pair_index: u16, collateral: f64, leverage: f64, is_long: bool) -> Self {
        Self {
            pair_index,
            collateral,
            leverage,
            is_long,
            order_type: OrderType::Market,
            ..Default::default()
        }
    }

    /// Set slippage tolerance
    pub fn with_slippage(mut self, slippage_percent: f64) -> Self {
        self.slippage = Some(slippage_percent);
        self
    }

    /// Set open price (required for market orders to set expected price)
    pub fn with_open_price(mut self, price: f64) -> Self {
        self.open_price = Some(price);
        self
    }

    /// Validate parameters
    pub fn validate(&self) -> Result<()> {
        ensure!(self.collateral > 0.0, "Collateral must be positive");
        ensure!(
            self.leverage >= MIN_LEVERAGE && self.leverage <= MAX_LEVERAGE,
            "Leverage must be between {} and {}",
            MIN_LEVERAGE,
            MAX_LEVERAGE
        );

        if let Some(slippage) = self.slippage {
            ensure!(
                slippage >= 0.0 && slippage <= MAX_SLIPPAGE,
                "Slippage must be between 0 and {}%",
                MAX_SLIPPAGE
            );
        }

        if self.order_type != OrderType::Market {
            ensure!(
                self.open_price.is_some(),
                "Open price required for limit/stop orders"
            );
        }

        Ok(())
    }

    /// Convert to contract Trade struct
    pub fn to_trade(&self, trader: Address, trade_index: u8) -> Trade {
        let collateral = scale_usdc(self.collateral);
        let open_price = u256_to_u192(self.open_price.map(scale_price).unwrap_or(U256::ZERO));
        let tp = u256_to_u192(self.take_profit.map(scale_price).unwrap_or(U256::ZERO));
        let sl = u256_to_u192(self.stop_loss.map(scale_price).unwrap_or(U256::ZERO));
        let leverage = scale_leverage(self.leverage);

        Trade {
            collateral,
            openPrice: open_price,
            tp,
            sl,
            trader,
            leverage,
            pairIndex: self.pair_index,
            index: trade_index,
            buy: self.is_long,
        }
    }

    /// Get slippage as scaled value (PRECISION_2 = 100)
    pub fn scaled_slippage(&self) -> U256 {
        let slippage = self.slippage.unwrap_or(DEFAULT_SLIPPAGE);
        // Slippage uses PRECISION_2 (100), so 2% = 200
        let scaled = (slippage * 100.0) as u128;
        U256::from(scaled)
    }
}

/// Parameters for closing a trade
#[derive(Debug, Clone)]
pub struct CloseTradeParams {
    /// Trading pair index
    pub pair_index: u16,
    /// Trade index (0-2)
    pub trade_index: u8,
    /// Percentage to close (100.0 = 100%)
    pub close_percentage: f64,
    /// Current market price estimate
    pub market_price: f64,
    /// Slippage tolerance in percentage
    pub slippage: Option<f64>,
}

impl CloseTradeParams {
    /// Create params to close entire position
    pub fn close_all(pair_index: u16, trade_index: u8, market_price: f64) -> Self {
        Self {
            pair_index,
            trade_index,
            close_percentage: 100.0,
            market_price,
            slippage: Some(DEFAULT_SLIPPAGE),
        }
    }

    /// Get close percentage scaled (10000 = 100%)
    pub fn scaled_close_percentage(&self) -> u16 {
        (self.close_percentage * 100.0) as u16
    }

    /// Get market price scaled as U192
    pub fn scaled_market_price(&self) -> U192 {
        u256_to_u192(scale_price(self.market_price))
    }

    /// Get slippage scaled (100 = 1%)
    pub fn scaled_slippage(&self) -> u32 {
        let slippage = self.slippage.unwrap_or(DEFAULT_SLIPPAGE);
        scale_slippage(slippage) as u32
    }
}

/// Builder fee parameters (for referral/builder rewards)
#[derive(Debug, Clone, Default)]
pub struct BuilderFeeParams {
    /// Builder address
    pub builder: Option<Address>,
    /// Fee in basis points (100 = 1%)
    pub fee_bps: u32,
}

impl BuilderFeeParams {
    /// Create zero builder fee
    pub fn none() -> Self {
        Self::default()
    }

    /// Convert to contract BuilderFee struct
    pub fn to_builder_fee(&self) -> BuilderFee {
        BuilderFee {
            builder: self.builder.unwrap_or(Address::ZERO),
            builderFee: self.fee_bps,
        }
    }
}

/// Position information returned from queries
#[derive(Debug, Clone)]
pub struct Position {
    /// Trader address
    pub trader: Address,
    /// Trading pair index
    pub pair_index: u16,
    /// Trade index
    pub trade_index: u8,
    /// Collateral in USDC
    pub collateral: f64,
    /// Current leverage
    pub leverage: f64,
    /// Is long position
    pub is_long: bool,
    /// Open price
    pub open_price: f64,
    /// Take profit price
    pub take_profit: Option<f64>,
    /// Stop loss price
    pub stop_loss: Option<f64>,
    /// Unrealized PnL (if available)
    pub unrealized_pnl: Option<f64>,
}

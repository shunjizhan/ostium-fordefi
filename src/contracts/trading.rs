//! Trading contract bindings

use alloy::sol;

sol! {
    /// Trade struct for opening positions
    #[derive(Debug, Default)]
    struct Trade {
        uint256 collateral;    // USDC amount (6 decimals)
        uint192 openPrice;     // Price (18 decimals)
        uint192 tp;            // Take profit price (18 decimals)
        uint192 sl;            // Stop loss price (18 decimals)
        address trader;
        uint32 leverage;       // Leverage in basis points (100x = 10000)
        uint16 pairIndex;
        uint8 index;
        bool buy;              // true = long, false = short
    }

    /// Builder fee struct
    #[derive(Debug, Default)]
    struct BuilderFee {
        address builder;
        uint32 builderFee;     // Fee in basis points
    }

    /// Order type enum
    /// 0 = MARKET
    /// 1 = LIMIT_OPEN
    /// 2 = STOP_OPEN

    /// Trading contract interface
    #[sol(rpc)]
    interface ITrading {
        /// Open a new trade
        function openTrade(
            Trade calldata t,
            BuilderFee calldata bf,
            uint8 orderType,
            uint256 slippageP
        ) external;

        /// Close trade at market price
        function closeTradeMarket(
            uint16 pairIndex,
            uint8 index,
            uint16 closePercentage,
            uint192 marketPrice,
            uint32 slippageP
        ) external;

        /// Cancel an open limit order
        function cancelOpenLimitOrder(
            uint16 pairIndex,
            uint8 index
        ) external;

        /// Update take profit price
        function updateTp(
            uint16 pairIndex,
            uint8 index,
            uint192 newTp
        ) external;

        /// Update stop loss price
        function updateSl(
            uint16 pairIndex,
            uint8 index,
            uint192 newSl
        ) external;

        /// Execute delegated action
        function delegatedAction(
            address trader,
            bytes calldata call_data
        ) external returns (bytes memory);

        /// Get max allowed collateral
        function maxAllowedCollateral() external view returns (uint256);

        /// Check if paused
        function isPaused() external view returns (bool);

        /// Price requested event
        event PriceRequested(
            uint256 indexed orderId,
            address indexed sender,
            bytes32 indexed job,
            uint16 pairIndex,
            bool open,
            uint8 orderType,
            uint256 timestamp
        );
    }
}

/// Order type for opening trades
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum OrderType {
    /// Market order - execute immediately at current price
    #[default]
    Market = 0,
    /// Limit order - execute when price reaches target
    LimitOpen = 1,
    /// Stop order - execute when price moves past threshold
    StopOpen = 2,
}

impl From<OrderType> for u8 {
    fn from(order_type: OrderType) -> u8 {
        order_type as u8
    }
}

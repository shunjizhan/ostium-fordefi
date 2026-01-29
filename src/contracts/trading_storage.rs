//! TradingStorage contract bindings for querying positions

use alloy::sol;

sol! {
    /// Trade struct returned from storage
    #[derive(Debug, Default)]
    struct StoredTrade {
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

    /// Trade info struct with additional metadata
    #[derive(Debug, Default)]
    struct TradeInfo {
        uint192 tradeId;
        uint192 oiNotional;    // Open interest notional
        uint32 lastTradeBlock;
        uint32 lastTradeTs;
    }

    /// TradingStorage contract interface for querying positions
    #[sol(rpc)]
    interface ITradingStorage {
        /// Get count of open trades for a trader on a specific pair
        function openTradesCount(address trader, uint16 pairIndex) external view returns (uint32);

        /// Get a specific open trade
        function getOpenTrade(
            address trader,
            uint16 pairIndex,
            uint8 index
        ) external view returns (StoredTrade memory);

        /// Get trade info (metadata)
        function getOpenTradeInfo(
            address trader,
            uint16 pairIndex,
            uint8 index
        ) external view returns (TradeInfo memory);

        /// Get all open trades for a trader on a pair
        /// Returns an array of trade indices that have open positions
        function openTrades(address trader, uint16 pairIndex, uint8 index) external view returns (StoredTrade memory);

        /// Get the number of trading pairs
        function pairsCount() external view returns (uint16);

        /// Get max trades per pair
        function maxTradesPerPair() external view returns (uint8);

        /// Check if a trade is open (by checking if collateral > 0)
        function hasOpenTrade(address trader, uint16 pairIndex, uint8 index) external view returns (bool);
    }
}

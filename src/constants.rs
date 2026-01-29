//! Constants and precision values for Ostium SDK

use alloy::primitives::U256;

/// USDC has 6 decimals
pub const USDC_DECIMALS: u8 = 6;

/// Prices in Ostium use 18 decimals
pub const PRICE_DECIMALS: u8 = 18;

/// Leverage uses 2 decimals (basis points / 100)
/// e.g., 100x leverage = 10000
pub const LEVERAGE_DECIMALS: u8 = 2;

/// Slippage uses 2 decimals (percentage * 100)
/// e.g., 2% slippage = 200
pub const SLIPPAGE_DECIMALS: u8 = 2;

/// Minimum leverage allowed (2x)
pub const MIN_LEVERAGE: f64 = 2.0;

/// Maximum leverage allowed (1000x)
pub const MAX_LEVERAGE: f64 = 1000.0;

/// Maximum slippage allowed (100%)
pub const MAX_SLIPPAGE: f64 = 100.0;

/// Default slippage (2%)
pub const DEFAULT_SLIPPAGE: f64 = 2.0;

/// Scale a floating point value to U256 with specified decimals
pub fn scale_to_decimals(value: f64, decimals: u8) -> U256 {
    let multiplier = 10u64.pow(decimals as u32);
    let scaled = (value * multiplier as f64) as u128;
    U256::from(scaled)
}

/// Unscale a U256 value to floating point with specified decimals
pub fn unscale_from_decimals(value: U256, decimals: u8) -> f64 {
    let divisor = 10u64.pow(decimals as u32) as f64;
    let value_u128: u128 = value.try_into().unwrap_or(u128::MAX);
    value_u128 as f64 / divisor
}

/// Scale USDC amount (6 decimals)
pub fn scale_usdc(amount: f64) -> U256 {
    scale_to_decimals(amount, USDC_DECIMALS)
}

/// Scale price (18 decimals)
pub fn scale_price(price: f64) -> U256 {
    scale_to_decimals(price, PRICE_DECIMALS)
}

/// Scale leverage (2 decimals / basis points / 100)
pub fn scale_leverage(leverage: f64) -> u32 {
    (leverage * 100.0) as u32
}

/// Scale slippage (2 decimals / percentage * 100)
pub fn scale_slippage(slippage_percent: f64) -> u16 {
    (slippage_percent * 100.0) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_usdc() {
        // 100 USDC = 100_000_000 (6 decimals)
        assert_eq!(scale_usdc(100.0), U256::from(100_000_000u64));
        // 0.5 USDC = 500_000
        assert_eq!(scale_usdc(0.5), U256::from(500_000u64));
    }

    #[test]
    fn test_scale_price() {
        // $50,000 with 18 decimals
        let expected = U256::from(50000u64) * U256::from(10u64).pow(U256::from(18u64));
        assert_eq!(scale_price(50000.0), expected);
    }

    #[test]
    fn test_scale_leverage() {
        // 100x leverage = 10000
        assert_eq!(scale_leverage(100.0), 10000);
        // 2x leverage = 200
        assert_eq!(scale_leverage(2.0), 200);
    }

    #[test]
    fn test_scale_slippage() {
        // 2% slippage = 200
        assert_eq!(scale_slippage(2.0), 200);
        // 0.5% slippage = 50
        assert_eq!(scale_slippage(0.5), 50);
    }
}

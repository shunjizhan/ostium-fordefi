//! OLP Vault contract bindings (ERC-4626)

use alloy::sol;

sol! {
    /// Locked deposit struct
    #[derive(Debug, Default)]
    struct LockedDeposit {
        address owner;
        uint256 shares;
        uint256 assetsDeposited;
        uint256 assetsDiscount;
        uint48 atTimestamp;
        uint48 lockDuration;
    }

    /// ERC-4626 Vault interface with Ostium extensions
    #[sol(rpc)]
    interface IOstiumVault {
        // ERC-4626 Standard Functions

        /// Returns the underlying asset (USDC)
        function asset() external view returns (address);

        /// Returns total assets managed by vault
        function totalAssets() external view returns (uint256);

        /// Converts assets to shares
        function convertToShares(uint256 assets) external view returns (uint256);

        /// Converts shares to assets
        function convertToAssets(uint256 shares) external view returns (uint256);

        /// Returns max deposit amount
        function maxDeposit(address receiver) external view returns (uint256);

        /// Preview deposit shares
        function previewDeposit(uint256 assets) external view returns (uint256);

        /// Deposit assets and receive shares
        function deposit(uint256 assets, address receiver) external returns (uint256 shares);

        /// Returns max mint amount
        function maxMint(address receiver) external view returns (uint256);

        /// Preview mint cost
        function previewMint(uint256 shares) external view returns (uint256);

        /// Mint shares by depositing assets
        function mint(uint256 shares, address receiver) external returns (uint256 assets);

        /// Returns max withdraw amount
        function maxWithdraw(address owner) external view returns (uint256);

        /// Preview withdraw shares
        function previewWithdraw(uint256 assets) external view returns (uint256);

        /// Withdraw assets by burning shares
        function withdraw(uint256 assets, address receiver, address owner) external returns (uint256 shares);

        /// Returns max redeem amount
        function maxRedeem(address owner) external view returns (uint256);

        /// Preview redeem assets
        function previewRedeem(uint256 shares) external view returns (uint256);

        /// Redeem shares for assets
        function redeem(uint256 shares, address receiver, address owner) external returns (uint256 assets);

        // ERC-20 Functions (OLP token)

        /// Returns the name of the vault token
        function name() external view returns (string memory);

        /// Returns the symbol of the vault token
        function symbol() external view returns (string memory);

        /// Returns the decimals of the vault token
        function decimals() external view returns (uint8);

        /// Returns total supply of vault shares
        function totalSupply() external view returns (uint256);

        /// Returns share balance of account
        function balanceOf(address account) external view returns (uint256);

        /// Approves spender
        function approve(address spender, uint256 amount) external returns (bool);

        /// Returns allowance
        function allowance(address owner, address spender) external view returns (uint256);

        /// Transfers shares
        function transfer(address to, uint256 amount) external returns (bool);

        /// Transfers shares from
        function transferFrom(address from, address to, uint256 amount) external returns (bool);

        // Ostium-specific Extensions

        /// Make a withdrawal request for epoch-locked withdrawals
        function makeWithdrawRequest(uint256 shares, address owner) external;

        /// Get locked deposit by ID
        function getLockedDeposit(uint256 depositId) external view returns (LockedDeposit memory);

        /// Current epoch number
        function currentEpoch() external view returns (uint256);

        /// Get pending withdrawal request shares for an address at a specific epoch
        function withdrawRequests(address owner, uint16 withdrawEpoch) external view returns (uint256);

        /// Get current epoch start timestamp
        function currentEpochStart() external view returns (uint256);

        // Events

        /// Emitted on deposit
        event Deposit(address indexed sender, address indexed owner, uint256 assets, uint256 shares);

        /// Emitted on withdraw
        event Withdraw(
            address indexed sender,
            address indexed receiver,
            address indexed owner,
            uint256 assets,
            uint256 shares
        );
    }
}

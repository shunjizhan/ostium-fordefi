//! ERC20 (USDC) contract bindings

use alloy::sol;

sol! {
    /// Standard ERC20 interface
    #[sol(rpc)]
    interface IERC20 {
        /// Returns the name of the token
        function name() external view returns (string memory);

        /// Returns the symbol of the token
        function symbol() external view returns (string memory);

        /// Returns the decimals of the token
        function decimals() external view returns (uint8);

        /// Returns the total supply of the token
        function totalSupply() external view returns (uint256);

        /// Returns the balance of an account
        function balanceOf(address account) external view returns (uint256);

        /// Returns the allowance of a spender
        function allowance(address owner, address spender) external view returns (uint256);

        /// Approves a spender to spend tokens
        function approve(address spender, uint256 amount) external returns (bool);

        /// Transfers tokens to a recipient
        function transfer(address to, uint256 amount) external returns (bool);

        /// Transfers tokens from one address to another
        function transferFrom(address from, address to, uint256 amount) external returns (bool);

        /// Emitted when tokens are transferred
        event Transfer(address indexed from, address indexed to, uint256 value);

        /// Emitted when allowance is set
        event Approval(address indexed owner, address indexed spender, uint256 value);
    }
}

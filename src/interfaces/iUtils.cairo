#[starknet::interface]
pub trait IUtility<TContractState> {
    /// @notice Returns the current STRK/USD price.
    /// @return The price as a u128.
    fn get_strk_usd_price(self: @TContractState) -> u128;
}

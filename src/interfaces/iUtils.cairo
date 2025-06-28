#[starknet::interface]
pub trait IUtility<TContractState> {
    /// @notice Returns the STRK/USD price from the oracle.
    /// @return price The current STRK/USD price as u128.
    fn get_strk_usd_price(self: @TContractState) -> u128;
}

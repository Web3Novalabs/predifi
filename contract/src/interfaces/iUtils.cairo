#[starknet::interface]
pub trait IUtility<TContractState> {
    fn get_strk_usd_price(self: @TContractState) -> u128;
}

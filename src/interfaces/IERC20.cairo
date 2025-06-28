use starknet::ContractAddress;

#[starknet::interface]
pub trait IERC20<TContractState> {
    /// @notice Returns the name of the token.
    fn name(self: @TContractState) -> ByteArray;
    /// @notice Returns the symbol of the token.
    fn symbol(self: @TContractState) -> ByteArray;
    /// @notice Returns the number of decimals used by the token.
    fn decimals(self: @TContractState) -> u8;

    /// @notice Returns the total token supply.
    fn total_supply(self: @TContractState) -> u256;
    /// @notice Returns the balance of a given account.
    /// @param account The account address.
    fn balance_of(self: @TContractState, account: ContractAddress) -> u256;
    /// @notice Returns the allowance from owner to spender.
    /// @param owner The owner address.
    /// @param spender The spender address.
    fn allowance(self: @TContractState, owner: ContractAddress, spender: ContractAddress) -> u256;
    /// @notice Approves a spender to spend tokens.
    /// @param spender The spender address.
    /// @param amount The amount to approve.
    fn approve(ref self: TContractState, spender: ContractAddress, amount: u256) -> bool;
    /// @notice Transfers tokens to a recipient.
    /// @param recipient The recipient address.
    /// @param amount The amount to transfer.
    fn transfer(ref self: TContractState, recipient: ContractAddress, amount: u256) -> bool;
    /// @notice Transfers tokens from sender to recipient.
    /// @param sender The sender address.
    /// @param recipient The recipient address.
    /// @param amount The amount to transfer.
    fn transfer_from(
        ref self: TContractState, sender: ContractAddress, recipient: ContractAddress, amount: u256,
    ) -> bool;

    /// @notice Mints new tokens to a recipient.
    /// @param recipient The recipient address.
    /// @param amount The amount to mint.
    fn mint(ref self: TContractState, recipient: ContractAddress, amount: u256) -> bool;
}

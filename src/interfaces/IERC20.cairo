use starknet::ContractAddress;

#[starknet::interface]
pub trait IERC20<TContractState> {
    /// @notice Returns the name of the token.
    /// @return The token name as a ByteArray.
    fn name(self: @TContractState) -> ByteArray;

    /// @notice Returns the symbol of the token.
    /// @return The token symbol as a ByteArray.
    fn symbol(self: @TContractState) -> ByteArray;

    /// @notice Returns the number of decimals used by the token.
    /// @return The decimals as a u8.
    fn decimals(self: @TContractState) -> u8;

    /// @notice Returns the total token supply.
    /// @return The total supply as u256.
    fn total_supply(self: @TContractState) -> u256;

    /// @notice Returns the balance of a specific account.
    /// @param account The address to query.
    /// @return The account balance as u256.
    fn balance_of(self: @TContractState, account: ContractAddress) -> u256;

    /// @notice Returns the remaining number of tokens that spender can spend on behalf of owner.
    /// @param owner The token owner's address.
    /// @param spender The spender's address.
    /// @return The remaining allowance as u256.
    fn allowance(self: @TContractState, owner: ContractAddress, spender: ContractAddress) -> u256;

    /// @notice Approves the passed address to spend the specified amount of tokens on behalf of
    /// msg.sender.
    /// @param spender The address which will spend the funds.
    /// @param amount The amount of tokens to be spent.
    /// @return True if the operation succeeded.
    fn approve(ref self: TContractState, spender: ContractAddress, amount: u256) -> bool;

    /// @notice Transfers tokens to a specified address.
    /// @param recipient The address to transfer to.
    /// @param amount The amount to be transferred.
    /// @return True if the operation succeeded.
    fn transfer(ref self: TContractState, recipient: ContractAddress, amount: u256) -> bool;

    /// @notice Transfers tokens from one address to another using allowance mechanism.
    /// @param sender The address to send tokens from.
    /// @param recipient The address to transfer to.
    /// @param amount The amount to be transferred.
    /// @return True if the operation succeeded.
    fn transfer_from(
        ref self: TContractState, sender: ContractAddress, recipient: ContractAddress, amount: u256,
    ) -> bool;

    /// @notice Mints new tokens to a specified address.
    /// @param recipient The address to mint tokens to.
    /// @param amount The amount of tokens to mint.
    /// @return True if the operation succeeded.
    fn mint(ref self: TContractState, recipient: ContractAddress, amount: u256) -> bool;
}

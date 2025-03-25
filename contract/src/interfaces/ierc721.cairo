use starknet::ContractAddress;

// TODO: Implement the ERC721Mintable interface
#[starknet::interface]
pub trait IERC721Mintable<TContractState> {
    fn mint(ref self: TContractState, to: ContractAddress, token_id: u256);
    fn safeTransferFrom(
        ref self: TContractState,
        from: ContractAddress,
        to: ContractAddress,
        tokenId: u256,
        data: Span<felt252>,
    );
    fn transferFrom(
        ref self: TContractState, from: ContractAddress, to: ContractAddress, tokenId: u256,
    );
}

#[starknet::interface]
pub trait IERC721Receiver<TContractState> {
    fn on_erc721_received(
        ref self: TContractState,
        operator: ContractAddress,
        from: ContractAddress,
        token_id: u256,
        data: Span<felt252>,
    ) -> felt252;
}

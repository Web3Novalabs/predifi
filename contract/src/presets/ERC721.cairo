#[starknet::contract]
pub mod ERC721 {
    use openzeppelin::introspection::src5::SRC5Component;
    use openzeppelin::token::erc721::{ERC721Component, ERC721HooksEmptyImpl};
    use starknet::{ContractAddress, get_block_timestamp, get_caller_address, get_contract_address};
    use crate::interfaces::ierc721::IERC721Mintable;

    component!(path: ERC721Component, storage: erc721, event: ERC721Event);
    component!(path: SRC5Component, storage: src5, event: SRC5Event);


    impl ERC721InternalImpl = ERC721Component::InternalImpl<ContractState>;


    #[abi(embed_v0)]
    impl ERC721MintableImpl of IERC721Mintable<ContractState> {
        fn mint(ref self: ContractState, to: ContractAddress, token_id: u256) {
            // Only allow minting from within the contract
            assert(get_caller_address() == get_contract_address(), 'Only contract can mint');

            // Check if token already exists

            self.erc721.mint(to, token_id);
        }

        fn safeTransferFrom(
            ref self: ContractState,
            from: ContractAddress,
            to: ContractAddress,
            tokenId: u256,
            data: Span<felt252>,
        ) {
            assert(false, 'NFTs are non-transferable');
        }
    }


    #[storage]
    struct Storage {
        address: ContractAddress,
        #[substorage(v0)]
        erc721: ERC721Component::Storage,
        #[substorage(v0)]
        src5: SRC5Component::Storage,
        owner: ContractAddress,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        ERC721Event: ERC721Component::Event,
        SRC5Event: SRC5Component::Event,
    }

    #[constructor]
    fn constructor(ref self: ContractState, _recipient: ContractAddress) {
        let name = "MNFT";
        let symbol = "NFT";
        let base_uri = "https://api.example.com/v1/";
        self.erc721.initializer(name, symbol, base_uri);
    }
}

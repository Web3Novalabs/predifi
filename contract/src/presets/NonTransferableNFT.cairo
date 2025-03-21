#[starknet::contract]
pub mod NonTransferableNFT {
    use starknet::{ContractAddress, get_caller_address};
    use starknet::storage::{
        Map, StorageMapReadAccess, StorageMapWriteAccess, StoragePointerReadAccess,
        StoragePointerWriteAccess,
    };
    use starknet::event::EventEmitter;
    use core::num::traits::Zero;


    #[starknet::interface]
    pub trait INonTransferableNFT<TContractState> {
        fn mint(ref self: TContractState, pool_id: u256) -> u256;
        fn owner_of(self: @TContractState, token_id: u256) -> ContractAddress;
        fn balance_of(self: @TContractState, owner: ContractAddress) -> u256;
        fn get_pool_id(self: @TContractState, token_id: u256) -> u256;
    }

    #[derive(Copy, Drop, Serde, PartialEq, Debug, starknet::Store)]
    pub struct TokenData {
        pub owner: ContractAddress,
        pub pool_id: u256,
    }

    #[storage]
    struct Storage {
        _token_data: Map<u256, TokenData>,
        _next_token_id: u256,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        Transfer: Transfer,
    }

    #[derive(Drop, starknet::Event)]
    struct Transfer {
        from: ContractAddress,
        to: ContractAddress,
        token_id: u256,
    }

    #[abi(embed_v0)]
    impl NonTransferableNFTImpl of INonTransferableNFT<ContractState> {
        fn mint(ref self: ContractState, pool_id: u256) -> u256 {
            // Get the caller address
            let caller = get_caller_address();

            // Get the next token ID
            let token_id = self._next_token_id.read();

            // Create token data
            let token_data = TokenData { owner: caller, pool_id: pool_id };

            // Store token data
            self._token_data.write(token_id, token_data);

            // Increment next token ID
            self._next_token_id.write(token_id + 1);

            // Emit transfer event
            self.emit(Transfer { from: Zero::zero(), to: caller, token_id: token_id });

            token_id
        }

        fn owner_of(self: @ContractState, token_id: u256) -> ContractAddress {
            let token_data = self._token_data.read(token_id);
            token_data.owner
        }

        fn balance_of(self: @ContractState, owner: ContractAddress) -> u256 {
            let mut balance = 0;
            let next_token_id = self._next_token_id.read();

            let mut i = 0;
            loop {
                if i >= next_token_id {
                    break;
                }

                let token_data = self._token_data.read(i);
                if token_data.owner == owner {
                    balance += 1;
                }
                i += 1;
            };

            balance
        }

        fn get_pool_id(self: @ContractState, token_id: u256) -> u256 {
            let token_data = self._token_data.read(token_id);
            token_data.pool_id
        }
    }
}

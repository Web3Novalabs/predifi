// SPDX-License-Identifier: MIT
use starknet::ContractAddress;

#[starknet::interface]
trait IExternal<ContractState> {
    fn mint(ref self: ContractState, recipient: ContractAddress, amount: u256);
}
#[starknet::contract]
pub mod STARKTOKEN {
    use core::byte_array::ByteArray;
    use openzeppelin::access::ownable::OwnableComponent;
    use openzeppelin::token::erc20::interface::IERC20Metadata;
    use openzeppelin::token::erc20::{ERC20Component, ERC20HooksEmptyImpl};
    use starknet::ContractAddress;
    use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};

    component!(path: ERC20Component, storage: erc20, event: ERC20Event);
    component!(path: OwnableComponent, storage: ownable, event: OwnableEvent);

    #[storage]
    pub struct Storage {
        #[substorage(v0)]
        pub erc20: ERC20Component::Storage,
        #[substorage(v0)]
        pub ownable: OwnableComponent::Storage,
        custom_decimals: u8,
        token_name: ByteArray,
        token_symbol: ByteArray,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        #[flat]
        ERC20Event: ERC20Component::Event,
        #[flat]
        OwnableEvent: OwnableComponent::Event,
    }

    #[constructor]
    fn constructor(
        ref self: ContractState, recipient: ContractAddress, owner: ContractAddress, decimals: u8,
    ) {
        let name: ByteArray = "STRK";
        let symbol: ByteArray = "STRK";
        // Initialize the ERC20 component
        self.erc20.initializer(name, symbol);
        self.ownable.initializer(owner);
        self.custom_decimals.write(decimals);
        self.erc20.mint(recipient, 200_000_000_000_000_000_000_000);
    }

    #[abi(embed_v0)]
    impl CustomERC20MetadataImpl of IERC20Metadata<ContractState> {
        fn name(self: @ContractState) -> ByteArray {
            self.token_name.read()
        }

        fn symbol(self: @ContractState) -> ByteArray {
            self.token_symbol.read()
        }

        fn decimals(self: @ContractState) -> u8 {
            self.custom_decimals.read() // Return custom value
        }
    }

    // Keep existing implementations
    #[abi(embed_v0)]
    impl ERC20Impl = ERC20Component::ERC20Impl<ContractState>;
    #[abi(embed_v0)]
    impl OwnableImpl = OwnableComponent::OwnableImpl<ContractState>;
    impl InternalImpl = ERC20Component::InternalImpl<ContractState>;
    impl OwnableInternalImpl = OwnableComponent::InternalImpl<ContractState>;

    #[abi(embed_v0)]
    impl ExternalImpl of super::IExternal<ContractState> {
        fn mint(ref self: ContractState, recipient: ContractAddress, amount: u256) {
            self.erc20.mint(recipient, amount);
        }
    }
}

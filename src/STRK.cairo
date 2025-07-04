// SPDX-License-Identifier: MIT

/// @title STRK.cairo
/// @notice This file defines the STARKTOKEN contract, which implements the ERC20 token standard.
/// @dev The contract uses OpenZeppelin components for access control and ERC20 functionality.

use starknet::ContractAddress;

#[starknet::interface]
trait IExternal<ContractState> {
    /// @notice Mints new tokens to a specified recipient.
    /// @param self The contract state.
    /// @param recipient The address of the recipient who will receive the minted tokens.
    /// @param amount The amount of tokens to mint.
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
    /// @notice Storage struct for the Predifi contract.
    /// @dev Holds all pools, user stakes, odds, roles, and protocol parameters.
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
    /// @notice Events emitted by the Predifi contract.
    enum Event {
        #[flat]
        ERC20Event: ERC20Component::Event,
        #[flat]
        OwnableEvent: OwnableComponent::Event,
    }

    /// @notice Contract constructor. Initializes the ERC20 and Ownable components.
    /// @param self The contract state.
    /// @param recipient The address to receive the initial supply.
    /// @param owner The address to be set as the contract owner.
    /// @param decimals The number of decimals for the token.
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
        /// @notice Returns the name of the token.
        /// @param self The contract state.
        /// @return The name of the token.
        fn name(self: @ContractState) -> ByteArray {
            self.token_name.read()
        }

        /// @notice Returns the symbol of the token.
        /// @param self The contract state.
        /// @return The symbol of the token.
        fn symbol(self: @ContractState) -> ByteArray {
            self.token_symbol.read()
        }

        /// @notice Returns the number of decimals used to get its user representation.
        /// @param self The contract state.
        /// @return The number of decimals.
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
        /// @notice Mints new tokens to a specified recipient.
        /// @param self The contract state.
        /// @param recipient The address of the recipient who will receive the minted tokens.
        /// @param amount The amount of tokens to mint.
        fn mint(ref self: ContractState, recipient: ContractAddress, amount: u256) {
            self.erc20.mint(recipient, amount);
        }
    }
}

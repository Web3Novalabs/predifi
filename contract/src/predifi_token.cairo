// SPDX-License-Identifier: MIT
// Compatible with OpenZeppelin Contracts for Cairo ^1.0.0
#[starknet::contract]
mod PredifiToken {
    use core::num::traits::Zero;
    use core::starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};
    use openzeppelin::token::erc20::{ERC20Component, ERC20HooksEmptyImpl};
    use starknet::ContractAddress;

    // Include the ERC20 component
    component!(path: ERC20Component, storage: erc20, event: ERC20Event);

    // Implement the ERC20 interfaces by embedding them in the contract's ABI
    #[abi(embed_v0)]
    impl ERC20MixinImpl = ERC20Component::ERC20MixinImpl<ContractState>;

    // Internal implementation for token operations
    impl ERC20InternalImpl = ERC20Component::InternalImpl<ContractState>;

    // Implement the empty hooks implementation (required for v1.0.0)
    impl ERC20HooksImpl = ERC20HooksEmptyImpl<ContractState>;

    #[storage]
    struct Storage {
        #[substorage(v0)]
        erc20: ERC20Component::Storage,
        // Decimal units for the token
        decimal_units: u8,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        #[flat]
        ERC20Event: ERC20Component::Event,
    }

    #[constructor]
    fn constructor(ref self: ContractState, initial_supply: u256, recipient: ContractAddress) {
        // Ensure the recipient address is valid
        assert!(!recipient.is_zero(), "Recipient cannot be the zero address");

        // Initialize the ERC20 token with name and symbol
        let name = "Predifi"; // Will be converted to ByteArray by the initializer
        let symbol = "PDFI"; // Will be converted to ByteArray by the initializer

        self.erc20.initializer(name, symbol);

        // Set the decimals value in storage
        self.decimal_units.write(18);

        // Mint the initial supply to the recipient
        self.erc20.mint(recipient, initial_supply);
    }

    // External function for custom access to decimals
    #[abi(embed_v0)]
    fn get_decimals(self: @ContractState) -> u8 {
        self.decimal_units.read()
    }
}

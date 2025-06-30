#[starknet::contract]
pub mod Utils {
    use core::panics::panic;
    use core::traits::TryInto;
    use pragma_lib::abi::{IPragmaABIDispatcher, IPragmaABIDispatcherTrait};
    use pragma_lib::types::{DataType, PragmaPricesResponse};
    use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};
    use starknet::{ContractAddress, get_caller_address};
    use crate::interfaces::iUtils::IUtility;

    const STRK_USD: felt252 = 6004514686061859652; // STRK/USD in felt252

    #[storage]
    struct Storage {
        pub owner: ContractAddress, // authority of the contract
        pub pragma_contract: ContractAddress //contract address of the pragma contract on respective networks
    }

    /// @notice Initializes the Utils contract.
    /// @param owner The address of the contract owner.
    /// @param pragma_contract The address of the Pragma oracle contract.
    #[constructor]
    fn constructor(
        ref self: ContractState, owner: ContractAddress, pragma_contract: ContractAddress,
    ) {
        self.owner.write(owner);
        self.pragma_contract.write(pragma_contract);
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    pub enum Event {
        OwnerUpdate: OwnerUpdate,
        ContractAddressUpdate: ContractAddressUpdate,
    }

    #[derive(Drop, starknet::Event)]
    pub struct OwnerUpdate {
        pub prev_owner: ContractAddress,
        pub new_owner: ContractAddress,
        #[key]
        pub updated_by: ContractAddress,
    }

    #[derive(Drop, starknet::Event)]
    pub struct ContractAddressUpdate {
        pub prev_contract_address: ContractAddress,
        pub new_contract_address: ContractAddress,
        #[key]
        pub updated_by: ContractAddress,
    }

    #[abi(embed_v0)]
    impl UtilsImpl of IUtility<ContractState> {
        ///   @notice Returns the STRK/USD price from the Pragma oracle.
        ///   @dev Calls the Pragma oracle contract using the stored pragma contract address.
        ///   @return price The current STRK/USD price as u128.
        fn get_strk_usd_price(self: @ContractState) -> u128 {
            /// Retrieve the oracle dispatcher
            let oracle_dispatcher = IPragmaABIDispatcher {
                contract_address: self.pragma_contract.read(),
            };

            /// Call the Oracle contract, for a spot entry
            let output: PragmaPricesResponse = oracle_dispatcher
                .get_data_median(DataType::SpotEntry(STRK_USD));

            return output.price;
        }
    }

    #[generate_trait]
    pub impl InternalFunctions of InternalFunctionsTrait {
        /// @notice Returns the current contract owner.
        /// @return owner The address of the contract owner.
        fn get_owner(self: @ContractState) -> ContractAddress {
            self.owner.read()
        }
        /// @notice Sets a new contract owner.
        /// @dev Only callable by the current owner. Emits OwnerUpdate event.
        /// @param new_owner The address of the new owner.
        fn set_owner(ref self: ContractState, new_owner: ContractAddress) {
            let caller: ContractAddress = get_caller_address();
            let zero_addr: ContractAddress = 0x0.try_into().unwrap();

            if (caller != self.get_owner()) {
                panic!("Only the owner can set ownership");
            }

            if (new_owner == zero_addr) {
                panic!("Cannot change ownership to 0x0");
            }

            let prev_owner: ContractAddress = self.owner.read();
            self.owner.write(new_owner);

            self.emit(OwnerUpdate { prev_owner, new_owner, updated_by: caller });
        }

        // PRAGMA PRICE FEED INTERNAL FUNCTIONS

        /// @notice Returns the current Pragma contract address.
        /// @return pragma_contract The address of the Pragma contract.
        fn get_pragma_contract_address(self: @ContractState) -> ContractAddress {
            self.pragma_contract.read()
        }

        /// @notice Updates the Pragma contract address.
        /// @dev Only callable by the owner. Emits ContractAddressUpdate event.
        /// @param pragma_contract The new Pragma contract address.
        fn set_pragma_contract_address(ref self: ContractState, pragma_contract: ContractAddress) {
            let caller: ContractAddress = get_caller_address();
            let zero_addr: ContractAddress = 0x0.try_into().unwrap();

            if (caller != self.get_owner()) {
                panic!("Only the owner can change contract address");
            }

            if (pragma_contract == zero_addr) {
                panic!("Cannot change contract address to 0x0");
            }

            let current_contract: ContractAddress = self.pragma_contract.read();
            self.pragma_contract.write(pragma_contract);

            self
                .emit(
                    ContractAddressUpdate {
                        prev_contract_address: current_contract,
                        new_contract_address: pragma_contract,
                        updated_by: caller,
                    },
                );
        }
    }
}

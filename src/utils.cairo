#[starknet::contract]
pub mod Utils {
    use core::traits::TryInto;
    use pragma_lib::abi::{IPragmaABIDispatcher, IPragmaABIDispatcherTrait};
    use pragma_lib::types::{DataType, PragmaPricesResponse};
    use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};
    use starknet::{ContractAddress, get_caller_address};
    use crate::interfaces::iUtils::IUtility;

    const STRK_USD: felt252 = 6004514686061859652; // STRK/USD in felt252

    /// @notice Storage struct for Utils contract.
    /// @dev Holds the owner and the pragma contract address.
    #[storage]
    struct Storage {
        /// @notice The authority of the contract.
        pub owner: ContractAddress,
        /// @notice The contract address of the pragma contract on respective networks.
        pub pragma_contract: ContractAddress,
    }

    /// @notice Initializes the contract with the owner and pragma contract address.
    /// @param self The contract state.
    /// @param owner The address to be set as the contract owner.
    /// @param pragma_contract The address of the pragma contract.
    #[constructor]
    fn constructor(
        ref self: ContractState, owner: ContractAddress, pragma_contract: ContractAddress,
    ) {
        self.owner.write(owner);
        self.pragma_contract.write(pragma_contract);
    }

    /// @notice Events emitted by the Utils contract.
    #[event]
    #[derive(Drop, starknet::Event)]
    pub enum Event {
        /// @notice Emitted when the owner is updated.
        OwnerUpdate: OwnerUpdate,
        /// @notice Emitted when the pragma contract address is updated.
        ContractAddressUpdate: ContractAddressUpdate,
    }

    /// @notice Event emitted when the owner is updated.
    /// @param prev_owner The previous owner address.
    /// @param new_owner The new owner address.
    /// @param updated_by The address that performed the update.
    #[derive(Drop, starknet::Event)]
    pub struct OwnerUpdate {
        /// @notice The previous owner address.
        pub prev_owner: ContractAddress,
        /// @notice The new owner address.
        pub new_owner: ContractAddress,
        /// @notice The address that performed the update.
        #[key]
        pub updated_by: ContractAddress,
    }

    /// @notice Event emitted when the pragma contract address is updated.
    /// @param prev_contract_address The previous pragma contract address.
    /// @param new_contract_address The new pragma contract address.
    /// @param updated_by The address that performed the update.
    #[derive(Drop, starknet::Event)]
    pub struct ContractAddressUpdate {
        /// @notice The previous pragma contract address.
        pub prev_contract_address: ContractAddress,
        /// @notice The new pragma contract address.
        pub new_contract_address: ContractAddress,
        /// @notice The address that performed the update.
        #[key]
        pub updated_by: ContractAddress,
    }

    #[abi(embed_v0)]
    impl UtilsImpl of IUtility<ContractState> {
        /// @notice Gets the STRK/USD price from the Pragma oracle.
        /// @dev Calls the Pragma contract for the STRK/USD spot entry.
        /// @return The STRK/USD price as u128.
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
        // CONTRACT OWNER SPECIFICS

        /// @notice Returns the current owner of the contract.
        /// @return The owner address.
        fn get_owner(self: @ContractState) -> ContractAddress {
            self.owner.read()
        }

        /// @notice Sets a new owner for the contract.
        /// @dev Only callable by the current owner. Emits OwnerUpdate event.
        /// @param new_owner The address to set as the new owner.
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

        /// @notice Returns the current pragma contract address.
        /// @return The pragma contract address.
        fn get_pragma_contract_address(self: @ContractState) -> ContractAddress {
            self.pragma_contract.read()
        }

        /// @notice Updates the pragma contract address.
        /// @dev Only callable by the owner. Emits ContractAddressUpdate event.
        /// @param pragma_contract The new pragma contract address.
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

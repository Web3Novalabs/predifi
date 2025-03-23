#[starknet::contract]
pub mod Utils {
    use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};
    use starknet::{ContractAddress, get_caller_address};
    use core::traits::TryInto;
    use core::panics::panic;

    use pragma_lib::abi::{IPragmaABIDispatcher, IPragmaABIDispatcherTrait};
    use pragma_lib::types::{DataType, PragmaPricesResponse};

    use crate::interfaces::iUtils::IUtility;

    const STRK_USD: felt252 = 6004514686061859652; // STRK/USD in felt252

    #[storage]
    struct Storage {
        pub owner: ContractAddress, // authority of the contract
        pub pragma_contract: ContractAddress //contract address of the pragma contract on respective networks
    }

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
        //  PRAGMA PRICE FEED INTEGRATION
        //   @inputs - Contract State to reterieve the contract address of the pragma contract
        //   @output - STRK/USD price in felt

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

        fn get_owner(self: @ContractState) -> ContractAddress {
            self.owner.read()
        }

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

        /// reading pragma contract address
        fn get_pragma_contract_address(self: @ContractState) -> ContractAddress {
            self.pragma_contract.read()
        }

        /// updating pragma contract address
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
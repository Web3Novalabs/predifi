use contract::base::events::Events::{
    PoolCancelled, StakeRefunded, ValidatorAdded, ValidatorRemoved,
};
use contract::base::types::{Pool, PoolDetails, Status};
use contract::interfaces::iUtils::IUtilityDispatcher;
use contract::interfaces::ipredifi::{
    IPredifiDispatcher, IPredifiDispatcherTrait, IPredifiDisputeDispatcher,
    IPredifiDisputeDispatcherTrait, IPredifiValidator, IPredifiValidatorDispatcher,
    IPredifiValidatorDispatcherTrait,
};
use contract::predifi::Predifi;
use contract::utils::Utils;
use contract::utils::Utils::InternalFunctionsTrait;
use core::array::ArrayTrait;
use core::felt252;
use core::serde::Serde;
use core::traits::{Into, TryInto};
use openzeppelin::access::accesscontrol::AccessControlComponent::InternalTrait as AccessControlInternalTrait;
use openzeppelin::access::accesscontrol::DEFAULT_ADMIN_ROLE;
use openzeppelin::token::erc20::interface::{IERC20Dispatcher, IERC20DispatcherTrait};
use openzeppelin::upgrades::upgradeable::UpgradeableComponent::{Event as UpgradeEvent, Upgraded};
use snforge_std::{
    ContractClassTrait, DeclareResultTrait, EventSpyAssertionsTrait, EventSpyTrait, declare,
    get_class_hash, spy_events, start_cheat_block_timestamp, start_cheat_caller_address,
    stop_cheat_block_timestamp, stop_cheat_caller_address, test_address,
};
use starknet::storage::{MutableVecTrait, StoragePointerReadAccess, StoragePointerWriteAccess};
use starknet::{ClassHash, ContractAddress, get_block_timestamp, get_caller_address};


// Validator role
const VALIDATOR_ROLE: felt252 = selector!("VALIDATOR_ROLE");
// Pool creator address constant
const POOL_CREATOR: ContractAddress = 123.try_into().unwrap();

const USER_ONE: ContractAddress = 'User1'.try_into().unwrap();

pub fn deploy_predifi() -> (
    IPredifiDispatcher,
    IPredifiDisputeDispatcher,
    IPredifiValidatorDispatcher,
    ContractAddress,
    ContractAddress,
) {
    let owner: ContractAddress = 'owner'.try_into().unwrap();
    let admin: ContractAddress = 'admin'.try_into().unwrap();

    // Deploy mock ERC20
    let erc20_class = declare("STARKTOKEN").unwrap().contract_class();
    let mut calldata = array![POOL_CREATOR.into(), owner.into(), 6];
    let (erc20_address, _) = erc20_class.deploy(@calldata).unwrap();

    let contract_class = declare("Predifi").unwrap().contract_class();

    let (contract_address, _) = contract_class
        .deploy(@array![erc20_address.into(), admin.into()])
        .unwrap();

    // Dispatchers
    let dispatcher = IPredifiDispatcher { contract_address };
    let dispute_dispatcher = IPredifiDisputeDispatcher { contract_address };
    let validator_dispatcher = IPredifiValidatorDispatcher { contract_address };
    (dispatcher, dispute_dispatcher, validator_dispatcher, POOL_CREATOR, erc20_address)
}


// Helper function for creating pools with default parameters
pub fn create_default_pool(contract: IPredifiDispatcher) -> u256 {
    contract
        .create_pool(
            'Example Pool',
            0, // 0 = WinBet
            "A simple betting pool",
            "image.png",
            "event.com/details",
            1710000000,
            1710003600,
            1710007200,
            'Team A',
            'Team B',
            100,
            10000,
            5,
            false,
            0,
        )
}

// Helper function to declare Contract Class and return the Class Hash
pub fn declare_contract(name: ByteArray) -> ClassHash {
    let declare_result = declare(name);
    let declared_contract = declare_result.unwrap().contract_class();
    *declared_contract.class_hash
}

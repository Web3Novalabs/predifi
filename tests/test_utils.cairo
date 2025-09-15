use contract::STRK::{IExternalDispatcher as STRKDispatcher, IExternalDispatcherTrait};
use contract::base::types::PoolDetails;
use contract::interfaces::ipredifi::{
    IPredifiDispatcher, IPredifiDispatcherTrait, IPredifiDisputeDispatcher,
    IPredifiValidatorDispatcher,
};
use core::array::ArrayTrait;
use core::felt252;
use core::traits::{Into, TryInto};
use openzeppelin::token::erc20::interface::{IERC20Dispatcher, IERC20DispatcherTrait};
use snforge_std::{
    ContractClassTrait, DeclareResultTrait, declare, start_cheat_caller_address,
    stop_cheat_caller_address,
};
use starknet::{ClassHash, ContractAddress, get_block_timestamp};


// Validator role
const VALIDATOR_ROLE: felt252 = selector!("VALIDATOR_ROLE");
// Helper functions to avoid const try_into issues
fn get_pool_creator() -> ContractAddress {
    123.try_into().unwrap()
}

fn get_user_one() -> ContractAddress {
    'User1'.try_into().unwrap()
}

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
    let mut calldata = array![get_pool_creator().into(), owner.into(), 6];
    let (erc20_address, _) = erc20_class.deploy(@calldata).unwrap();

    let contract_class = declare("Predifi").unwrap().contract_class();

    let (contract_address, _) = contract_class
        .deploy(@array![erc20_address.into(), admin.into()])
        .unwrap();

    // Dispatchers
    let dispatcher = IPredifiDispatcher { contract_address };
    let dispute_dispatcher = IPredifiDisputeDispatcher { contract_address };
    let validator_dispatcher = IPredifiValidatorDispatcher { contract_address };
    (dispatcher, dispute_dispatcher, validator_dispatcher, get_pool_creator(), erc20_address)
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


pub fn setup_user_with_tokens(user: ContractAddress, erc20_address: ContractAddress, amount: u256) {
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    start_cheat_caller_address(erc20_address, user);
    erc20.approve(erc20_address, amount); // Approve the ERC20 contract itself to mint
    stop_cheat_caller_address(erc20_address);
}


pub fn pool_exists_in_array(pools: Array<PoolDetails>, pool_id: u256) -> bool {
    let mut i = 0;
    let len = pools.len();

    loop {
        if i >= len {
            break false;
        }

        let pool = pools.at(i);
        // Use the correct reference type for comparison
        if *pool.pool_id == pool_id {
            break true;
        }

        i += 1;
    }
}


pub fn get_default_pool_params() -> (
    felt252,
    u8,
    ByteArray,
    ByteArray,
    ByteArray,
    u64,
    u64,
    u64,
    felt252,
    felt252,
    u256,
    u256,
    u8,
    bool,
    u8,
) {
    let current_time = get_block_timestamp();
    (
        'Default Pool',
        0, // 0 = WinBet
        "Default Description",
        "default_image.jpg",
        "https://example.com",
        current_time + 86400,
        current_time + 172800,
        current_time + 259200,
        'Option A',
        'Option B',
        1_000_000_000_000_000_000,
        10_000_000_000_000_000_000,
        5,
        false,
        0,
    )
}


pub fn create_test_pool(
    dispatcher: IPredifiDispatcher,
    poolName: felt252,
    poolStartTime: u64,
    poolLockTime: u64,
    poolEndTime: u64,
) -> u256 {
    dispatcher
        .create_pool(
            poolName,
            0, // 0 = WinBet
            "Test Description",
            "Test Image",
            "Test URL",
            poolStartTime,
            poolLockTime,
            poolEndTime,
            'Option 1',
            'Option 2',
            100_u256,
            1000_u256,
            5,
            false,
            0,
        )
}


pub fn approve_tokens_for_payment(
    contract_address: ContractAddress, erc20_address: ContractAddress, amount: u256,
) {
    // Approve token spending for pool creation
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
    // Approve the contract to spend tokens
    erc20.approve(contract_address, amount);
}

// Helper to mint STRK to a user on the mock token
pub fn mint_tokens_for(user: ContractAddress, erc20_address: ContractAddress, amount: u256) {
    let mut strk: STRKDispatcher = STRKDispatcher { contract_address: erc20_address };
    strk.mint(user, amount);
}

// Helper function to setup token distribution and approvals for multiple users
pub fn setup_tokens_and_approvals(
    erc20_address: ContractAddress, contract_address: ContractAddress, users: Span<ContractAddress>,
) {
    let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };

    // Distribute tokens and approve contract
    let mut i = 0;
    while i != users.len() {
        let user = *users.at(i);
        start_cheat_caller_address(erc20_address, POOL_CREATOR);

        // Transfer tokens to user
        erc20.transfer(user, 1000 * 1_000_000_000_000_000_000);
        stop_cheat_caller_address(erc20_address);

        start_cheat_caller_address(erc20_address, user);
        erc20.approve(contract_address, 1000 * 1_000_000_000_000_000_000);
        stop_cheat_caller_address(erc20_address);
        i += 1;
    };
}

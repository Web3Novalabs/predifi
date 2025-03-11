
use starknet::{ContractAddress, contract_address_const, ClassHash};

use snforge_std::{declare, ContractClassTrait, DeclareResultTrait};

//use starknet::{ContractAddress, SyscallResultTrait};
//use core::result::ResultTrait;
//use core::byte_array::ByteArray;

use contract::base::types::{PoolDetails, Pool, Status, Category};
use contract::interfaces::ipredifi::{IPredifiDispatcher, IPredifiDispatcherTrait};

fn owner() -> ContractAddress {
    'owner'.try_into().unwrap()
}
fn deploy_predifi() -> IPredifiDispatcher {
    let contract_class = declare("Predifi").unwrap().contract_class();

    let mut calldata = array![];
    owner().serialize(ref calldata);
    let (contract_address, _) = contract_class.deploy(@calldata).unwrap();
    (IPredifiDispatcher { contract_address })
}


// #[test]
fn test_create_pool() {
    let contract = deploy_predifi();
    let result = contract
        .create_pool(
            'Example Pool',
            Pool::WinBet,
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
            Category::Sports,
        );

    assert!(result == true, "not created");
}

#[test]
fn test_get_all_pools() {
    // Deploy the contract
    let contract = deploy_predifi();

    // Verify that initially there are no pools
    let initial_pools = contract.get_all_pools();
    assert!(initial_pools.len() == 0, "Should have 0 pools initially");

    // Create the first pool
    let result1 = contract
        .create_pool(
            'Pool1',
            Pool::WinBet,
            "P1",
            "i1.png",
            "u1.com",
            1710000000,
            1710003600,
            1710007200,
            'OpA',
            'OpB',
            100,
            10000,
            5,
            false,
            Category::Sports,
        );
    assert!(result1 == true, "Pool 1 not created");

    // Verify that there is one pool
    let pools_after_first = contract.get_all_pools();
    assert!(pools_after_first.len() == 1, "Should have 1 pool after the first creation");

    // Verify the details of the first pool
    let first_pool = pools_after_first.at(0);
    assert!(*first_pool.pool_id == 1_u256, "Incorrect pool ID");
    assert!(*first_pool.poolName == 'Pool1', "Incorrect pool name");
    assert!(*first_pool.option1 == 'OpA', "Incorrect option 1");
    assert!(*first_pool.option2 == 'OpB', "Incorrect option 2");

    // Create a second pool
    let result2 = contract
        .create_pool(
            'Pool2',
            Pool::VoteBet,
            "P2",
            "i2.png",
            "u2.com",
            1720000000,
            1720003600,
            1720007200,
            'EqX',
            'EqY',
            200,
            20000,
            10,
            true,
            Category::Entertainment,
        );
    assert!(result2 == true, "Pool 2 not created");

    // Verify that there are two pools
    let final_pools = contract.get_all_pools();
    assert!(final_pools.len() == 2, "Should have 2 pools after the second creation");

    // Verify the details of the second pool
    let second_pool = final_pools.at(1);
    assert!(*second_pool.pool_id == 2_u256, "Incorrect second pool ID");
    assert!(*second_pool.poolName == 'Pool2', "Incorrect second pool name");
    assert!(*second_pool.option1 == 'EqX', "Incorrect second pool option 1");
    assert!(*second_pool.option2 == 'EqY', "Incorrect second pool option 2");
}

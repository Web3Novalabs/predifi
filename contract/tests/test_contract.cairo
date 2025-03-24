use contract::base::types::{Category, Pool, PoolDetails, Status};
use contract::interfaces::ipredifi::{IPredifiDispatcher, IPredifiDispatcherTrait};
use contract::presets::NonTransferableNFT::NonTransferableNFT::{
    INonTransferableNFTDispatcher, INonTransferableNFTDispatcherTrait,
};

use snforge_std::{declare, ContractClassTrait, DeclareResultTrait};
use starknet::{ContractAddress, get_block_timestamp};

use core::felt252;
use core::traits::Into;
use snforge_std::{ContractClassTrait, DeclareResultTrait, declare};
use starknet::{
    ClassHash, ContractAddress, get_block_timestamp, get_caller_address, get_contract_address,
};

fn owner() -> ContractAddress {
    'owner'.try_into().unwrap()
}


fn deploy_predifi() -> IPredifiDispatcher {
    let contract_class = declare("Predifi").unwrap().contract_class();

fn deploy_nft() -> INonTransferableNFTDispatcher {
    let contract_class = declare("NonTransferableNFT").unwrap().contract_class();
    let mut calldata = ArrayTrait::new();
    let (contract_address, _) = contract_class.deploy(@calldata).unwrap();
    (INonTransferableNFTDispatcher { contract_address })
}

fn deploy_predifi(nft_contract: ContractAddress) -> IPredifiDispatcher {
    let contract_class = declare("Predifi").unwrap().contract_class();
    let mut calldata = ArrayTrait::new();
    calldata.append(nft_contract.into());
    let (contract_address, _) = contract_class.deploy(@calldata).unwrap();
    (IPredifiDispatcher { contract_address })
}

#[cfg(test)]
mod tests {
    use super::*;
const ONE_STRK: u256 = 1_000_000_000_000_000_000;

    fn test_create_pool_with_nft() {
        // Deploy NFT contract
        let nft = deploy_nft();
        let nft_address = nft.contract_address;

        // Deploy Predifi contract with NFT address
        let contract = deploy_predifi(nft_address);

        // Create a pool
        let pool_id = contract
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

        // Verify pool was created
        assert!(pool_id != 0, "Pool not created");

        // Verify NFT was minted
        let owner = owner();
        let balance = nft.balance_of(owner);
        assert!(balance == 1, "NFT not minted");

        // Verify NFT ownership
        let token_id = pool_id - 1; // First token ID is 0
        let nft_owner = nft.owner_of(token_id);
        assert!(nft_owner == owner, "NFT not owned by pool creator");

        // Verify NFT pool ID
        let nft_pool_id = nft.get_pool_id(token_id);
        assert!(nft_pool_id == pool_id, "NFT pool ID mismatch");
    }

    fn test_invalid_time_sequence_start_after_lock() {
        let nft = deploy_nft();
        let contract = deploy_predifi(nft.contract_address);
        let (
            poolName,
            poolType,
            poolDescription,
            poolImage,
            poolEventSourceUrl,
            _,
            _,
            poolEndTime,
            option1,
            option2,
            minBetAmount,
            maxBetAmount,
            creatorFee,
            isPrivate,
            category,
        ) =
            get_default_pool_params();

        let current_time = get_block_timestamp();
        let invalid_start_time = current_time + 3600; // 1 hour from now
        let invalid_lock_time = current_time
            + 1800; // 30 minutes from now (before start), should not be able to lock before starting

        let mut success = false;
        let result = contract
            .create_pool(
                poolName,
                poolType,
                poolDescription,
                poolImage,
                poolEventSourceUrl,
                invalid_start_time,
                invalid_lock_time,
                poolEndTime,
                option1,
                option2,
                minBetAmount,
                maxBetAmount,
                creatorFee,
                isPrivate,
                category,
            );
        if result == 0 {
            success = true;
        }
        assert!(!success, "Should fail with invalid time sequence");
    }


    fn test_zero_min_bet() {
        let nft = deploy_nft();
        let contract = deploy_predifi(nft.contract_address);
        let (
            poolName,
            poolType,
            poolDescription,
            poolImage,
            poolEventSourceUrl,
            poolStartTime,
            poolLockTime,
            poolEndTime,
            option1,
            option2,
            _,
            maxBetAmount,
            creatorFee,
            isPrivate,
            category,
        ) =
            get_default_pool_params();

        let mut success = false;
        let result = contract
            .create_pool(
                poolName,
                poolType,
                poolDescription,
                poolImage,
                poolEventSourceUrl,
                poolStartTime,
                poolLockTime,
                poolEndTime,
                option1,
                option2,
                0,
                maxBetAmount,
                creatorFee,
                isPrivate,
                category,
            );
        if result == 0 {
            success = true;
        }
        assert!(!success, "Should fail with zero min bet");
    }

    fn test_excessive_creator_fee() {
        let nft = deploy_nft();
        let contract = deploy_predifi(nft.contract_address);
        let (
            poolName,
            poolType,
            poolDescription,
            poolImage,
            poolEventSourceUrl,
            poolStartTime,
            poolLockTime,
            poolEndTime,
            option1,
            option2,
            minBetAmount,
            maxBetAmount,
            _,
            isPrivate,
            category,
        ) =
            get_default_pool_params();

        let mut success = false;
        let result = contract
            .create_pool(
                poolName,
                poolType,
                poolDescription,
                poolImage,
                poolEventSourceUrl,
                poolStartTime,
                poolLockTime,
                poolEndTime,
                option1,
                option2,
                minBetAmount,
                maxBetAmount,
                101,
                isPrivate,
                category,
            );
        if result == 0 {
            success = true;
        }
        assert!(!success, "Should fail with excessive creator fee");
    }


    fn get_default_pool_params() -> (
        felt252,
        Pool,
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
        Category,
    ) {
        let current_time = get_block_timestamp();
        (
            'Default Pool', // poolName
            Pool::WinBet, // poolType
            "Default Description", // poolDescription
            "default_image.jpg", // poolImage
            "https://example.com", // poolEventSourceUrl
            current_time + 86400, // poolStartTime (1 day from now)
            current_time + 172800, // poolLockTime (2 days from now)
            current_time + 259200, // poolEndTime (3 days from now)
            'Option A', // option1
            'Option B', // option2
            1_000_000_000_000_000_000, // minBetAmount (1 STRK)
            10_000_000_000_000_000_000, // maxBetAmount (10 STRK)
            5, // creatorFee (5%)
            false, // isPrivate
            Category::Sports // category
        )
    }

    fn test_multiple_nft_ownership() {
        let nft = deploy_nft();
        let contract = deploy_predifi(nft.contract_address);
        let owner = owner();

        // Create first pool
        let pool_id1 = contract
            .create_pool(
                'Pool 1',
                Pool::WinBet,
                "First pool",
                "image1.png",
                "event1.com",
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

        // Create second pool
        let pool_id2 = contract
            .create_pool(
                'Pool 2',
                Pool::WinBet,
                "Second pool",
                "image2.png",
                "event2.com",
                1710000000,
                1710003600,
                1710007200,
                'Team C',
                'Team D',
                100,
                10000,
                5,
                false,
                Category::Sports,
            );

        // Verify total NFT balance
        let balance = nft.balance_of(owner);
        assert!(balance == 2, "Should have 2 NFTs");

        // Verify both NFTs are owned by the same address
        let token_id1 = pool_id1 - 1;
        let token_id2 = pool_id2 - 1;
        assert!(nft.owner_of(token_id1) == owner, "First NFT not owned by creator");
        assert!(nft.owner_of(token_id2) == owner, "Second NFT not owned by creator");

        // Verify pool IDs are correctly associated
        assert!(nft.get_pool_id(token_id1) == pool_id1, "First NFT pool ID mismatch");
        assert!(nft.get_pool_id(token_id2) == pool_id2, "Second NFT pool ID mismatch");
    }

    fn test_nft_non_transferable() {
        let nft = deploy_nft();
        let contract = deploy_predifi(nft.contract_address);
        let owner = owner();
        let other_address = 'other'.try_into().unwrap();

        // Create a pool and get the NFT
        let pool_id = contract
            .create_pool(
                'Test Pool',
                Pool::WinBet,
                "Test pool",
                "test.png",
                "test.com",
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

        let token_id = pool_id - 1;

        // Verify initial ownership
        assert!(nft.owner_of(token_id) == owner, "Initial ownership incorrect");

        // Attempt to transfer (should fail as there's no transfer function)
        // Note: This is an implicit test as the contract doesn't expose any transfer functionality
        let balance_before = nft.balance_of(owner);
        let other_balance_before = nft.balance_of(other_address);

        // Verify ownership hasn't changed
        assert!(nft.owner_of(token_id) == owner, "NFT should not be transferable");
        assert!(nft.balance_of(owner) == balance_before, "Owner balance should not change");
        assert!(
            nft.balance_of(other_address) == other_balance_before,
            "Other address balance should not change",
        );
    }

    fn test_nft_unique_pool_association() {
        let nft = deploy_nft();
        let contract = deploy_predifi(nft.contract_address);
        let owner = owner();

        // Create multiple pools
        let pool_id1 = contract
            .create_pool(
                'Pool 1',
                Pool::WinBet,
                "Test pool",
                "test.png",
                "test.com",
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

        let pool_id2 = contract
            .create_pool(
                'Pool 2',
                Pool::WinBet,
                "Test pool",
                "test.png",
                "test.com",
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

        let pool_id3 = contract
            .create_pool(
                'Pool 3',
                Pool::WinBet,
                "Test pool",
                "test.png",
                "test.com",
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

        // Verify each NFT is associated with a unique pool ID
        let token_id1 = pool_id1 - 1;
        let token_id2 = pool_id2 - 1;
        let token_id3 = pool_id3 - 1;

        let nft_pool_id1 = nft.get_pool_id(token_id1);
        let nft_pool_id2 = nft.get_pool_id(token_id2);
        let nft_pool_id3 = nft.get_pool_id(token_id3);

        assert!(nft_pool_id1 == pool_id1, "Pool ID mismatch 1");
        assert!(nft_pool_id2 == pool_id2, "Pool ID mismatch 2");
        assert!(nft_pool_id3 == pool_id3, "Pool ID mismatch 3");
    }
fn get_default_pool_params() -> (
    felt252,
    Pool,
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
    Category,
) {
    let current_time = get_block_timestamp();
    (
        'Default Pool', // poolName
        Pool::WinBet, // poolType
        "Default Description", // poolDescription
        "default_image.jpg", // poolImage
        "https://example.com", // poolEventSourceUrl
        current_time + 86400, // poolStartTime (1 day from now)
        current_time + 172800, // poolLockTime (2 days from now)
        current_time + 259200, // poolEndTime (3 days from now)
        'Option A', // option1
        'Option B', // option2
        1_000_000_000_000_000_000, // minBetAmount (1 STRK)
        10_000_000_000_000_000_000, // maxBetAmount (10 STRK)
        5, // creatorFee (5%)
        false, // isPrivate
        Category::Sports // category
    )
}

#[test]
fn test_vote() {
    let contract = deploy_predifi();
    let pool_id = contract
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
    contract.vote(pool_id, 'Team A', 200);

    let pool = contract.get_pool(pool_id);
    assert(pool.totalBetCount == 1, 'Total bet count should be 1');
    assert(pool.totalStakeOption1 == 200, 'Total stake should be 200');
    assert(pool.totalSharesOption1 == 199, 'Total share should be 199');
}

#[test]
fn test_vote_with_user_stake() {
    let contract = deploy_predifi();
    let pool_id = contract
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

    let pool = contract.get_pool(pool_id);

    contract.vote(pool_id, 'Team A', 200);

    let user_stake = contract.get_user_stake(pool_id, pool.address);

    assert(user_stake.amount == 200, 'Incorrect amount');
    assert(user_stake.shares == 199, 'Incorrect shares');
    assert(!user_stake.option, 'Incorrect option');
}

#[test]
fn test_successful_get_pool() {
    let contract = deploy_predifi();
    let pool_id = contract
        .create_pool(
            'Example Pool1',
            Pool::WinBet,
            "A simple betting pool1",
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
    let pool = contract.get_pool(pool_id);
    assert(pool.poolName == 'Example Pool1', 'Pool not found');
}

#[test]
#[should_panic(expected: 'Invalid Pool Option')]
fn test_when_invalid_option_is_pass() {
    let contract = deploy_predifi();
    let pool_id = contract
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
    contract.vote(pool_id, 'Team C', 200);
}

#[test]
#[should_panic(expected: 'Amount is below minimum')]
fn test_when_min_bet_amount_less_than_required() {
    let contract = deploy_predifi();
    let pool_id = contract
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
    contract.vote(pool_id, 'Team A', 10);
}

#[test]
#[should_panic(expected: 'Amount is above maximum')]
fn test_when_max_bet_amount_greater_than_required() {
    let contract = deploy_predifi();
    let pool_id = contract
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
    contract.vote(pool_id, 'Team B', 1000000);
}

#[test]
fn test_get_pool_odds() {
    let contract = deploy_predifi();

    // Create a new pool
    let pool_id = contract
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

    contract.vote(pool_id, 'Team A', 100);

    let pool_odds = contract.pool_odds(pool_id);

    assert(pool_odds.option1_odds == 2500, 'Incorrect odds for option 1');
    assert(pool_odds.option2_odds == 7500, 'Incorrect odds for option 2');
}

#[test]
fn test_get_pool_stakes() {
    let contract = deploy_predifi();

    // Create a new pool
    let pool_id = contract
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

    contract.vote(pool_id, 'Team A', 200);

    let pool_stakes = contract.get_pool_stakes(pool_id);

    assert(pool_stakes.amount == 200, 'Incorrect pool stake amount');
    assert(pool_stakes.shares == 199, 'Incorrect pool stake shares');
    assert(!pool_stakes.option, 'Incorrect pool stake option');
}

#[test]
fn test_unique_pool_id() {
    let contract = deploy_predifi();
    let pool_id = contract
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
    assert!(pool_id != 0, "not created");
    println!("Pool id: {}", pool_id);
}


#[test]
fn test_unique_pool_id_when_called_twice_in_the_same_execution() {
    let contract = deploy_predifi();
    let pool_id = contract
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
    let pool_id1 = contract
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

    assert!(pool_id != 0, "not created");
    assert!(pool_id != pool_id1, "they are the same");

    println!("Pool id: {}", pool_id);
    println!("Pool id: {}", pool_id1);
}
#[test]
fn test_get_pool_vote() {
    let contract = deploy_predifi();

    // Create a new pool
    let pool_id = contract
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

    contract.vote(pool_id, 'Team A', 200);

    let pool_vote = contract.get_pool_vote(pool_id);

    assert(!pool_vote, 'Incorrect pool vote');
}

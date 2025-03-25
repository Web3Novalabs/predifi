use contract::base::types::{Category, Pool, PoolDetails, Status};
use contract::interfaces::ierc721::IERC721MintableDispatcher;
use contract::interfaces::ipredifi::{IPredifiDispatcher, IPredifiDispatcherTrait};
use core::felt252;
use core::traits::Into;
use openzeppelin::token::erc721::interface::{IERC721Dispatcher, IERC721DispatcherTrait};
use snforge_std::{
    ContractClassTrait, DeclareResultTrait, declare, start_cheat_caller_address,
    stop_cheat_caller_address,
};
use starknet::{
    ClassHash, ContractAddress, get_block_timestamp, get_caller_address, get_contract_address,
};

fn owner() -> ContractAddress {
    'owner'.try_into().unwrap()
}

fn deploy_predifi() -> IPredifiDispatcher {
    let contract_class = declare("Predifi").unwrap().contract_class();

    let (contract_address, _) = contract_class.deploy(@array![].into()).unwrap();
    (IPredifiDispatcher { contract_address })
}

const ONE_STRK: u256 = 1_000_000_000_000_000_000;

#[test]
fn test_create_pool() {
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
}

#[test]
#[should_panic(expected: "Start time must be before lock time")]
fn test_invalid_time_sequence_start_after_lock() {
    let contract = deploy_predifi();
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

    contract
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
}

#[test]
#[should_panic(expected: "Minimum bet must be greater than 0")]
fn test_zero_min_bet() {
    let contract = deploy_predifi();
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

    contract
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
}

#[test]
#[should_panic(expected: "Creator fee cannot exceed 5%")]
fn test_excessive_creator_fee() {
    let contract = deploy_predifi();
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

    contract
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
            6,
            isPrivate,
            category,
        );
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
// #[test]
// fn test_nft_minting_during_pool_creation() {
//     let contract = deploy_predifi();
//     let contract_address = contract.contract_address;
//     let nft_dispatcher = IERC721Dispatcher { contract_address };

//     // Create a pool which should mint an NFT
//     let pool_id = contract.create_pool(
//         'NFT Test Pool',
//         Pool::WinBet,
//         "A pool for NFT testing",
//         "nft_image.png",
//         "nft_event.com/details",
//         1710000000,
//         1710003600,
//         1710007200,
//         'Option A',
//         'Option B',
//         100,
//         10000,
//         5,
//         false,
//         Category::Sports,
//     );

//     // Verify NFT was minted to pool creator
//     let owner = nft_dispatcher.owner_of(pool_id);
//     assert(owner == contract_address.into(), 'NFT should be owned by contract creator');

//     // Verify NFT balance
//     let balance = nft_dispatcher.balance_of(contract_address.into());
//     assert(balance == 1, 'Contract creator should own 1 NFT');
// }

// #[test]
// #[should_panic(expected: "NFTs are non-transferable")]
// fn test_nft_non_transferable() {
//     let contract = deploy_predifi();
//     let contract_address = contract.contract_address;
//     let nft_dispatcher = IERC721Dispatcher { contract_address };
//     let user = starknet::contract_address_const::<0x123>();

//     // Create a pool to get an NFT
//     let pool_id = contract.create_pool(
//         'Transfer Test Pool',
//         Pool::WinBet,
//         "A pool for transfer testing",
//         "transfer_image.png",
//         "transfer_event.com/details",
//         1710000000,
//         1710003600,
//         1710007200,
//         'Option A',
//         'Option B',
//         100,
//         10000,
//         5,
//         false,
//         Category::Sports,
//     );

//     // Attempt to transfer the NFT (should panic)
//     nft_dispatcher.transfer_from(contract_address, user, pool_id);
// }

// #[test]
// fn test_multiple_nft_ownership() {
//     let contract = deploy_predifi();
//     let contract_address = contract.contract_address;
//     let nft_dispatcher = IERC721Dispatcher { contract_address };

//     // Create multiple pools to get multiple NFTs
//     let mut pool_ids = ArrayTrait::new();
//     for i in 0..3_u8 {
//         let pool_id = contract.create_pool(
//             'Multiple NFT Pool',
//             Pool::WinBet,
//             "A pool for multiple NFT testing",
//             "multiple_image.png",
//             "multiple_event.com/details",
//             1710000000,
//             1710003600,
//             1710007200,
//             'Option A',
//             'Option B',
//             100,
//             10000,
//             5,
//             false,
//             Category::Sports,
//         );
//         pool_ids.append(pool_id);
//     }

//     // Verify contract creator owns all NFTs
//     let balance = nft_dispatcher.balance_of(contract_address);
//     assert(balance == 3, 'Contract creator should own 3 NFTs');

//     // Verify ownership of each NFT
//     for i in 0..3_u8 {
//         let owner = nft_dispatcher.owner_of(*pool_ids.get(i).unwrap());
//         assert(owner == contract_address, 'NFT should be owned by contract creator');
//     }
// }

// #[test]
// #[should_panic(expected: "NFTs are non-transferable")]
// fn test_safe_transfer_from_failure() {
//     let contract = deploy_predifi();
//     let contract_address = contract.contract_address;
//     let nft_dispatcher = IERC721Dispatcher { contract_address };
//     let user = starknet::contract_address_const::<0x123>();

//     // Create a pool to get an NFT
//     let pool_id = contract.create_pool(
//         'Safe Transfer Test Pool',
//         Pool::WinBet,
//         "A pool for safe transfer testing",
//         "safe_transfer_image.png",
//         "safe_transfer_event.com/details",
//         1710000000,
//         1710003600,
//         1710007200,
//         'Option A',
//         'Option B',
//         100,
//         10000,
//         5,
//         false,
//         Category::Sports,
//     );

//     // Attempt safe transfer (should panic)
//     nft_dispatcher.safe_transfer_from(contract_address, user, pool_id, ArrayTrait::new().span());
// }



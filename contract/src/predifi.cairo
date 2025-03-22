#[starknet::contract]
pub mod Predifi {
    // Cairo imports
    use starknet::storage::{
        Map, StorageMapReadAccess, StorageMapWriteAccess, StoragePointerReadAccess,
        StoragePointerWriteAccess,
    };
    use starknet::{ContractAddress, get_block_timestamp, get_caller_address};
    use crate::base::errors::Errors::{
        AMOUNT_ABOVE_MAXIMUM, AMOUNT_BELOW_MINIMUM, INACTIVE_POOL, INVALID_POOL_OPTION,
    };
    // oz imports

    // package imports
    use crate::base::types::{Category, Pool, PoolDetails, PoolOdds, Status, UserStake};
    use crate::interfaces::ipredifi::IPredifi;

    // 1 STRK in WEI
    const ONE_STRK: u256 = 1_000_000_000_000_000_000;

    #[storage]
    struct Storage {
        pools: Map<u256, PoolDetails>, // pool id to pool details struct
        pool_count: u256, // number of pools available totally
        pool_odds: Map<u256, PoolOdds>,
        pool_vote: Map<u256, bool>, // pool id to vote
        user_stakes: Map<(u256, ContractAddress), UserStake> // Mapping user -> stake details
    }

    // Events
    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        BetPlaced: BetPlaced,
    }

    #[derive(Drop, starknet::Event)]
    struct BetPlaced {
        pool_id: u256,
        address: ContractAddress,
        option: felt252,
        amount: u256,
        shares: u256,
    }

    #[constructor]
    fn constructor(ref self: ContractState) {}

    #[abi(embed_v0)]
    impl predifi of IPredifi<ContractState> {
        fn create_pool(
            ref self: ContractState,
            poolName: felt252,
            poolType: Pool,
            poolDescription: ByteArray,
            poolImage: ByteArray,
            poolEventSourceUrl: ByteArray,
            poolStartTime: u64,
            poolLockTime: u64,
            poolEndTime: u64,
            option1: felt252,
            option2: felt252,
            minBetAmount: u256,
            maxBetAmount: u256,
            creatorFee: u8,
            isPrivate: bool,
            category: Category,
        ) -> u256 {
            // Validation checks
            assert!(poolStartTime < poolLockTime, "Start time must be before lock time");
            assert!(poolLockTime < poolEndTime, "Lock time must be before end time");
            assert!(minBetAmount > 0, "Minimum bet must be greater than 0");
            assert!(
                maxBetAmount >= minBetAmount, "Max bet must be greater than or equal to min bet",
            );
            let current_time = get_block_timestamp();
            assert!(current_time < poolStartTime, "Start time must be in the future");
            assert!(creatorFee <= 100, "Creator fee cannot exceed 100%");

            // Collect pool creation fee (1 STRK)
            self.collect_pool_creation_fee(get_caller_address());

            // Generate new pool ID
            let pool_id = self.pool_count.read() + 1;

            // Create pool details structure
            let creator_address = get_caller_address();
            let pool_details = PoolDetails {
                pool_id,
                address: creator_address,
                poolName,
                poolType,
                poolDescription,
                poolImage,
                poolEventSourceUrl,
                createdTimeStamp: current_time,
                poolStartTime,
                poolLockTime,
                poolEndTime,
                option1,
                option2,
                minBetAmount,
                maxBetAmount,
                creatorFee,
                status: Status::Active,
                isPrivate,
                category,
                totalBetAmountStrk: 0_u256,
                totalBetCount: 0_u8,
                totalStakeOption1: 0_u256,
                totalStakeOption2: 0_u256,
                totalSharesOption1: 0_u256,
                totalSharesOption2: 0_u256,
                initial_share_price: 5000 // 0.5 in basis points (10000 = 1.0)
            };

            self.pools.write(pool_id, pool_details);
            self.pool_count.write(pool_id);

            let initial_odds = PoolOdds {
                option1_odds: 5000, // 0.5 in decimal (5000/10000)
                option2_odds: 5000,
                option1_probability: 5000, // 50% probability
                option2_probability: 5000,
                implied_probability1: 5000,
                implied_probability2: 5000,
            };

            self.pool_odds.write(pool_id, initial_odds);

            pool_id
        }

        fn pool_count(self: @ContractState) -> u256 {
            self.pool_count.read()
        }

        fn pool_odds(self: @ContractState, pool_id: u256) -> PoolOdds {
            self.pool_odds.read(pool_id)
        }

        fn get_pool(self: @ContractState, pool_id: u256) -> PoolDetails {
            self.pools.read(pool_id)
        }

        fn vote(ref self: ContractState, pool_id: u256, option: felt252, amount: u256) {
            let pool = self.pools.read(pool_id);
            let option1: felt252 = pool.option1;
            let option2: felt252 = pool.option2;
            assert(option == option1 || option == option2, INVALID_POOL_OPTION);
            assert(pool.status == Status::Active, INACTIVE_POOL);
            assert(amount >= pool.minBetAmount, AMOUNT_BELOW_MINIMUM);
            assert(amount <= pool.maxBetAmount, AMOUNT_ABOVE_MAXIMUM);

            let mut pool = self.pools.read(pool_id);
            if option == option1 {
                pool.totalStakeOption1 += amount;
                pool
                    .totalSharesOption1 +=
                        calculate_shares(amount, pool.totalStakeOption1, pool.totalStakeOption2);
            } else {
                pool.totalStakeOption2 += amount;
                pool
                    .totalSharesOption2 +=
                        calculate_shares(amount, pool.totalStakeOption2, pool.totalStakeOption1);
            }
            pool.totalBetAmountStrk += amount;
            pool.totalBetCount += 1;

            // Update pool odds
            let odds = calculate_odds(pool.totalStakeOption1, pool.totalStakeOption2);
            self.pool_odds.write(pool_id, odds);

            let shares: u256 = calculate_shares(
                amount, pool.totalStakeOption1, pool.totalStakeOption2,
            );
            // Store user stake
            let user_stake = UserStake {
                pool_id,
                option: option == option2,
                amount,
                shares,
                timestamp: get_block_timestamp(),
            };
            let address = get_caller_address();
            self.user_stakes.write((pool_id, address), user_stake);
            self.pools.write(pool_id, pool);
            self.pool_vote.write(pool_id, option == option2);
            // Emit event
            self.emit(Event::BetPlaced(BetPlaced { pool_id, address, option, amount, shares }));
        }

        fn get_user_stake(
            self: @ContractState, pool_id: u256, address: ContractAddress,
        ) -> UserStake {
            self.user_stakes.read((pool_id, address))
        }
    }

    #[generate_trait]
    impl Private of PrivateTrait {
        fn collect_pool_creation_fee(
            ref self: ContractState, creator: ContractAddress,
        ) { // TODO: Uncomment code after ERC20 implementation
        // let strk_token = IErc20Dispatcher { contract_address: self.strk_token_address.read() };
        // strk_token.transfer_from(creator, get_contract_address(), ONE_STRK);
        }
    }

    // Helper functions

    fn calculate_shares(
        amount: u256, total_stake_selected_option: u256, total_stake_other_option: u256,
    ) -> u256 {
        let total_pool_amount = total_stake_selected_option + total_stake_other_option;
        let shares = (amount * total_pool_amount) / (total_stake_selected_option * 2);
        shares
    }

    fn calculate_odds(total_stake_option1: u256, total_stake_option2: u256) -> PoolOdds {
        let total_pool_amount = total_stake_option1 + total_stake_option2;
        let option1_odds = (total_stake_option2 * 10000) / total_pool_amount;
        let option2_odds = (total_stake_option1 * 10000) / total_pool_amount;
        PoolOdds {
            option1_odds,
            option2_odds,
            option1_probability: option1_odds,
            option2_probability: option2_odds,
            implied_probability1: option1_odds,
            implied_probability2: option2_odds,
        }
    }
}

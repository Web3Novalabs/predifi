#[starknet::contract]
pub mod Predifi {
    use crate::base::types::{PoolDetails, Pool, Category, Status, UserStake};
    use crate::interfaces::ipredifi::IPredifi;
    use starknet::get_caller_address;
    use starknet::get_block_timestamp;
    use starknet::ContractAddress;
    use core::starknet::storage::{
        Map, StoragePathEntry, StoragePointerReadAccess, StoragePointerWriteAccess,
    };
    use core::array::ArrayTrait;

    use openzeppelin::token::erc20::interface::{IERC20Dispatcher, IERC20DispatcherTrait};

    #[storage]
    struct Storage {
        pools: Map<u256, PoolDetails>,
        pool_count: u256,
        pool_vote: Map<u256, bool>,
        user_stakes: Map<ContractAddress, UserStake> // Mapping user -> stake details
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
        ) -> bool {
            let pool_id = self.pool_count.read() + 1;
            self.pool_count.write(pool_id);

            // Validation checks
            assert!(poolStartTime < poolLockTime, "Start time must be before lock time");
            assert!(poolLockTime < poolEndTime, "Lock time must be before end time");
            assert!(minBetAmount > 0, "Minimum bet must be greater than 0");
            assert!(
                maxBetAmount >= minBetAmount, "Max bet must be greater than or equal to min bet",
            );

            let new_pool = PoolDetails {
                pool_id,
                address: get_caller_address(),
                poolName: poolName,
                poolType: poolType,
                poolDescription: poolDescription,
                poolImage: poolImage,
                poolEventSourceUrl: poolEventSourceUrl,
                createdTimeStamp: get_block_timestamp(),
                poolStartTime: poolStartTime,
                poolLockTime: poolLockTime,
                poolEndTime: poolEndTime,
                option1,
                option2,
                minBetAmount,
                maxBetAmount,
                creatorFee,
                status: Status::Active,
                isPrivate,
                category,
                totalBetAmountStrk: 0,
                totalBetCount: 0,
                totalStakeOption1: 0,
                totalStakeOption2: 0,
                totalSharesOption1: 0,
                totalSharesOption2: 0,
                initial_share_price: 100,
            };

            self.pools.entry(pool_id).write(new_pool);
            true
        }

        fn get_all_pools(self: @ContractState) -> Array<PoolDetails> {
            let mut pool_array = array![];
            let pool_count = self.pool_count.read();
            let mut i: u256 = 1;

            loop {
                if i > pool_count {
                    break;
                }

                let pool = self.pools.entry(i).read();
                pool_array.append(pool);

                i += 1;
            };

            pool_array
        }


        fn vote_in_pool(
            ref self: ContractState,
            pool_id: u256,
            amount: u256,
            option: u8,
            recepient: ContractAddress,
            token: ContractAddress,
        ) -> bool {
            // Check for double voting
            let has_voted = self.pool_vote.entry(pool_id).read();
            assert(has_voted == false, 'User has already voted');
            // Check for option validity (must be 1 or 2)
            assert(option == 1 || option == 2, 'Invalid option');
            // Ensure the pool exists
            let current_count = self.pool_count.read();
            assert(pool_id <= current_count, 'Pool ID does not exist');
            // Retrieve the pool data
            let mut pool = self.pools.entry(pool_id).read();
            // Ensure the pool is still active
            assert(pool.poolEndTime >= get_block_timestamp(), 'Pool has ended');
            assert(pool.poolLockTime >= get_block_timestamp(), 'Pool is locked');
            // Ensure the stake amount is within limits
            assert(
                (amount >= pool.minBetAmount) && (amount <= pool.maxBetAmount),
                'Invalid stake amount',
            );
            // Ensure pool is active before voting
            assert(pool.status == Status::Active, 'Pool is not active');
            // Token transfer from user to contract
            let success = IERC20Dispatcher { contract_address: token }
                .transfer_from(get_caller_address(), recepient, amount);
            assert(success, 'Token transfer failed');
            // Share Calculation:
            let mut shares = 0;
            if option == 1 {
                pool.totalStakeOption1 += amount;
                shares = self
                    .calculate_user_shares(
                        amount,
                        pool.totalStakeOption1,
                        pool.totalSharesOption1,
                        pool.initial_share_price,
                    );
                pool.totalSharesOption1 += shares;
            } else {
                pool.totalStakeOption2 += amount;
                shares = self
                    .calculate_user_shares(
                        amount,
                        pool.totalStakeOption2,
                        pool.totalSharesOption2,
                        pool.initial_share_price,
                    );
                pool.totalSharesOption2 += shares;
            }
            // Update pool statistics
            pool.totalBetCount += 1;
            pool.totalBetAmountStrk += amount;
            // Update Pool Storage
            self.pools.entry(pool_id).write(pool);
            // Mark user as having voted
            self.pool_vote.entry(pool_id).write(true);
            // Update UserSake Map
            self.update_user_stake(get_caller_address(), amount, shares, option);
            true
        }

        // Function to update user stake
        fn update_user_stake(
            ref self: ContractState, user: ContractAddress, amount: u256, shares: u256, option: u8,
        ) {
            let mut user_stake = self.user_stakes.entry(user).read();
            user_stake.amount = amount;
            user_stake.shares = shares;
            user_stake.option = option;

            // Store user stake data
            self.user_stakes.entry(user).write(user_stake);
        }

        // Function to get user stake details
        fn get_user_stake(ref self: ContractState, user: ContractAddress) -> UserStake {
            let mut stake = self.user_stakes.entry(user).read();
            stake
        }

        // Open to change when a proper Logic is decided retuning dummy data for now
        fn calculate_user_shares(
            ref self: ContractState,
            user_stake: u256,
            total_stake: u256,
            total_shares: u256,
            odds: u16,
        ) -> u256 {
            // let pool_weight: u256 = 5; // Represents 0.5 in fixed-point (5/10)
            // if total_shares == 0 {
            //     // First user gets shares based on initial odds
            //     return (user_stake * pool_weight / odds);
            // } else {
            //     // Adjust based on odds & total stake
            //     let adjusted_stake = user_stake * odds;
            //     return (adjusted_stake * total_shares / total_stake);
            // }

            50
        }
    }

    #[generate_trait]
    impl Private of PrivateTrait {}
}

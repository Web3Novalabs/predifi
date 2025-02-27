#[starknet::contract]
pub mod Predifi {
    use crate::base::types::{PoolDetails, Pool, Category, Status};
    use crate::interfaces::ipredifi::IPredifi;
    use starknet::get_caller_address;
    use starknet::get_block_timestamp;
    use core::starknet::storage::{
        Map, StoragePathEntry, StoragePointerReadAccess, StoragePointerWriteAccess,
    };
    use core::array::ArrayTrait;


    #[storage]
    struct Storage {
        pools: Map<u256, PoolDetails>,
        pool_count: u256,
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
                
                // Leer el pool usando el método entry().read()
                let pool = self.pools.entry(i).read();
                pool_array.append(pool);
                
                i += 1;
            };
            
            pool_array
        }
    }


    #[generate_trait]
    impl Private of PrivateTrait {}
}

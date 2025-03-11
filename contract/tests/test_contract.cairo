#[cfg(test)]
pub mod tests {
    use super::*;
    use starknet::testing::{deploy_contract, assert};

    #[test]
    fn test_get_pool_by_id() {
        let mut contract = deploy_contract::<PoolContract>();

        let pool = PoolDetails {
            pool_id: 1_u256,
            address: ContractAddress::default(),
            poolName: 'TestPool',
            poolType: Pool::Single,
            poolDescription: ByteArray::new(),
            poolImage: ByteArray::new(),
            poolEventSourceUrl: ByteArray::new(),
            createdTimeStamp: 1700000000_u64,
            poolStartTime: 1700001000_u64,
            poolLockTime: 1700002000_u64,
            poolEndTime: 1700003000_u64,
            option1: 'Option 1',
            option2: 'Option 2',
            minBetAmount: 100_u256,
            maxBetAmount: 1000_u256,
            creatorFee: 5_u8,
            status: Status::Open,
            isPrivate: false,
            category: Category::Sports,
            totalBetAmountStrk: 0_u256,
            totalBetCount: 0_u8,
            totalStakeOption1: 0_u256,
            totalStakeOption2: 0_u256,
            totalSharesOption1: 0_u256,
            totalSharesOption2: 0_u256,
            initial_share_price: 100_u16,
        };

        contract.create_pool(pool.clone());

        let result = contract.get_pool_by_id(1_u256);
        assert(result == pool, "Pool details should match");

        // Test pool not found
        assert_panic(|| contract.get_pool_by_id(2_u256));
    }
}

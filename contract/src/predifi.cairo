#[starknet::contract]
pub mod Predifi {
    // Cairo imports
    use starknet::{ContractAddress, get_block_timestamp, get_caller_address};
    use starknet::storage::{
        Map, StorageMapReadAccess, StorageMapWriteAccess, StoragePointerReadAccess,
        StoragePointerWriteAccess,
    };
    // oz imports

    // package imports
    use crate::base::types::{PoolDetails, PoolOdds, Pool, Category, Status, UserStake};
    use crate::interfaces::ipredifi::IPredifi;
    use crate::presets::NonTransferableNFT::NonTransferableNFT::{
        INonTransferableNFTDispatcher, INonTransferableNFTDispatcherTrait,
    };


    // 1 STRK in WEI
    const ONE_STRK: u256 = 1_000_000_000_000_000_000;

    #[storage]
    struct Storage {
        pools: Map<u256, PoolDetails>, // pool id to pool details struct
        pool_count: u256, // number of pools available totally
        pool_odds: Map<u256, PoolOdds>,
        pool_vote: Map<u256, bool>, // pool id to vote
        user_stakes: Map<ContractAddress, UserStake>, // Mapping user -> stake details
        nft_contract: ContractAddress // Address of the NFT contract
    }

    #[constructor]
    fn constructor(ref self: ContractState, nft_contract: ContractAddress) {
        self.nft_contract.write(nft_contract);
    }

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

            let nft_contract = self.nft_contract.read();
            let nft = INonTransferableNFTDispatcher { contract_address: nft_contract };
            nft.mint(pool_id);

            pool_id
        }

        fn pool_count(self: @ContractState) -> u256 {
            self.pool_count.read()
        }

        fn pool_odds(self: @ContractState, pool_id: u256) -> PoolOdds {
            self.pool_odds.read(pool_id)
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
}

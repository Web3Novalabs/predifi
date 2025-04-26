#[starknet::contract]
pub mod Predifi {
    // Cairo imports
    use core::hash::{HashStateExTrait, HashStateTrait};
    use core::pedersen::PedersenTrait;
    use core::poseidon::PoseidonTrait;
    // oz imports
    use openzeppelin::access::accesscontrol::AccessControlComponent;
    use openzeppelin::introspection::src5::SRC5Component;
    use starknet::storage::{
        Map, MutableVecTrait, StorageMapReadAccess, StorageMapWriteAccess, StoragePointerReadAccess,
        StoragePointerWriteAccess, Vec, VecTrait,
    };
    use starknet::{ContractAddress, get_block_timestamp, get_caller_address, get_contract_address};
    use crate::base::errors::Errors::{
        AMOUNT_ABOVE_MAXIMUM, AMOUNT_BELOW_MINIMUM, INACTIVE_POOL, INVALID_POOL_OPTION,
    };

    // package imports
    use crate::base::types::{Category, Pool, PoolDetails, PoolOdds, Status, UserStake};
    use crate::interfaces::ipredifi::IPredifi;

    // 1 STRK in WEI
    const ONE_STRK: u256 = 1_000_000_000_000_000_000;

    // 200 PREDIFI TOKEN in WEI
    const MIN_STAKE_AMOUNT: u256 = 200_000_000_000_000_000_000;

    // Validator role
    const VALIDATOR_ROLE: felt252 = selector!("VALIDATOR_ROLE");

    // components definition
    component!(path: AccessControlComponent, storage: accesscontrol, event: AccessControlEvent);
    component!(path: SRC5Component, storage: src5, event: SRC5Event);

    // AccessControl
    #[abi(embed_v0)]
    impl AccessControlImpl =
        AccessControlComponent::AccessControlImpl<ContractState>;
    impl AccessControlInternalImpl = AccessControlComponent::InternalImpl<ContractState>;

    // SRC5
    #[abi(embed_v0)]
    impl SRC5Impl = SRC5Component::SRC5Impl<ContractState>;

    #[storage]
    pub struct Storage {
        pools: Map<u256, PoolDetails>, // pool id to pool details struct
        pool_count: u256, // number of pools available totally
        pool_odds: Map<u256, PoolOdds>,
        pool_stakes: Map<u256, UserStake>,
        pool_vote: Map<u256, bool>, // pool id to vote
        user_stakes: Map<(u256, ContractAddress), UserStake>, // Mapping user -> stake details
        #[substorage(v0)]
        pub accesscontrol: AccessControlComponent::Storage,
        #[substorage(v0)]
        src5: SRC5Component::Storage,
        validators: Vec<ContractAddress>,
        user_hash_poseidon: felt252,
        user_hash_pedersen: felt252,
        nonce: felt252,
    }

    // Events
    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        BetPlaced: BetPlaced,
        UserStaked: UserStaked,
        PoolStateUpdated: PoolStateUpdated,
        #[flat]
        AccessControlEvent: AccessControlComponent::Event,
        #[flat]
        SRC5Event: SRC5Component::Event,
    }

    #[derive(Drop, starknet::Event)]
    struct BetPlaced {
        pool_id: u256,
        address: ContractAddress,
        option: felt252,
        amount: u256,
        shares: u256,
    }
    #[derive(Drop, starknet::Event)]
    struct UserStaked {
        pool_id: u256,
        address: ContractAddress,
        amount: u256,
    }

    #[derive(Drop, starknet::Event)]
    struct PoolStateUpdated {
        pool_id: u256,
        previous_state: Status,
        new_state: Status,
        updated_by: ContractAddress,
        timestamp: u64,
    }

    #[derive(Drop, Hash)]
    struct HashingProperties {
        username: felt252,
        password: felt252,
    }

    #[derive(Drop, Hash)]
    struct Hashed {
        id: felt252,
        login: HashingProperties,
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
            assert!(creatorFee <= 5, "Creator fee cannot exceed 5%");

            // Collect pool creation fee (1 STRK)
            self.collect_pool_creation_fee(get_caller_address());

            let mut pool_id = self.generate_deterministic_number();

            // While a pool with this pool_id already exists, generate a new one.
            while self.retrieve_pool(pool_id) {
                pool_id = self.generate_deterministic_number();
            }

            // Create pool details structure
            let creator_address = get_caller_address();
            let pool_details = PoolDetails {
                pool_id: pool_id,
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
                initial_share_price: 5000, // 0.5 in basis points (10000 = 1.0)
                exists: true,
            };

            self.pools.write(pool_id, pool_details);

            let initial_odds = PoolOdds {
                option1_odds: 5000, // 0.5 in decimal (5000/10000)
                option2_odds: 5000,
                option1_probability: 5000, // 50% probability
                option2_probability: 5000,
                implied_probability1: 5000,
                implied_probability2: 5000,
            };

            self.pool_odds.write(pool_id, initial_odds);

            // Add to pool count
            self.pool_count.write(self.pool_count.read() + 1);

            pool_id
        }

        fn pool_count(self: @ContractState) -> u256 {
            self.pool_count.read()
        }

        fn get_pool_creator(self: @ContractState, pool_id: u256) -> ContractAddress {
            let pool = self.pools.read(pool_id);
            pool.address
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
                    .totalSharesOption1 += self
                    .calculate_shares(amount, pool.totalStakeOption1, pool.totalStakeOption2);
            } else {
                pool.totalStakeOption2 += amount;
                pool
                    .totalSharesOption2 += self
                    .calculate_shares(amount, pool.totalStakeOption2, pool.totalStakeOption1);
            }
            pool.totalBetAmountStrk += amount;
            pool.totalBetCount += 1;

            // Update pool odds
            let odds = self
                .calculate_odds(pool.pool_id, pool.totalStakeOption1, pool.totalStakeOption2);
            self.pool_odds.write(pool_id, odds);

            // Calculate the user's shares
            let shares: u256 = if option == option1 {
                self.calculate_shares(amount, pool.totalStakeOption1, pool.totalStakeOption2)
            } else {
                self.calculate_shares(amount, pool.totalStakeOption2, pool.totalStakeOption1)
            };

            // Store user stake
            let user_stake = UserStake {
                option: option == option2,
                amount: amount,
                shares: shares,
                timestamp: get_block_timestamp(),
            };
            let address: ContractAddress = get_caller_address();
            self.user_stakes.write((pool.pool_id, address), user_stake);
            self.pool_vote.write(pool.pool_id, option == option2);
            self.pool_stakes.write(pool.pool_id, user_stake);
            self.pools.write(pool.pool_id, pool);
            // Emit event
            self.emit(Event::BetPlaced(BetPlaced { pool_id, address, option, amount, shares }));
        }

        fn stake(ref self: ContractState, pool_id: u256, amount: u256) {
            assert(amount >= MIN_STAKE_AMOUNT, 'stake amount too low');
            let address: ContractAddress = get_caller_address();
            // Add to previous stake if any
            let mut stake = self.user_stakes.read((pool_id, address));
            stake.amount = amount + stake.amount;
            // write the new stake
            self.user_stakes.write((pool_id, address), stake);
            // grant the validator role
            self.accesscontrol._grant_role(VALIDATOR_ROLE, address);
            // add caller to validator list
            self.validators.append().write(address);
            // emit event
            self.emit(UserStaked { pool_id, address, amount });
        }

        fn get_user_stake(
            self: @ContractState, pool_id: u256, address: ContractAddress,
        ) -> UserStake {
            self.user_stakes.read((pool_id, address))
        }
        fn get_pool_stakes(self: @ContractState, pool_id: u256) -> UserStake {
            self.pool_stakes.read(pool_id)
        }

        fn get_pool_vote(self: @ContractState, pool_id: u256) -> bool {
            self.pool_vote.read(pool_id)
        }
        fn get_pool_count(self: @ContractState) -> u256 {
            self.pool_count.read()
        }


        fn retrieve_pool(self: @ContractState, pool_id: u256) -> bool {
            let pool = self.pools.read(pool_id);
            pool.exists
        }

        fn get_creator_fee_percentage(self: @ContractState, pool_id: u256) -> u8 {
            let pool = self.pools.read(pool_id);
            pool.creatorFee
        }

        fn get_validator_fee_percentage(self: @ContractState, pool_id: u256) -> u8 {
            10_u8
        }

        fn update_pool_state(ref self: ContractState, pool_id: u256) -> Status {
            // Get the pool details
            let pool = self.pools.read(pool_id);
            assert(pool.exists, 'Pool does not exist');

            // Get current timestamp
            let current_time = get_block_timestamp();

            // Store previous state for event emission
            let previous_state = pool.status;
            let new_state = match pool.status {
                Status::Active => {
                    if current_time >= pool.poolLockTime {
                        Status::Locked
                    } else {
                        Status::Active
                    }
                },
                Status::Locked => {
                    if current_time >= pool.poolEndTime {
                        Status::Settled
                    } else {
                        Status::Locked
                    }
                },
                Status::Settled => {
                    if current_time >= pool.poolEndTime + 86400 {
                        Status::Closed
                    } else {
                        Status::Settled
                    }
                },
                Status::Closed => Status::Closed,
            };

            // If state has changed, update storage and emit event
            if new_state != previous_state {
                // Create a new PoolDetails with updated status
                let mut updated_pool = pool;
                updated_pool.status = new_state;
                self.pools.write(pool_id, updated_pool);

                // Emit state change event
                self
                    .emit(
                        PoolStateUpdated {
                            pool_id,
                            previous_state,
                            new_state,
                            updated_by: get_caller_address(),
                            timestamp: current_time,
                        },
                    );
            }

            new_state
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


        /// Generates a deterministic `u256` with 6 decimal places.
        /// Combines block number, timestamp, and sender address for uniqueness.

        fn generate_deterministic_number(ref self: ContractState) -> u256 {
            let nonce: felt252 = self.nonce.read();
            let nonci: felt252 = self.save_user_with_pedersen(nonce);
            // Increment the nonce and update storage.
            self.nonce.write(nonci);

            let username: felt252 = get_contract_address().into();
            let id: felt252 = get_caller_address().into();
            let password: felt252 = nonce.into();
            let login = HashingProperties { username, password };
            let user = Hashed { id, login };

            let poseidon_hash: felt252 = PoseidonTrait::new().update_with(user).finalize();
            self.user_hash_poseidon.write(poseidon_hash);

            // Convert poseidon_hash from felt252 to u256.
            let hash_as_u256: u256 = poseidon_hash.try_into().unwrap();

            // Define divisor for 6 digits: 1,000,000.
            let divisor: u256 = 1000000;

            // Calculate quotient and remainder manually.
            let quotient: u256 = hash_as_u256 / divisor;
            let remainder: u256 = hash_as_u256 - quotient * divisor;

            remainder
        }


        fn save_user_with_pedersen(ref self: ContractState, salt: felt252) -> felt252 {
            let username: felt252 = salt;
            let id: felt252 = get_caller_address().into();
            let password: felt252 = get_block_timestamp().into();
            let login = HashingProperties { username, password };
            let user = Hashed { id, login };

            let pedersen_hash = PedersenTrait::new(0).update_with(user).finalize();

            self.user_hash_pedersen.write(pedersen_hash);
            pedersen_hash
        }


        fn calculate_shares(
            ref self: ContractState,
            amount: u256,
            total_stake_selected_option: u256,
            total_stake_other_option: u256,
        ) -> u256 {
            let total_pool_amount = total_stake_selected_option + total_stake_other_option;

            if total_stake_selected_option == 0 {
                return amount;
            }

            let shares = (amount * total_pool_amount) / (total_stake_selected_option + 1);
            shares
        }

        fn calculate_odds(
            ref self: ContractState,
            pool_id: u256,
            total_stake_option1: u256,
            total_stake_option2: u256,
        ) -> PoolOdds {
            // Fetch the current pool odds
            let current_pool_odds = self.pool_odds.read(pool_id);

            // If no current pool odds exist, use the initial odds (5000 for both options)
            let initial_odds = 5000; // 0.5 in decimal (5000/10000)
            let current_option1_odds = if current_pool_odds.option1_odds == 0 {
                initial_odds
            } else {
                current_pool_odds.option1_odds
            };
            let current_option2_odds = if current_pool_odds.option2_odds == 0 {
                initial_odds
            } else {
                current_pool_odds.option2_odds
            };

            // Calculate the total pool amount
            let total_pool_amount = total_stake_option1 + total_stake_option2;

            // If no stakes are placed, return the current pool odds
            if total_pool_amount == 0 {
                return PoolOdds {
                    option1_odds: current_option1_odds,
                    option2_odds: current_option2_odds,
                    option1_probability: current_option1_odds,
                    option2_probability: current_option2_odds,
                    implied_probability1: 10000 / current_option1_odds,
                    implied_probability2: 10000 / current_option2_odds,
                };
            }

            // Calculate the new odds based on the stakes
            let new_option1_odds = (total_stake_option2 * 10000) / total_pool_amount;
            let new_option2_odds = (total_stake_option1 * 10000) / total_pool_amount;

            // update the new odds with the current odds (weighted average)
            let option1_odds = (current_option1_odds + new_option1_odds) / 2;
            let option2_odds = (current_option2_odds + new_option2_odds) / 2;

            // Calculate probabilities
            let option1_probability = option1_odds;
            let option2_probability = option2_odds;

            // Calculate implied probabilities
            let implied_probability1 = 10000 / option1_odds;
            let implied_probability2 = 10000 / option2_odds;

            // Return the updated PoolOdds struct
            PoolOdds {
                option1_odds: option1_odds,
                option2_odds: option2_odds,
                option1_probability,
                option2_probability,
                implied_probability1,
                implied_probability2,
            }
        }
    }
}

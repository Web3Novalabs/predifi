#[starknet::contract]
pub mod Predifi {
    // Cairo imports
    use core::hash::{HashStateExTrait, HashStateTrait};
    use core::pedersen::PedersenTrait;
    use core::poseidon::PoseidonTrait;
    use starknet::storage::{
        Map, StorageMapReadAccess, StorageMapWriteAccess, StoragePointerReadAccess,
        StoragePointerWriteAccess,
    };
    use starknet::{
        ContractAddress, get_block_number, get_block_timestamp, get_caller_address,
        get_contract_address, class_hash::ClassHash,
    };
    use crate::base::errors::Errors::{
        AMOUNT_ABOVE_MAXIMUM, AMOUNT_BELOW_MINIMUM, INACTIVE_POOL, INVALID_POOL_OPTION,
    };
    // oz imports
    use openzeppelin::upgrades::{interface::IUpgradeable, UpgradeableComponent};

    //components
    component!(path: UpgradeableComponent, storage: upgradeable, event: UpgradeableEvent);

    // package imports
    use crate::base::types::{
        Category, Pool, PoolDetails, PoolOdds, Status, UserStake, ValidatorData,
    };
    use crate::interfaces::ipredifi::IPredifi;

    // 1 STRK in WEI
    const ONE_STRK: u256 = 1_000_000_000_000_000_000;

    #[storage]
    struct Storage {
        pools: Map<u256, PoolDetails>, // pool id to pool details struct
        pool_count: u256, // number of pools available totally
        pool_odds: Map<u256, PoolOdds>,
        pool_stakes: Map<u256, UserStake>,
        pool_vote: Map<u256, bool>, // pool id to vote
        user_stakes: Map<(u256, ContractAddress), UserStake>, // Mapping user -> stake details
        user_hash_poseidon: felt252,
        user_hash_pedersen: felt252,
        nonce: felt252,
        validators: Map<ContractAddress, ValidatorData>,
        validators_index: Map<ContractAddress, u256>, // Tracks validator's position in list
        validators_list: Map<u256, ContractAddress>, // Index -> Address
        validators_count: u256,
        pool_validators: Map<(u256, u256), ContractAddress>, // (pool_id, index) -> Address
        pool_validator_count: Map<u256, u256>,
        version: u8,
        #[substorage(v0)]
        upgradeable: UpgradeableComponent::Storage,
    }

    // Events
    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        BetPlaced: BetPlaced,
        ValidatorRegistered: ValidatorRegistered,
        UserStaked: UserStaked,
        #[flat]
        AccessControlEvent: AccessControlComponent::Event,
        #[flat]
        SRC5Event: SRC5Component::Event,
        #[flat]
        UpgradeableEvent: UpgradeableComponent::Event,
    }

    #[derive(Drop, starknet::Event)]
    struct BetPlaced {
        pool_id: u256,
        address: ContractAddress,
        option: felt252,
        amount: u256,
        shares: u256,
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

    #[derive(Drop, starknet::Event)]
    struct ValidatorRegistered {
        validator: ContractAddress,
        stake: u256,
    }

    #[derive(Drop, starknet::Event)]
    struct UserStaked {
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
            assert!(creatorFee <= 5, "Creator fee cannot exceed 5%");
            assert(self.validators_count.read() >= 10, 'Need 10+ validators');

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

            self.select_validators(pool_id);

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
                option: option == option2, amount, shares, timestamp: get_block_timestamp(),
            };
            let address: ContractAddress = get_caller_address();
            self.user_stakes.write((pool.pool_id, address), user_stake);
            self.pool_vote.write(pool.pool_id, option == option2);
            self.pool_stakes.write(pool.pool_id, user_stake);
            self.pools.write(pool.pool_id, pool);
            // Emit event
            self.emit(Event::BetPlaced(BetPlaced { pool_id, address, option, amount, shares }));
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

        fn register_validator(ref self: ContractState, stake: u256) {
            let caller = get_caller_address();
            assert(stake > 0, 'Stake must be positive');

            // Read existing validator data
            let mut data = self.validators.read(caller);

            // Check if already registered using status flag
            let is_new_validator = !data.status;

            // Update validator data
            data.status = true;
            data.preodifiTokenAmount += stake;
            self.validators.write(caller, data);

            // Only add to list if new validator
            if is_new_validator {
                let count = self.validators_count.read();
                self.validators_list.write(count, caller);
                self.validators_index.write(caller, count);
                self.validators_count.write(count + 1_u256);
            }
        }


        fn verify_validator_assignment(
            self: @ContractState, pool_id: u256, validator: ContractAddress,
        ) -> bool {
            let count = self.pool_validator_count.read(pool_id);
            let mut i = 0;
            loop {
                if i >= count {
                    break false;
                }
                if self.pool_validators.read((pool_id, i)) == validator {
                    break true;
                }
                i += 1;
            }
        }

        fn get_validators_count(self: @ContractState) -> u256 {
            self.validators_count.read()
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

        fn select_validators(ref self: ContractState, pool_id: u256) {
            let total_validators = self.validators_count.read();
            assert(total_validators >= 10_u256, 'Insufficient validators');

            // Create a temporary array of validator indexes
            let mut validator_indexes = ArrayTrait::<u256>::new();
            let mut i: u256 = 0_u256;
            while i < total_validators {
                validator_indexes.append(i);
                i += 1_u256;
            }

            // Fisher-Yates shuffle implementation
            let seed = self.generate_strong_seed(pool_id);
            let mut rng_state = seed;
            let mut selected_count: u256 = 0_u256;
            let mut selected_validators = ArrayTrait::<ContractAddress>::new();

            while selected_count < 10_u256 {
                // Generate random index using remaining length
                let remaining = total_validators - selected_count;
                let rand_num = self.prng(rng_state) % remaining;
                rng_state = self.prng(rng_state);

                // Get validator address directly from storage
                let validator_index = validator_indexes
                    .at((selected_count + rand_num).try_into().unwrap());
                let validator_addr = self.validators_list.read(*validator_index);

                if self.validators.read(validator_addr).status {
                    selected_validators.append(validator_addr);
                    selected_count += 1_u256;
                }
            }

            // Write selected validators to storage
            let mut count: u256 = 0_u256;
            while count < selected_count {
                self
                    .pool_validators
                    .write((pool_id, count), *selected_validators.at(count.try_into().unwrap()));
                count += 1_u256;
            }

            self.pool_validator_count.write(pool_id, selected_count);
        }

        fn generate_strong_seed(ref self: ContractState, pool_id: u256) -> u256 {
            let nonce = self.nonce.read();
            let block_number = get_block_number();
            let timestamp = get_block_timestamp();
            let caller = get_caller_address();

            // Build Pedersen hash chain
            let mut hasher = PedersenTrait::new(0);
            hasher = hasher.update_with(pool_id);
            hasher = hasher.update_with(block_number);
            hasher = hasher.update_with(timestamp);
            hasher = hasher.update_with(caller);
            hasher = hasher.update_with(nonce);

            hasher.finalize().try_into().unwrap()
        }

        fn prng(ref self: ContractState, input: u256) -> u256 {
            let mut hasher = PoseidonTrait::new();
            hasher = hasher.update_with(input);
            hasher.finalize().try_into().unwrap()
        }
    }

    #[abi(embed_v0)]
    impl UpgradeableImpl of IUpgradeable<ContractState> {
        fn upgrade(ref self: ContractState, new_class_hash: ClassHash) {
            self.version.write(self.version.read() + 1);
            self.upgradeable.upgrade(new_class_hash);
        }
    }

    impl UpgradeableInternalImpl = UpgradeableComponent::InternalImpl<ContractState>;
}
#[starknet::contract]
pub mod Predifi {
    // Cairo imports
    use core::hash::{HashStateExTrait, HashStateTrait};
    use core::pedersen::PedersenTrait;
    use core::poseidon::PoseidonTrait;
    // oz imports
    use openzeppelin::access::accesscontrol::AccessControlComponent;
    use openzeppelin::introspection::src5::SRC5Component;
    use openzeppelin::token::erc20::interface::{
        ERC20ABIDispatcher, ERC20ABIDispatcherTrait, IERC20Dispatcher, IERC20DispatcherTrait,
        IERC20MetadataDispatcher, IERC20MetadataDispatcherTrait,
    };
    use starknet::storage::{
        Map, MutableVecTrait, StorageMapReadAccess, StorageMapWriteAccess, StoragePointerReadAccess,
        StoragePointerWriteAccess, Vec, VecTrait,
    };
    use starknet::{
        ContractAddress, contract_address_const, get_block_timestamp, get_caller_address,
        get_contract_address,
    };
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
    const ADMIN_ROLE: felt252 = selector!("ADMIN_ROLE");

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
        token_addr: ContractAddress,
        #[substorage(v0)]
        pub accesscontrol: AccessControlComponent::Storage,
        #[substorage(v0)]
        src5: SRC5Component::Storage,
        validators: Vec<ContractAddress>,
        user_hash_poseidon: felt252,
        user_hash_pedersen: felt252,
        nonce: felt252,
        protocol_treasury: u256,
        creator_treasuries: Map<ContractAddress, u256>,
        validator_fee: Map<u256, u256>,
        validator_treasuries: Map<
            ContractAddress, u256,
        >, // Validator address to their accumulated fees
        pool_outcomes: Map<
            u256, bool,
        >, // Pool ID to outcome (true = option2 won, false = option1 won)
        pool_resolved: Map<u256, bool>,
        // Track which pools a validator is assigned to
        validator_assignments: Map<(ContractAddress, u256), bool>,
    }

    // Events
    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        BetPlaced: BetPlaced,
        UserStaked: UserStaked,
        FeesCollected: FeesCollected,
        PoolStateTransition: PoolStateTransition,
        PoolResolved: PoolResolved,
        FeeWithdrawn: FeeWithdrawn,
        ValidatorsAssigned: ValidatorsAssigned,
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
    struct FeesCollected {
        fee_type: felt252,
        pool_id: u256,
        recipient: ContractAddress,
        amount: u256,
    }

    #[derive(Drop, starknet::Event)]
    struct PoolStateTransition {
        pool_id: u256,
        previous_status: Status,
        new_status: Status,
        timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    struct PoolResolved {
        pool_id: u256,
        winning_option: bool,
        total_payout: u256,
    }

    #[derive(Drop, starknet::Event)]
    struct FeeWithdrawn {
        fee_type: felt252,
        recipient: ContractAddress,
        amount: u256,
    }

    #[derive(Drop, starknet::Event)]
    struct ValidatorsAssigned {
        pool_id: u256,
        validator1: ContractAddress,
        validator2: ContractAddress,
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
    fn constructor(
        ref self: ContractState,
        token_addr: ContractAddress,
        validator: ContractAddress,
        admin: ContractAddress,
    ) {
        self.token_addr.write(token_addr);
        self.accesscontrol._grant_role(ADMIN_ROLE, admin);
        self.accesscontrol._grant_role(VALIDATOR_ROLE, validator);
    }

    #[abi(embed_v0)]
    impl predifi of IPredifi<ContractState> {
        // Get validators assigned to a specific pools
        fn get_pool_validators(
            self: @ContractState, pool_id: u256,
        ) -> (ContractAddress, ContractAddress) {
            let pool_details = self.pools.read(pool_id);
            (pool_details.assigned_validator1, pool_details.assigned_validator2)
        }


        fn grant_validator_role(ref self: ContractState, validator: ContractAddress) {
            // Only admin can grant validator role
            let caller = get_caller_address();
            assert(self.accesscontrol.has_role(ADMIN_ROLE, caller), 'Caller not admin');
            // // Grant validator role
        // self.accesscontrol.grant_role(VALIDATOR_ROLE, validator);
        }


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

            let creator_address = get_caller_address();

            // Collect pool creation fee (1 STRK)
            self.collect_pool_creation_fee(creator_address);

            let mut pool_id = self.generate_deterministic_number();

            // While a pool with this pool_id already exists, generate a new one.
            while self.retrieve_pool(pool_id) {
                pool_id = self.generate_deterministic_number();
            }

            // Select two random validators
            let (validator1, validator2) = self.select_random_validators();

            // Create pool details structure
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
                assigned_validator1: validator1,
                assigned_validator2: validator2,
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

        /// This can be called by anyone to update the state of a pool
        fn update_pool_state(ref self: ContractState, pool_id: u256) -> Status {
            let pool = self.pools.read(pool_id);
            assert(pool.exists, 'Pool does not exist');

            let current_status = pool.status;
            let current_time = get_block_timestamp();
            let mut new_status = current_status;

            // Determine the new status based on current time and pool timestamps
            if current_time >= pool.poolEndTime {
                if current_status == Status::Active || current_status == Status::Locked {
                    new_status = Status::Settled;
                } else if current_status == Status::Settled
                    && current_time >= (pool.poolEndTime + 86400) {
                    new_status = Status::Closed;
                }
            } else if current_time >= pool.poolLockTime && current_status == Status::Active {
                new_status = Status::Locked;
            }

            // Only update if there's a change in status
            if new_status != current_status {
                // Update the pool status
                let mut updated_pool = pool;
                updated_pool.status = new_status;
                self.pools.write(pool_id, updated_pool);

                // Emit event for the state transition
                let transition_event = PoolStateTransition {
                    pool_id, previous_status: current_status, new_status, timestamp: current_time,
                };
                self.emit(Event::PoolStateTransition(transition_event));
            }

            // Return the (potentially updated) status
            if new_status != current_status {
                new_status
            } else {
                current_status
            }
        }

        /// Manually update the state of a pool - can only be called by admin or validator
        fn manually_update_pool_state(
            ref self: ContractState, pool_id: u256, new_status: Status,
        ) -> Status {
            let pool = self.pools.read(pool_id);
            assert(pool.exists, 'Pool does not exist');

            // Check if caller has appropriate role (admin or validator)
            let caller = get_caller_address();
            let is_admin = self.accesscontrol.has_role(ADMIN_ROLE, caller);
            let is_validator = self.accesscontrol.has_role(VALIDATOR_ROLE, caller);
            assert(is_admin || is_validator, 'Caller not authorized');

            // Enforce status transition rules
            let current_status = pool.status;

            // Don't update if status is the same
            if new_status == current_status {
                return current_status;
            }

            // Check for invalid transitions
            let is_valid_transition = if is_admin {
                !(current_status == Status::Locked && new_status == Status::Active)
                    && !(current_status == Status::Settled
                        && (new_status == Status::Active || new_status == Status::Locked))
                    && !(current_status == Status::Closed)
            } else {
                // Active -> Locked -> Settled -> Closed
                (current_status == Status::Active && new_status == Status::Locked)
                    || (current_status == Status::Locked && new_status == Status::Settled)
                    || (current_status == Status::Settled && new_status == Status::Closed)
            };

            assert(is_valid_transition, 'Invalid state transition');

            // Update the pool status
            let mut updated_pool = pool;
            updated_pool.status = new_status;
            self.pools.write(pool_id, updated_pool);

            // Emit event for the manual state transition
            let current_time = get_block_timestamp();
            let transition_event = PoolStateTransition {
                pool_id, previous_status: current_status, new_status, timestamp: current_time,
            };
            self.emit(Event::PoolStateTransition(transition_event));

            new_status
        }

        fn vote(ref self: ContractState, pool_id: u256, option: felt252, amount: u256) {
            let pool = self.pools.read(pool_id);
            let option1: felt252 = pool.option1;
            let option2: felt252 = pool.option2;
            assert(option == option1 || option == option2, INVALID_POOL_OPTION);
            assert(pool.status == Status::Active, INACTIVE_POOL);
            assert(amount >= pool.minBetAmount, AMOUNT_BELOW_MINIMUM);
            assert(amount <= pool.maxBetAmount, AMOUNT_ABOVE_MAXIMUM);

            // Transfer betting amount from the user to the contract
            let caller = get_caller_address();
            let dispatcher = IERC20Dispatcher { contract_address: self.token_addr.read() };

            // Check balance and allowance
            let user_balance = dispatcher.balance_of(caller);
            assert(user_balance >= amount, 'Insufficient balance');

            let contract_address = get_contract_address();
            let allowed_amount = dispatcher.allowance(caller, contract_address);
            assert(allowed_amount >= amount, 'Insufficient allowance');

            // Transfer the tokens
            dispatcher.transfer_from(caller, contract_address, amount);

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

            // Transfer stake amount from user to contract
            let dispatcher = IERC20Dispatcher { contract_address: self.token_addr.read() };

            // Check balance and allowance
            let user_balance = dispatcher.balance_of(address);
            assert(user_balance >= amount, 'Insufficient balance');

            let contract_address = get_contract_address();
            let allowed_amount = dispatcher.allowance(address, contract_address);
            assert(allowed_amount >= amount, 'Insufficient allowance');

            // Transfer the tokens
            dispatcher.transfer_from(address, contract_address, amount);

            // Add to previous stake if any
            let mut stake = self.user_stakes.read((pool_id, address));
            stake.amount = amount + stake.amount;
            // write the new stake
            self.user_stakes.write((pool_id, address), stake);
            // grant the validator role
            self.accesscontrol._grant_role(VALIDATOR_ROLE, address);
            // add caller to validator list
            self.validators.push(address);
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
        fn retrieve_validator_fee(self: @ContractState, pool_id: u256) -> u256 {
            self.validator_fee.read(pool_id)
        }

        fn get_validator_fee_percentage(self: @ContractState, pool_id: u256) -> u8 {
            10_u8
        }

        fn collect_pool_creation_fee(ref self: ContractState, creator: ContractAddress) {
            // Retrieve the STRK token contract
            let strk_token = IERC20Dispatcher { contract_address: self.token_addr.read() };

            // Check if the creator has sufficient balance for pool creation fee
            let creator_balance = strk_token.balance_of(creator);
            assert(creator_balance >= ONE_STRK, 'Insufficient STRK balance');

            // Check allowance to ensure the contract can transfer tokens
            let contract_address = get_contract_address();
            let allowed_amount = strk_token.allowance(creator, contract_address);
            assert(allowed_amount >= ONE_STRK, 'Insufficient allowance');

            // Transfer the pool creation fee from creator to the contract
            strk_token.transfer_from(creator, contract_address, ONE_STRK);
        }

        fn calculate_validator_fee(
            ref self: ContractState, pool_id: u256, total_amount: u256,
        ) -> u256 {
            // Validator fee is fixed at 10%
            let validator_fee_percentage = 5_u8;
            let mut validator_fee = (total_amount * validator_fee_percentage.into()) / 100_u256;

            self.validator_fee.write(pool_id, validator_fee);
            validator_fee
        }

        // Helper function to distribute validator fees evenly
        fn distribute_validator_fees(ref self: ContractState, pool_id: u256) {
            let total_validator_fee = self.validator_fee.read(pool_id);

            let validator_count = self.validators.len();

            // Convert validator_count to u256 for the division
            let validator_count_u256: u256 = validator_count.into();
            let fee_per_validator = total_validator_fee / validator_count_u256;

            let strk_token = IERC20Dispatcher { contract_address: self.token_addr.read() };

            // Distribute to each validator
            let mut i: u64 = 0;
            while i < validator_count {
                // Add debug info to trace the exact point of failure

                // Safe access to validator - check bounds first
                if i < self.validators.len() {
                    let validator_address = self.validators.at(i).read();
                    strk_token.transfer(validator_address, fee_per_validator);
                } else {}
                i += 1;
            }
            // Reset the validator fee for this pool after distribution
            self.validator_fee.write(pool_id, 0);
        }

        fn add_validators(
            ref self: ContractState,
            validator1: ContractAddress,
            validator2: ContractAddress,
            validator3: ContractAddress,
            validator4: ContractAddress,
        ) -> Array<ContractAddress> {
            // Initialize empty array
            let mut validators = array![];
            // Append each validator to the array
            self.validators.push(validator1);
            self.validators.push(validator2);
            self.validators.push(validator3);
            self.validators.push(validator4);

            validators
        }
    }

    #[generate_trait]
    impl Private of PrivateTrait {
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


        fn select_random_validators(ref self: ContractState) -> (ContractAddress, ContractAddress) {
            let validators_len = self.validators.len();

            // Handle edge cases where there are less than 2 validators
            if validators_len == 0 {
                let zero_address: ContractAddress = contract_address_const::<0>();
                return (zero_address, zero_address);
            }

            if validators_len == 1 {
                let validator = self.validators.at(0).read();
                let zero_address: ContractAddress = contract_address_const::<0>();
                self.update_validator_assignments(validator, self.pool_count.read());
                return (validator, zero_address);
            }

            // Generate two different random indices
            let timestamp = get_block_timestamp();
            let nonce: u64 = self.nonce.read().try_into().unwrap();

            // Update nonce for next random selection
            self.nonce.write(nonce.into() + 1);

            // Use timestamp and nonce to create pseudo-random indices
            let random_seed: u64 = timestamp + nonce;
            // Convert validators_len to u64 for modulo operation
            let validators_len_u64: u64 = validators_len.into();
            let random_index1: u64 = random_seed % validators_len_u64;

            // Ensure second index is different from the first
            let mut random_index2: u64 = (random_seed + 1) % validators_len_u64;
            if random_index1 == random_index2 && validators_len > 1 {
                random_index2 = (random_index2 + 1) % validators_len_u64;
            }

            let validator1 = self.validators.at(random_index1).read();
            let validator2 = self.validators.at(random_index2).read();

            // Update validator assignments
            self.update_validator_assignments(validator1, self.pool_count.read());
            self.update_validator_assignments(validator2, self.pool_count.read());

            return (validator1, validator2);
        }

        /// Update validator assignments mapping
        fn update_validator_assignments(
            ref self: ContractState, validator: ContractAddress, pool_id: u256,
        ) {
            // Skip updating for zero address
            let zero_address: ContractAddress = contract_address_const::<0>();
            if validator == zero_address {
                return;
            }

            self.validator_assignments.write((validator, pool_id), true);
        }
    }
}

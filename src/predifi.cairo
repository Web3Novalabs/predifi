#[starknet::contract]
pub mod Predifi {
    // Cairo imports
    use core::hash::{HashStateExTrait, HashStateTrait};
    use core::panic_with_felt252;
    use core::pedersen::PedersenTrait;
    use core::poseidon::PoseidonTrait;
    // oz imports
    use openzeppelin::access::accesscontrol::{AccessControlComponent, DEFAULT_ADMIN_ROLE};
    use openzeppelin::introspection::src5::SRC5Component;
    use openzeppelin::security::{PausableComponent, ReentrancyGuardComponent};
    use openzeppelin::token::erc20::interface::{IERC20Dispatcher, IERC20DispatcherTrait};
    use openzeppelin::upgrades::UpgradeableComponent;
    use starknet::storage::{
        Map, MutableVecTrait, StorageMapReadAccess, StorageMapWriteAccess, StoragePointerReadAccess,
        StoragePointerWriteAccess, Vec, VecTrait,
    };
    use starknet::{
        ClassHash, ContractAddress, get_block_timestamp, get_caller_address, get_contract_address,
    };
    use crate::base::errors::Errors;
    use crate::base::events::Events::{
        BetPlaced, ContractPaused, ContractUnpaused, ContractUpgraded, CreatorFeesCollected,
        DisputeRaised, DisputeResolved, DisputeThresholdUpdated, EmergencyActionCancelled,
        EmergencyActionExecuted, EmergencyActionScheduled, EmergencyWithdrawal, FeeWithdrawn,
        FeesCollected, PoolAutomaticallySettled, PoolCancelled, PoolCreated,
        PoolCreationFeeCollected, PoolEmergencyFrozen, PoolEmergencyResolved,
        PoolEmergencyUnfrozen, PoolResolved, PoolStateTransition, PoolSuspended,
        ProtocolFeesCollected, StakeRefunded, UserStaked, ValidatorAdded,
        ValidatorConfirmationsUpdated, ValidatorFeesDistributed, ValidatorRemoved,
        ValidatorResultSubmitted, ValidatorsAssigned,
    };
    use crate::base::security::{Security, SecurityTrait};

    // package imports
    use crate::base::types::{
        CategoryType, EmergencyAction, EmergencyActionStatus, EmergencyActionType,
        EmergencyPoolState, PoolDetails, PoolOdds, Status, UserStake, u8_to_category, u8_to_pool,
        u8_to_status,
    };
    use crate::interfaces::ipredifi::{IPredifi, IPredifiDispute, IPredifiValidator};


    // 1 STRK in WEI
    const ONE_STRK: u256 = 1_000_000_000_000_000_000;

    // 200 PREDIFI TOKEN in WEI
    const MIN_STAKE_AMOUNT: u256 = 200_000_000_000_000_000_000;

    // Validator role
    const VALIDATOR_ROLE: felt252 = selector!("VALIDATOR_ROLE");

    // components definition
    component!(path: AccessControlComponent, storage: accesscontrol, event: AccessControlEvent);
    component!(path: SRC5Component, storage: src5, event: SRC5Event);
    component!(path: PausableComponent, storage: pausable, event: PausableEvent);
    component!(path: UpgradeableComponent, storage: upgradeable, event: UpgradeableEvent);
    component!(
        path: ReentrancyGuardComponent, storage: reentrancy_guard, event: ReentrancyGuardEvent,
    );


    // AccessControl
    #[abi(embed_v0)]
    impl AccessControlImpl =
        AccessControlComponent::AccessControlImpl<ContractState>;
    impl AccessControlInternalImpl = AccessControlComponent::InternalImpl<ContractState>;

    // SRC5
    #[abi(embed_v0)]
    impl SRC5Impl = SRC5Component::SRC5Impl<ContractState>;

    // Pausable
    #[abi(embed_v0)]
    impl PausableImpl = PausableComponent::PausableImpl<ContractState>;
    impl PausableInternalImpl = PausableComponent::InternalImpl<ContractState>;

    // Upgradeable
    impl UpgradeableInternalImpl = UpgradeableComponent::InternalImpl<ContractState>;

    // SecurityTrait Implementation
    impl SecurityImpl = Security<ContractState>;

    impl InternalImpl = ReentrancyGuardComponent::InternalImpl<ContractState>;

    #[storage]
    /// @notice Storage struct for the Predifi contract.
    /// @dev Holds all pools, user stakes, odds, roles, and protocol parameters.
    pub struct Storage {
        pools: Map<u256, PoolDetails>, // pool id to pool details struct
        pool_ids: Vec<u256>,
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
        pub validators: Vec<ContractAddress>,
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
        user_pools: Map<
            (ContractAddress, u256), bool,
        >, // Mapping (user, pool_id) -> has_participated
        user_pool_count: Map<
            ContractAddress, u256,
        >, // Tracks how many pools each user has participated in
        user_participated_pools: Map<
            (ContractAddress, u256), bool,
        >, // Maps (user, pool_id) to participation status
        user_pool_ids: Map<(ContractAddress, u256), u256>, // Maps (user, index) -> pool_id
        user_pool_ids_count: Map<
            ContractAddress, u256,
        >, // Tracks how many pool IDs are stored for each user
        // Mapping to track which validators are assigned to which pools
        pool_validator_assignments: Map<u256, (ContractAddress, ContractAddress)>,
        // Dispute functionality storage
        pool_dispute_users: Map<(u256, ContractAddress), bool>,
        pool_dispute_count: Map<u256, u256>,
        pool_previous_status: Map<u256, Status>,
        dispute_threshold: u256,
        // strg strc for Validator confirmation and validation results
        pool_validator_confirmations: Map<
            (u256, ContractAddress), bool,
        >, // (pool_id, validator) -> has_validated
        pool_validation_results: Map<
            (u256, ContractAddress), bool,
        >, // (pool_id, validator) -> selected_option
        pool_validation_count: Map<u256, u256>, // pool_id -> number_of_validations
        pool_final_outcome: Map<
            u256, bool,
        >, // pool_id -> final_outcome (true = option2, false = option1)
        required_validator_confirmations: u256, // Number of validators needed to settle a pool
        // Emergency functionality storage
        emergency_pool_states: Map<u256, EmergencyPoolState>, // pool_id -> emergency state
        scheduled_emergency_actions: Map<u256, EmergencyAction>, // action_id -> emergency action
        emergency_action_counter: u256, // Counter for generating unique action IDs
        emergency_timelock_delay: u64, // Minimum delay for emergency actions (default: 24 hours)
        #[substorage(v0)]
        pausable: PausableComponent::Storage,
        #[substorage(v0)]
        upgradeable: UpgradeableComponent::Storage,
        #[substorage(v0)]
        reentrancy_guard: ReentrancyGuardComponent::Storage,
    }

    /// @notice Events emitted by the Predifi contract.
    #[event]
    #[derive(Drop, starknet::Event)]
    pub enum Event {
        /// @notice Emitted when a bet is placed.
        BetPlaced: BetPlaced,
        /// @notice Emitted when a user stakes tokens.
        UserStaked: UserStaked,
        /// @notice Emitted when a user's stake is refunded.
        StakeRefunded: StakeRefunded,
        /// @notice Emitted when protocol or creator fees are collected.
        FeesCollected: FeesCollected,
        /// @notice Emitted when a pool changes state.
        PoolStateTransition: PoolStateTransition,
        /// @notice Emitted when a pool is resolved.
        PoolResolved: PoolResolved,
        /// @notice Emitted when a fee is withdrawn.
        FeeWithdrawn: FeeWithdrawn,
        /// @notice Emitted when validators are assigned to a pool.
        ValidatorsAssigned: ValidatorsAssigned,
        /// @notice Emitted when a validator is added.
        ValidatorAdded: ValidatorAdded,
        /// @notice Emitted when a validator is removed.
        ValidatorRemoved: ValidatorRemoved,
        /// @notice Emitted when a dispute is raised.
        DisputeRaised: DisputeRaised,
        /// @notice Emitted when a dispute is resolved.
        DisputeResolved: DisputeResolved,
        /// @notice Emitted when a pool is suspended.
        PoolSuspended: PoolSuspended,
        /// @notice Emitted when a pool is cancelled.
        PoolCancelled: PoolCancelled,
        /// @notice Emitted when a validator submits a result.
        ValidatorResultSubmitted: ValidatorResultSubmitted,
        /// @notice Emitted when a pool is automatically settled.
        PoolAutomaticallySettled: PoolAutomaticallySettled,
        // Emergency Events
        /// @notice Emitted when an emergency withdrawal is made.
        EmergencyWithdrawal: EmergencyWithdrawal,
        /// @notice Emitted when a pool is frozen due to emergency.
        PoolEmergencyFrozen: PoolEmergencyFrozen,
        /// @notice Emitted when a pool is unfrozen from emergency state.
        PoolEmergencyUnfrozen: PoolEmergencyUnfrozen,
        /// @notice Emitted when a pool is resolved through emergency resolution.
        PoolEmergencyResolved: PoolEmergencyResolved,
        /// @notice Emitted when an emergency action is scheduled.
        EmergencyActionScheduled: EmergencyActionScheduled,
        /// @notice Emitted when a scheduled emergency action is executed.
        EmergencyActionExecuted: EmergencyActionExecuted,
        /// @notice Emitted when a scheduled emergency action is cancelled.
        EmergencyActionCancelled: EmergencyActionCancelled,
        // Configuration Events
        /// @notice Emitted when validator confirmations requirement is updated.
        ValidatorConfirmationsUpdated: ValidatorConfirmationsUpdated,
        /// @notice Emitted when dispute threshold is updated.
        DisputeThresholdUpdated: DisputeThresholdUpdated,
        // Pool Lifecycle Events
        /// @notice Emitted when a new pool is created.
        PoolCreated: PoolCreated,
        // Contract Management Events
        /// @notice Emitted when the contract is paused.
        ContractPaused: ContractPaused,
        /// @notice Emitted when the contract is unpaused.
        ContractUnpaused: ContractUnpaused,
        /// @notice Emitted when the contract is upgraded.
        ContractUpgraded: ContractUpgraded,
        // Fee Collection Events
        /// @notice Emitted when protocol fees are collected.
        ProtocolFeesCollected: ProtocolFeesCollected,
        /// @notice Emitted when creator fees are collected.
        CreatorFeesCollected: CreatorFeesCollected,
        /// @notice Emitted when validator fees are distributed.
        ValidatorFeesDistributed: ValidatorFeesDistributed,
        /// @notice Emitted when pool creation fee is collected.
        PoolCreationFeeCollected: PoolCreationFeeCollected,
        #[flat]
        AccessControlEvent: AccessControlComponent::Event,
        #[flat]
        SRC5Event: SRC5Component::Event,
        #[flat]
        PausableEvent: PausableComponent::Event,
        #[flat]
        UpgradeableEvent: UpgradeableComponent::Event,
        #[flat]
        ReentrancyGuardEvent: ReentrancyGuardComponent::Event,
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

    /// @notice Initializes the Predifi contract.
    /// @param self The contract state.
    /// @param token_addr The address of the STRK token contract.
    /// @param admin The address to be set as the admin (DEFAULT_ADMIN_ROLE).
    #[constructor]
    fn constructor(ref self: ContractState, token_addr: ContractAddress, admin: ContractAddress) {
        self.token_addr.write(token_addr);
        self.accesscontrol._grant_role(DEFAULT_ADMIN_ROLE, admin);
        self.dispute_threshold.write(3);
        self
            .required_validator_confirmations
            .write(2); // Require at least 2 validator confirmations to settle

        // Initialize emergency functionality
        self.emergency_timelock_delay.write(86400); // 24 hours in seconds
        self.emergency_action_counter.write(0);
    }

    #[abi(embed_v0)]
    impl predifi of IPredifi<ContractState> {
        /// @notice Creates a new prediction pool.
        /// @dev Validates parameters, collects pool creation fee, and assigns validators.
        /// @param poolName The name of the pool.
        /// @param poolType The type of the pool (as u8).
        /// @param poolDescription The description of the pool.
        /// @param poolImage The image URL for the pool.
        /// @param poolEventSourceUrl The event source URL.
        /// @param poolStartTime The start time of the pool.
        /// @param poolLockTime The lock time of the pool.
        /// @param poolEndTime The end time of the pool.
        /// @param option1 The first option for the pool.
        /// @param option2 The second option for the pool.
        /// @param minBetAmount The minimum bet amount.
        /// @param maxBetAmount The maximum bet amount.
        /// @param creatorFee The fee percentage for the pool creator.
        /// @param isPrivate Whether the pool is private.
        /// @param category The category of the pool.
        /// @return The unique pool ID.
        fn create_pool(
            ref self: ContractState,
            poolName: felt252,
            poolType: u8,
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
            category: u8,
        ) -> u256 {
            self.pausable.assert_not_paused();
            // Convert u8 to Pool enum with validation
            let pool_type_enum = u8_to_pool(poolType);
            let creator_address = get_caller_address();

            // Validation checks using SecurityTrait
            self.assert_valid_pool_timing(poolStartTime, poolLockTime, poolEndTime);
            self.assert_future_start_time(poolStartTime);
            self.assert_valid_bet_amounts(minBetAmount, maxBetAmount);
            self.assert_valid_creator_fee(creatorFee);
            self.assert_valid_felt252(poolName);
            self.assert_valid_felt252(option1);
            self.assert_valid_felt252(option2);

            // Collect pool creation fee (1 STRK)
            IPredifi::collect_pool_creation_fee(ref self, creator_address);

            let mut pool_id = self.generate_deterministic_number();

            // While a pool with this pool_id already exists, generate a new one.
            while self.retrieve_pool(pool_id) {
                pool_id = self.generate_deterministic_number();
            }

            // Create pool details structure
            let pool_details = PoolDetails {
                pool_id: pool_id,
                address: creator_address,
                poolName,
                poolType: pool_type_enum,
                poolDescription,
                poolImage,
                poolEventSourceUrl,
                createdTimeStamp: get_block_timestamp(),
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
                category: u8_to_category(category),
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
            self.pool_ids.push(pool_id);

            // Automatically assign validators to the pool
            self.assign_random_validators(pool_id);

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

            // Emit pool created event
            self.emit(
                Event::PoolCreated(
                    PoolCreated {
                        pool_id,
                        creator: creator_address,
                        pool_name: poolName,
                        category: CategoryType(u8_to_category(category)),
                        end_time: poolEndTime,
                        min_bet_amount: minBetAmount,
                        max_bet_amount: maxBetAmount,
                        creator_fee: creatorFee,
                        timestamp: get_block_timestamp(),
                    }
                )
            );

            // Emit pool creation fee collected event
            self.emit(
                Event::PoolCreationFeeCollected(
                    PoolCreationFeeCollected {
                        pool_id,
                        creator: creator_address,
                        amount: ONE_STRK,
                        timestamp: get_block_timestamp(),
                    }
                )
            );

            pool_id
        }

        /// @notice Cancels a pool. Only the pool creator can cancel.
        /// @param pool_id The ID of the pool to cancel.
        fn cancel_pool(ref self: ContractState, pool_id: u256) {
            self.pausable.assert_not_paused();
            self.assert_greater_than_zero(pool_id);

            let caller = get_caller_address();
            let pool = self.get_pool(pool_id);

            // Validation checks using SecurityTrait
            self.assert_pool_owner(@pool, caller);
            let mut updated_pool = pool;
            updated_pool.status = Status::Closed;

            self.pools.write(pool_id, updated_pool);

            self
                .emit(
                    Event::PoolCancelled(
                        PoolCancelled { pool_id, timestamp: get_block_timestamp() },
                    ),
                );
        }

        /// @notice Returns the total number of pools.
        /// @return The pool count.
        fn pool_count(self: @ContractState) -> u256 {
            self.pool_count.read()
        }

        /// @notice Returns the creator address of a given pool.
        /// @param pool_id The pool ID.
        /// @return The creator's contract address.
        fn get_pool_creator(self: @ContractState, pool_id: u256) -> ContractAddress {
            self.assert_greater_than_zero(pool_id);
            let pool = self.pools.read(pool_id);
            pool.address
        }

        /// @notice Returns the odds for a given pool.
        /// @param pool_id The pool ID.
        /// @return The PoolOdds struct.
        fn pool_odds(self: @ContractState, pool_id: u256) -> PoolOdds {
            self.assert_greater_than_zero(pool_id);
            self.pool_odds.read(pool_id)
        }

        /// @notice Returns the details of a given pool.
        /// @param pool_id The pool ID.
        /// @return The PoolDetails struct.
        fn get_pool(self: @ContractState, pool_id: u256) -> PoolDetails {
            self.assert_greater_than_zero(pool_id);
            self.pools.read(pool_id)
        }

        /// @notice Manually updates the state of a pool.
        /// @dev Only callable by admin or validator. Enforces valid state transitions.
        /// @param pool_id The pool ID.
        /// @param new_status The new status to set.
        /// @return The updated status.
        fn manually_update_pool_state(
            ref self: ContractState, pool_id: u256, new_status: u8,
        ) -> Status {
            self.pausable.assert_not_paused();
            self.assert_greater_than_zero(pool_id);

            let pool = self.pools.read(pool_id);

            // Validation checks using SecurityTrait
            self.assert_pool_exists(@pool);

            // Check if caller has appropriate role (admin or validator)
            let caller = get_caller_address();
            let is_admin = self.accesscontrol.has_role(DEFAULT_ADMIN_ROLE, caller);
            let is_validator = self.accesscontrol.has_role(VALIDATOR_ROLE, caller);
            assert(is_admin || is_validator, Errors::UNAUTHORIZED_CALLER);

            // Enforce status transition rules
            let current_status = pool.status;

            // Don't update if status is the same
            if u8_to_status(new_status) == current_status {
                return current_status;
            }

            // Check for invalid transitions using SecurityTrait
            self.assert_valid_state_transition(current_status, u8_to_status(new_status), is_admin);

            // Update the pool status
            let mut updated_pool = pool;
            updated_pool.status = u8_to_status(new_status);
            self.pools.write(pool_id, updated_pool);

            // Emit event for the manual state transition
            let current_time = get_block_timestamp();
            let transition_event = PoolStateTransition {
                pool_id,
                previous_status: current_status,
                new_status: u8_to_status(new_status),
                timestamp: current_time,
            };
            self.emit(Event::PoolStateTransition(transition_event));

            u8_to_status(new_status)
        }

        /// @notice Places a bet on a specific outcome in a prediction pool
        /// @dev Transfers tokens from user, updates pool odds, and validates bet constraints
        /// @param pool_id The unique identifier of the pool to bet on
        /// @param option The outcome option to bet on (Yes/No or specific outcome)
        /// @param amount The amount of tokens to bet (must be >= pool's min_bet_amount)
        /// @custom:reentrancy-guard Protected against reentrancy attacks using internal locks
        /// @custom:validation Validates pool status, betting period, and user balance
        /// @custom:odds-update Automatically recalculates pool odds after successful bet
        fn vote(ref self: ContractState, pool_id: u256, option: felt252, amount: u256) {
            self.pausable.assert_not_paused();

            // Input Validation
            self.assert_greater_than_zero(amount);
            self.assert_greater_than_zero(pool_id);
            self.assert_valid_felt252(option);

            let mut pool = self.pools.read(pool_id);
            self.assert_pool_exists(@pool);
            self.assert_pool_active(@pool);

            let option1: felt252 = pool.option1;
            let option2: felt252 = pool.option2;

            // Validation checks using SecurityTrait
            self.assert_valid_pool_option(option, option1, option2);
            self.assert_amount_within_limits(amount, pool.minBetAmount, pool.maxBetAmount);

            // Transfer betting amount from the user to the contract
            let caller = get_caller_address();
            let dispatcher = IERC20Dispatcher { contract_address: self.token_addr.read() };

            // Check balance and allowance using SecurityTrait
            let contract_address = get_contract_address();
            self.assert_sufficient_balance(dispatcher, caller, amount);
            self.assert_sufficient_allowance(dispatcher, caller, contract_address, amount);

            // Start reentrancy guard
            self.reentrancy_guard.start();

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
            self.track_user_participation(address, pool_id);

            // End reentrancy guard
            self.reentrancy_guard.end();

            // Emit event
            self.emit(Event::BetPlaced(BetPlaced { pool_id, address, option, amount, shares }));
        }

        /// @notice Stakes tokens to become eligible for validation rewards
        /// @dev Increases user's validator stake and updates protocol security parameters
        /// @param amount Amount of tokens to stake (must be > 0)
        /// @custom:compound-rewards Automatically compounds previous rewards if any exist
        /// @custom:lock-period Staked tokens subject to unbonding period for withdrawal
        /// @custom:delegation Allows delegation of stake to other validators if desired
        fn stake(ref self: ContractState, pool_id: u256, amount: u256) {
            self.pausable.assert_not_paused();
            //Input Validatioin
            self.assert_greater_than_zero(amount);
            self.assert_greater_than_zero(pool_id);

            let pool = self.pools.read(pool_id);
            self.assert_pool_exists(@pool);

            // Validation checks using SecurityTrait
            self.assert_pool_not_suspended(@pool);
            self.assert_min_stake_amount(amount);

            let address: ContractAddress = get_caller_address();

            // Transfer stake amount from user to contract
            let dispatcher = IERC20Dispatcher { contract_address: self.token_addr.read() };

            // Check balance and allowance using SecurityTrait
            let contract_address = get_contract_address();
            self.assert_sufficient_balance(dispatcher, address, amount);
            self.assert_sufficient_allowance(dispatcher, address, contract_address, amount);

            // Start reentrancy guard
            self.reentrancy_guard.start();

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
            self.track_user_participation(address, pool_id);
            // emit event
            self.emit(UserStaked { pool_id, address, amount });

            // End reentrancy guard
            self.reentrancy_guard.end();
        }


        /// @notice Refunds the user's stake for a closed pool.
        /// @param pool_id The pool ID.
        fn refund_stake(ref self: ContractState, pool_id: u256) {
            self.pausable.assert_not_paused();
            let caller = get_caller_address();

            self.assert_greater_than_zero(pool_id);
            let pool = self.get_pool(pool_id);
            self.assert_pool_exists(@pool);

            // Validation checks using SecurityTrait
            self.assert_pool_closed(@pool);

            let user_stake = self.get_user_stake(pool_id, caller);
            self.assert_non_zero_stake(user_stake.amount);

            // Start reentrancy guard

            let dispatcher = IERC20Dispatcher { contract_address: self.token_addr.read() };
            let refund_amount = user_stake.amount;

            self
                .user_stakes
                .write(
                    (pool_id, caller),
                    UserStake {
                        option: user_stake.option,
                        amount: 0,
                        shares: user_stake.shares,
                        timestamp: user_stake.timestamp,
                    },
                );

            dispatcher.transfer(caller, refund_amount);

            self
                .emit(
                    Event::StakeRefunded(
                        StakeRefunded { pool_id, address: caller, amount: user_stake.amount },
                    ),
                );

            // End reentrancy guard
            self.reentrancy_guard.end();
        }


        /// @notice Returns whether a user has participated in a specific pool.
        /// @param user The user's address.
        /// @param pool_id The pool ID.
        /// @return True if the user has participated, false otherwise.
        fn has_user_participated_in_pool(
            self: @ContractState, user: ContractAddress, pool_id: u256,
        ) -> bool {
            self.assert_non_zero_address(user);
            self.assert_greater_than_zero(pool_id);
            self.user_participated_pools.read((user, pool_id))
        }

        /// @notice Returns the number of pools a user has participated in.
        /// @param user The user's address.
        /// @return The number of pools.
        fn get_user_pool_count(self: @ContractState, user: ContractAddress) -> u256 {
            self.assert_non_zero_address(user);
            self.user_pool_count.read(user)
        }

        /// @notice Returns a list of pool IDs the user has participated in, filtered by status.
        /// @param user The user's address.
        /// @param status_filter Optional status filter.
        /// @return Array of pool IDs.
        fn get_user_pools(
            self: @ContractState, user: ContractAddress, status_filter: Option<Status>,
        ) -> Array<u256> {
            self.assert_non_zero_address(user);
            let mut result: Array<u256> = ArrayTrait::new();
            let pool_ids_count = self.user_pool_ids_count.read(user);

            // Pre-check if we have any pools to avoid gas costs on empty iterations
            if pool_ids_count == 0 {
                return result;
            }

            // Iterate through all pool IDs this user has participated in
            let mut i: u256 = 0;
            while i != pool_ids_count {
                let pool_id = self.user_pool_ids.read((user, i));

                // Only read from storage if needed
                if self.has_user_participated_in_pool(user, pool_id) {
                    // Apply status filter only if a filter is provided
                    if let Option::Some(status) = status_filter {
                        let pool = self.pools.read(pool_id);
                        if pool.exists && pool.status == status {
                            result.append(pool_id);
                        }
                    } else if self.retrieve_pool(pool_id) {
                        // No filter, just check if pool exists
                        result.append(pool_id);
                    }
                }
                i += 1;
            }

            result
        }

        /// @notice Returns a list of active pools the user has participated in.
        /// @param user The user's address.
        /// @return Array of pool IDs.
        fn get_user_active_pools(self: @ContractState, user: ContractAddress) -> Array<u256> {
            self.assert_non_zero_address(user);
            self.get_user_pools(user, Option::Some(Status::Active))
        }

        /// @notice Returns a list of locked pools the user has participated in.
        /// @param user The user's address.
        /// @return Array of pool IDs.
        fn get_user_locked_pools(self: @ContractState, user: ContractAddress) -> Array<u256> {
            self.assert_non_zero_address(user);
            self.get_user_pools(user, Option::Some(Status::Locked))
        }

        /// @notice Returns a list of settled pools the user has participated in.
        /// @param user The user's address.
        /// @return Array of pool IDs.
        fn get_user_settled_pools(self: @ContractState, user: ContractAddress) -> Array<u256> {
            self.assert_non_zero_address(user);
            self.get_user_pools(user, Option::Some(Status::Settled))
        }


        /// @notice Checks if a user has participated in a specific pool.
        /// @param user The user's address.
        /// @param pool_id The pool ID.
        /// @return True if participated, false otherwise.
        fn check_user_participated(
            self: @ContractState, user: ContractAddress, pool_id: u256,
        ) -> bool {
            self.assert_non_zero_address(user);
            self.assert_greater_than_zero(pool_id);
            self.user_pools.read((user, pool_id))
        }

        /// @notice Returns the user's stake for a given pool.
        /// @param pool_id The pool ID.
        /// @param address The user's address.
        /// @return The UserStake struct.
        fn get_user_stake(
            self: @ContractState, pool_id: u256, address: ContractAddress,
        ) -> UserStake {
            self.assert_non_zero_address(address);
            self.assert_greater_than_zero(pool_id);
            self.user_stakes.read((pool_id, address))
        }

        /// @notice Returns the stake for a given pool.
        /// @param pool_id The pool ID.
        /// @return The UserStake struct.
        fn get_pool_stakes(self: @ContractState, pool_id: u256) -> UserStake {
            self.assert_greater_than_zero(pool_id);
            self.pool_stakes.read(pool_id)
        }

        /// @notice Returns the vote for a given pool.
        /// @param pool_id The pool ID.
        /// @return True if option2, false if option1.
        fn get_pool_vote(self: @ContractState, pool_id: u256) -> bool {
            self.assert_greater_than_zero(pool_id);
            self.pool_vote.read(pool_id)
        }

        /// @notice Returns the total pool count.
        /// @return The pool count.
        fn get_pool_count(self: @ContractState) -> u256 {
            self.pool_count.read()
        }

        /// @notice Returns true if the pool exists.
        /// @param pool_id The pool ID.
        /// @return True if the pool exists.
        fn retrieve_pool(self: @ContractState, pool_id: u256) -> bool {
            self.assert_greater_than_zero(pool_id);
            let pool = self.pools.read(pool_id);
            pool.exists
        }

        /// @notice Returns the creator fee percentage for a pool.
        /// @param pool_id The pool ID.
        /// @return The creator fee percentage.
        fn get_creator_fee_percentage(self: @ContractState, pool_id: u256) -> u8 {
            self.assert_greater_than_zero(pool_id);
            let pool = self.pools.read(pool_id);
            pool.creatorFee
        }

        /// @notice Collects the pool creation fee from the creator.
        /// @param creator The creator's address.
        fn collect_pool_creation_fee(ref self: ContractState, creator: ContractAddress) {
            self.assert_non_zero_address(creator);
            // Retrieve the STRK token contract
            let strk_token = IERC20Dispatcher { contract_address: self.token_addr.read() };

            // Check pool creation fee requirements using SecurityTrait
            let contract_address = get_contract_address();
            self.assert_pool_creation_fee_requirements(strk_token, creator, contract_address);

            // Transfer the pool creation fee from creator to the contract
            strk_token.transfer_from(creator, contract_address, ONE_STRK);
        }

        /// @notice Returns all active pools.
        /// @return Array of PoolDetails.
        fn get_active_pools(self: @ContractState) -> Array<PoolDetails> {
            self.get_pools_by_status(Status::Active)
        }

        /// @notice Returns all locked pools.
        /// @return Array of PoolDetails.
        fn get_locked_pools(self: @ContractState) -> Array<PoolDetails> {
            self.get_pools_by_status(Status::Locked)
        }

        /// @notice Returns all settled pools.
        /// @return Array of PoolDetails.
        fn get_settled_pools(self: @ContractState) -> Array<PoolDetails> {
            self.get_pools_by_status(Status::Settled)
        }

        /// @notice Returns all closed pools.
        /// @return Array of PoolDetails.
        fn get_closed_pools(self: @ContractState) -> Array<PoolDetails> {
            self.get_pools_by_status(Status::Closed)
        }

        // Emergency Functions

        /// @notice Emergency withdrawal function for problematic pools.
        /// @dev Allows users to withdraw funds from pools in emergency state.
        /// @param pool_id The pool ID to withdraw from.
        fn emergency_withdraw(ref self: ContractState, pool_id: u256) {
            // Check if pool exists
            let pool = self.pools.read(pool_id);
            assert(pool.exists, Errors::POOL_DOES_NOT_EXIST);

            // Check if pool is in emergency state
            let emergency_state = self.emergency_pool_states.read(pool_id);
            assert(emergency_state.is_emergency, Errors::POOL_NOT_IN_EMERGENCY_STATE);
            assert(
                emergency_state.allow_emergency_withdrawals,
                Errors::EMERGENCY_WITHDRAWALS_NOT_ALLOWED,
            );

            let caller = get_caller_address();

            // Check if user has participated in this pool
            let has_participated = self.has_user_participated_in_pool(caller, pool_id);
            assert(has_participated, Errors::UNAUTHORIZED_CALLER);

            // Get user's stake and calculate withdrawal amount
            let user_stake = self.get_user_stake(pool_id, caller);
            assert(user_stake.amount > 0, Errors::NO_FUNDS_TO_WITHDRAW);

            // Calculate withdrawal amount (return original stake)
            let withdrawal_amount = user_stake.amount;

            // Reset user's stake
            self
                .user_stakes
                .write(
                    (pool_id, caller),
                    UserStake { amount: 0, shares: 0, option: false, timestamp: 0 },
                );

            // Transfer tokens back to user
            let strk_token = IERC20Dispatcher { contract_address: self.token_addr.read() };
            strk_token.transfer(caller, withdrawal_amount);

            // Emit emergency withdrawal event
            self
                .emit(
                    Event::EmergencyWithdrawal(
                        EmergencyWithdrawal {
                            pool_id,
                            user: caller,
                            amount: withdrawal_amount,
                            timestamp: get_block_timestamp(),
                        },
                    ),
                );
        }


        /// @notice Schedules an emergency action with timelock.
        /// @dev Only callable by admin. Schedules emergency action for execution after delay.
        /// @param action_type The type of emergency action:
        ///                    0 = FreezePool (action_data: ignored, can be 0)
        ///                    1 = ResolvePool (action_data: winning option, 0 = option1, 1 =
        ///                    option2)
        ///                    2 = UnfreezePool (action_data: ignored, can be 0)
        ///                    3 = EmergencyWithdrawal (action_data: user address as felt252)
        /// @param pool_id The pool ID for the action.
        /// @param action_data Additional data for the action. For EmergencyWithdrawal, this must be
        /// a valid user address.
        /// @return The unique action ID.
        fn schedule_emergency_action(
            ref self: ContractState, action_type: u8, pool_id: u256, action_data: felt252,
        ) -> u256 {
            // Check if caller is admin
            self.accesscontrol.assert_only_role(DEFAULT_ADMIN_ROLE);

            // Check if pool exists
            let pool = self.pools.read(pool_id);
            assert(pool.exists, Errors::POOL_DOES_NOT_EXIST);

            // Validate action type
            assert(action_type < 4, Errors::INVALID_EMERGENCY_ACTION_TYPE);

            // Validate action data based on action type
            if action_type == 3 { // EmergencyWithdrawal
                // For emergency withdrawal, action_data must contain a valid user address
                // Use the helper function to validate the address and handle potential panics
                let user_address = self.extract_user_address_from_action_data(action_data);
                // Store the validated address for later use if needed
            // Note: The validation is done here to fail fast if the address is invalid
            }

            // Generate unique action ID
            let action_id = self.emergency_action_counter.read();
            self.emergency_action_counter.write(action_id + 1);

            // Calculate execution time (current time + timelock delay)
            let current_time = get_block_timestamp();
            let execution_time = current_time + self.emergency_timelock_delay.read();

            // Create emergency action
            let emergency_action = EmergencyAction {
                action_type: self.u8_to_emergency_action_type(action_type),
                pool_id,
                action_data,
                scheduled_time: current_time,
                execution_time,
                status: EmergencyActionStatus::Waiting,
                admin: get_caller_address(),
            };

            // Store the scheduled action
            self.scheduled_emergency_actions.write(action_id, emergency_action);

            // Emit action scheduled event
            self
                .emit(
                    Event::EmergencyActionScheduled(
                        EmergencyActionScheduled {
                            action_id: action_id,
                            action_type,
                            pool_id,
                            admin: get_caller_address(),
                            execution_time,
                            timestamp: current_time,
                        },
                    ),
                );

            action_id
        }


        /// @notice Executes a scheduled emergency action after timelock delay.
        /// @dev Only callable by admin. Executes action if delay has passed.
        /// @param action_id The ID of the scheduled action.
        fn execute_emergency_action(ref self: ContractState, action_id: u256) {
            // Check if caller is admin
            self.accesscontrol.assert_only_role(DEFAULT_ADMIN_ROLE);

            // Get the scheduled action
            let action = self.scheduled_emergency_actions.read(action_id);

            // Check if action exists and is in valid state for execution
            assert(action.status != EmergencyActionStatus::Cancelled, Errors::ERR_ACTION_CANCELLED);
            assert(
                action.status != EmergencyActionStatus::Done,
                Errors::EMERGENCY_ACTION_ALREADY_EXECUTED,
            );
            assert(action.status == EmergencyActionStatus::Waiting, Errors::ERR_ACTION_NOT_WAITING);

            // Check if timelock delay has passed
            let current_time = get_block_timestamp();
            assert(current_time >= action.execution_time, Errors::TIMELOCK_DELAY_NOT_PASSED);

            // Execute the action based on type
            match action.action_type {
                EmergencyActionType::FreezePool => {
                    self.emergency_freeze_pool_internal(action.pool_id);
                },
                EmergencyActionType::ResolvePool => {
                    // Extract winning option from action data
                    let winning_option = action.action_data != 0;
                    self.emergency_resolve_pool_internal(action.pool_id, winning_option);
                },
                EmergencyActionType::UnfreezePool => {
                    self.emergency_unfreeze_pool_internal(action.pool_id);
                },
                EmergencyActionType::EmergencyWithdrawal => {
                    // Extract user address from action data for admin-initiated emergency
                    // withdrawal action_data contains the user address as a felt252
                    // (ContractAddress)
                    let user_address = self
                        .extract_user_address_from_action_data(action.action_data);

                    // Check if pool exists
                    let pool = self.pools.read(action.pool_id);
                    assert(pool.exists, Errors::POOL_DOES_NOT_EXIST);

                    // Check if pool is in emergency state
                    let emergency_state = self.emergency_pool_states.read(action.pool_id);
                    assert(emergency_state.is_emergency, Errors::POOL_NOT_IN_EMERGENCY_STATE);
                    assert(
                        emergency_state.allow_emergency_withdrawals,
                        Errors::EMERGENCY_WITHDRAWALS_NOT_ALLOWED,
                    );

                    // Check if user has participated in this pool
                    let has_participated = self
                        .has_user_participated_in_pool(user_address, action.pool_id);
                    assert(has_participated, Errors::UNAUTHORIZED_CALLER);

                    // Get user's stake and calculate withdrawal amount
                    let user_stake = self.get_user_stake(action.pool_id, user_address);
                    assert(user_stake.amount > 0, Errors::NO_FUNDS_TO_WITHDRAW);

                    // Calculate withdrawal amount (return original stake)
                    let withdrawal_amount = user_stake.amount;

                    // Reset user's stake
                    self
                        .user_stakes
                        .write(
                            (action.pool_id, user_address),
                            UserStake { amount: 0, shares: 0, option: false, timestamp: 0 },
                        );

                    // Transfer tokens back to user
                    let strk_token = IERC20Dispatcher { contract_address: self.token_addr.read() };
                    strk_token.transfer(user_address, withdrawal_amount);

                    // Emit emergency withdrawal event
                    self
                        .emit(
                            Event::EmergencyWithdrawal(
                                EmergencyWithdrawal {
                                    pool_id: action.pool_id,
                                    user: user_address,
                                    amount: withdrawal_amount,
                                    timestamp: get_block_timestamp(),
                                },
                            ),
                        );
                },
            }

            // Mark action as executed
            let final_action = EmergencyAction {
                action_type: action.action_type,
                pool_id: action.pool_id,
                action_data: action.action_data,
                scheduled_time: action.scheduled_time,
                execution_time: action.execution_time,
                status: EmergencyActionStatus::Done,
                admin: action.admin,
            };
            self.scheduled_emergency_actions.write(action_id, final_action);

            // Emit action executed event
            self
                .emit(
                    Event::EmergencyActionExecuted(
                        EmergencyActionExecuted {
                            action_id: action_id,
                            admin: get_caller_address(),
                            timestamp: current_time,
                        },
                    ),
                );
        }

        /// @notice Cancels a scheduled emergency action.
        /// @dev Only callable by admin. Cancels action before execution.
        /// @param action_id The ID of the scheduled action.
        fn cancel_emergency_action(ref self: ContractState, action_id: u256) {
            // Check if caller is admin
            self.accesscontrol.assert_only_role(DEFAULT_ADMIN_ROLE);

            // Get the scheduled action
            let action = self.scheduled_emergency_actions.read(action_id);

            // Check if action exists and is in valid state for cancellation
            assert(action.status != EmergencyActionStatus::Cancelled, Errors::ERR_ACTION_CANCELLED);
            assert(
                action.status != EmergencyActionStatus::Done,
                Errors::EMERGENCY_ACTION_ALREADY_EXECUTED,
            );
            assert(action.status == EmergencyActionStatus::Waiting, Errors::ERR_ACTION_NOT_WAITING);

            // Update action status to cancelled
            let updated_action = EmergencyAction {
                action_type: action.action_type,
                pool_id: action.pool_id,
                action_data: action.action_data,
                scheduled_time: action.scheduled_time,
                execution_time: action.execution_time,
                status: EmergencyActionStatus::Cancelled,
                admin: action.admin,
            };
            self.scheduled_emergency_actions.write(action_id, updated_action);

            // Emit action cancelled event
            self
                .emit(
                    Event::EmergencyActionCancelled(
                        EmergencyActionCancelled {
                            action_id: action_id,
                            admin: get_caller_address(),
                            timestamp: get_block_timestamp(),
                        },
                    ),
                );
        }

        /// @notice Returns the status of a scheduled emergency action.
        /// @param action_id The ID of the scheduled action.
        /// @return The action status and execution time.
        fn get_emergency_action_status(self: @ContractState, action_id: u256) -> (u8, u64) {
            let action = self.scheduled_emergency_actions.read(action_id);
            let status_u8 = match action.status {
                EmergencyActionStatus::Unset => 0,
                EmergencyActionStatus::Waiting => 1,
                EmergencyActionStatus::Ready => 2,
                EmergencyActionStatus::Done => 3,
                EmergencyActionStatus::Cancelled => 4,
            };
            (status_u8, action.execution_time)
        }

        /// @notice Returns whether a pool is in emergency state.
        /// @param pool_id The pool ID.
        /// @return True if pool is in emergency state, false otherwise.
        fn is_pool_emergency_state(self: @ContractState, pool_id: u256) -> bool {
            let emergency_state = self.emergency_pool_states.read(pool_id);
            emergency_state.is_emergency
        }

        /// @notice Returns all pools in emergency state.
        /// @return Array of PoolDetails for emergency pools.
        fn get_emergency_pools(self: @ContractState) -> Array<PoolDetails> {
            let mut emergency_pools = array![];
            let len = self.pool_ids.len();

            let mut i: u64 = 0;
            loop {
                if i >= len {
                    break;
                }
                let pool_id = self.pool_ids.at(i).read();
                let pool = self.pools.read(pool_id);
                if pool.exists {
                    let emergency_state = self.emergency_pool_states.read(pool_id);
                    if emergency_state.is_emergency {
                        emergency_pools.append(pool);
                    }
                }
                i += 1;
            }
            emergency_pools
        }
    }

    #[abi(embed_v0)]
    impl dispute of IPredifiDispute<ContractState> {
        /// @notice Raises a dispute against a pool's validation result
        /// @dev Initiates multi-stage dispute resolution process with economic security
        /// @param pool_id Pool identifier where dispute is being raised
        /// @param evidence_hash IPFS hash containing dispute evidence and reasoning
        /// @param stake_amount Amount staked by disputer (forfeited if dispute fails)
        /// @return bool True if dispute is accepted for processing
        /// @custom:evidence-requirement All disputes must include verifiable evidence
        /// @custom:economic-security Disputers stake tokens that are slashed for frivolous disputes
        /// @custom:time-limit Disputes must be raised within specified time window after resolution
        fn raise_dispute(ref self: ContractState, pool_id: u256) {
            self.pausable.assert_not_paused();
            self.assert_greater_than_zero(pool_id);

            let pool = self.pools.read(pool_id);

            // Validation checks using SecurityTrait
            self.assert_pool_exists(@pool);
            self.assert_pool_not_suspended(@pool);

            let caller = get_caller_address();

            let already_disputed = self.pool_dispute_users.read((pool_id, caller));
            self.assert_user_has_not_disputed(already_disputed);

            self.pool_dispute_users.write((pool_id, caller), true);

            let current_count = self.pool_dispute_count.read(pool_id);
            let new_count = current_count + 1;
            self.pool_dispute_count.write(pool_id, new_count);

            self
                .emit(
                    Event::DisputeRaised(
                        DisputeRaised { pool_id, user: caller, timestamp: get_block_timestamp() },
                    ),
                );

            // check if threshold > 3 and suspend pool
            let threshold = self.dispute_threshold.read();
            if new_count >= threshold {
                self.pool_previous_status.write(pool_id, pool.status);

                let mut updated_pool = pool.clone();
                updated_pool.status = Status::Suspended;
                self.pools.write(pool_id, updated_pool);

                self
                    .emit(
                        Event::PoolSuspended(
                            PoolSuspended { pool_id, timestamp: get_block_timestamp() },
                        ),
                    );

                self
                    .emit(
                        Event::PoolStateTransition(
                            PoolStateTransition {
                                pool_id,
                                previous_status: pool.status,
                                new_status: Status::Suspended,
                                timestamp: get_block_timestamp(),
                            },
                        ),
                    );
            }
        }

        /// @notice Processes dispute resolution through community voting
        /// @dev Implements staged dispute resolution with escalation mechanisms
        /// @param dispute_id Unique identifier for the dispute being processed
        /// @param resolution Decision from dispute resolution (Accept/Reject)
        /// @custom:multi-stage Stage 1: Community vote, Stage 2: Expert panel, Stage 3: Governance
        /// @custom:economic-incentives Correct dispute voters receive rewards from dispute stakes
        /// @custom:finality Final dispute decisions are immutable and trigger pool re-resolution
        fn resolve_dispute(ref self: ContractState, pool_id: u256, winning_option: bool) {
            self.pausable.assert_not_paused();
            self.assert_greater_than_zero(pool_id);

            self.accesscontrol.assert_only_role(DEFAULT_ADMIN_ROLE);
            let pool = self.pools.read(pool_id);

            // Validation checks using SecurityTrait
            self.assert_pool_exists(@pool);
            self.assert_pool_suspended(@pool);

            let previous_status = self.pool_previous_status.read(pool_id);
            let mut updated_pool = pool;
            updated_pool.status = previous_status;
            self.pools.write(pool_id, updated_pool);

            self.pool_dispute_count.write(pool_id, 0);

            self
                .emit(
                    Event::DisputeResolved(
                        DisputeResolved {
                            pool_id, winning_option, timestamp: get_block_timestamp(),
                        },
                    ),
                );

            self
                .emit(
                    Event::PoolStateTransition(
                        PoolStateTransition {
                            pool_id,
                            previous_status: Status::Suspended,
                            new_status: previous_status,
                            timestamp: get_block_timestamp(),
                        },
                    ),
                );
        }

        /// @notice Returns the dispute count for a pool.
        /// @param pool_id The pool ID.
        /// @return The dispute count.
        fn get_dispute_count(self: @ContractState, pool_id: u256) -> u256 {
            self.assert_greater_than_zero(pool_id);
            self.pool_dispute_count.read(pool_id)
        }

        /// @notice Returns the dispute threshold.
        /// @return The dispute threshold.
        fn get_dispute_threshold(self: @ContractState) -> u256 {
            self.dispute_threshold.read()
        }

        /// @notice Returns whether a user has disputed a pool.
        /// @param pool_id The pool ID.
        /// @param user The user's address.
        /// @return True if user has disputed, false otherwise.
        fn has_user_disputed(self: @ContractState, pool_id: u256, user: ContractAddress) -> bool {
            self.assert_greater_than_zero(pool_id);
            self.assert_non_zero_address(user);

            self.pool_dispute_users.read((pool_id, user))
        }

        /// @notice Returns whether a pool is suspended.
        /// @param pool_id The pool ID.
        /// @return True if suspended, false otherwise.
        fn is_pool_suspended(self: @ContractState, pool_id: u256) -> bool {
            self.assert_greater_than_zero(pool_id);
            let pool = self.pools.read(pool_id);
            pool.status == Status::Suspended
        }

        /// @notice Returns all suspended pools.
        /// @return Array of PoolDetails.
        fn get_suspended_pools(self: @ContractState) -> Array<PoolDetails> {
            self.get_pools_by_status(Status::Suspended)
        }

        /// @notice Validates an outcome for a pool.
        /// @param pool_id The pool ID.
        /// @param outcome The outcome to validate.
        fn validate_outcome(ref self: ContractState, pool_id: u256, outcome: bool) {
            self.assert_greater_than_zero(pool_id);
            self.pausable.assert_not_paused();
            let pool = self.pools.read(pool_id);

            // Validation checks using SecurityTrait
            self.assert_pool_not_suspended(@pool);
        }

        /// @notice Claims reward for a pool.
        /// @param pool_id The pool ID.
        /// @return The claimed reward amount.
        fn claim_reward(ref self: ContractState, pool_id: u256) -> u256 {
            self.pausable.assert_not_paused();
            self.assert_greater_than_zero(pool_id);

            let pool = self.pools.read(pool_id);

            // Validation checks using SecurityTrait
            self.assert_pool_exists(@pool);
            self.assert_pool_not_suspended(@pool);
            0
        }
    }

    #[abi(embed_v0)]
    impl validator of IPredifiValidator<ContractState> {
        /// @notice Validates the result of a pool.
        /// @dev Only callable by validators. Emits ValidatorResultSubmitted and may settle the
        /// pool.
        /// @param pool_id The pool ID.
        /// @param selected_option The selected option (true = option2, false = option1).
        fn validate_pool_result(ref self: ContractState, pool_id: u256, selected_option: bool) {
            self.pausable.assert_not_paused();
            self.assert_greater_than_zero(pool_id);

            let pool = self.pools.read(pool_id);
            let caller = get_caller_address();

            // Validation checks using SecurityTrait
            self.assert_pool_not_suspended(@pool);

            // Check if caller is an authorized validator
            let is_validator = self.accesscontrol.has_role(VALIDATOR_ROLE, caller);
            assert(is_validator, Errors::VALIDATOR_NOT_AUTHORIZED);

            // Check if pool is in a state that can be validated (Locked status)
            self.assert_pool_ready_for_validation(@pool);

            // Check if validator has already validated this pool
            let has_already_validated = self.pool_validator_confirmations.read((pool_id, caller));
            self.assert_validator_not_already_validated(has_already_validated);

            // Check if the selected option is valid (must be true for option2 or false for option1)
            // Since selected_option is boolean, it's inherently valid

            // Record the validator's confirmation and selection
            self.pool_validator_confirmations.write((pool_id, caller), true);
            self.pool_validation_results.write((pool_id, caller), selected_option);

            // Increment validation count
            let current_count = self.pool_validation_count.read(pool_id);
            let new_count = current_count + 1;
            self.pool_validation_count.write(pool_id, new_count);

            // Emit validator result submitted event
            self
                .emit(
                    Event::ValidatorResultSubmitted(
                        ValidatorResultSubmitted {
                            pool_id,
                            validator: caller,
                            selected_option,
                            timestamp: get_block_timestamp(),
                        },
                    ),
                );

            // Check if we have reached the required number of confirmations
            let required_confirmations = self.required_validator_confirmations.read();

            if new_count >= required_confirmations {
                // Calculate the final outcome based on majority vote
                let final_outcome = self.calculate_validation_consensus(pool_id, new_count);

                // Store the final outcome
                self.pool_final_outcome.write(pool_id, final_outcome);

                // Update pool status to Settled
                let mut updated_pool = pool;
                updated_pool.status = Status::Settled;
                self.pools.write(pool_id, updated_pool);

                // Emit pool state transition event
                self
                    .emit(
                        Event::PoolStateTransition(
                            PoolStateTransition {
                                pool_id,
                                previous_status: Status::Locked,
                                new_status: Status::Settled,
                                timestamp: get_block_timestamp(),
                            },
                        ),
                    );

                // Emit pool automatically settled event
                self
                    .emit(
                        Event::PoolAutomaticallySettled(
                            PoolAutomaticallySettled {
                                pool_id,
                                final_outcome,
                                total_validations: new_count,
                                timestamp: get_block_timestamp(),
                            },
                        ),
                    );

                // Emit pool resolved event for compatibility
                let total_payout = self.calculate_total_payout(pool_id, final_outcome);
                self
                    .emit(
                        Event::PoolResolved(
                            PoolResolved { pool_id, winning_option: final_outcome, total_payout },
                        ),
                    );
            }
        }

        /// @notice Gets pool validation status.
        /// @param pool_id The ID of the pool to check.
        /// @return (validation count, is settled, final outcome).
        fn get_pool_validation_status(self: @ContractState, pool_id: u256) -> (u256, bool, bool) {
            self.assert_greater_than_zero(pool_id);
            let validation_count = self.pool_validation_count.read(pool_id);
            let required_confirmations = self.required_validator_confirmations.read();
            let is_settled = validation_count >= required_confirmations;
            let final_outcome = self.pool_final_outcome.read(pool_id);

            (validation_count, is_settled, final_outcome)
        }

        /// @notice Gets validator confirmation status.
        /// @param pool_id The ID of the pool to check.
        /// @param validator The address of the validator to check.
        /// @return (has confirmed, selected option).
        fn get_validator_confirmation(
            self: @ContractState, pool_id: u256, validator: ContractAddress,
        ) -> (bool, bool) {
            self.assert_greater_than_zero(pool_id);
            self.assert_non_zero_address(validator);
            let has_validated = self.pool_validator_confirmations.read((pool_id, validator));
            let selected_option = self.pool_validation_results.read((pool_id, validator));

            (has_validated, selected_option)
        }

        /// @notice Sets the required number of validator confirmations for a pool.
        /// @dev Only callable by admin.
        /// @param count The number of confirmations required.
        fn set_required_validator_confirmations(ref self: ContractState, count: u256) {
            self.pausable.assert_not_paused();

            // Only admin can set this
            self.accesscontrol.assert_only_role(DEFAULT_ADMIN_ROLE);
            self.assert_positive_count(count);
            
            let previous_count = self.required_validator_confirmations.read();
            self.required_validator_confirmations.write(count);

            // Emit event for validator confirmations update
            self.emit(
                Event::ValidatorConfirmationsUpdated(
                    ValidatorConfirmationsUpdated {
                        previous_count,
                        new_count: count,
                        admin: get_caller_address(),
                        timestamp: get_block_timestamp(),
                    }
                )
            );
        }

        /// @notice Gets the validators assigned to a pool.
        /// @param pool_id The pool ID.
        /// @return (validator1, validator2).
        fn get_pool_validators(
            self: @ContractState, pool_id: u256,
        ) -> (ContractAddress, ContractAddress) {
            self.assert_greater_than_zero(pool_id);
            self.pool_validator_assignments.read(pool_id)
        }

        /// @notice Assigns random validators to a pool.
        /// @dev Internal function.
        /// @param pool_id The pool ID.
        fn assign_random_validators(ref self: ContractState, pool_id: u256) {
            // Get the number of available validators
            let validator_count = self.validators.len();

            // If we have fewer than 2 validators, handle the edge case
            if validator_count == 0 {
                // No validators available, don't assign any
                return;
            } else if validator_count == 1 {
                // Only one validator available, assign the same validator twice
                let validator = self.validators.at(0).read();
                self.assign_validators(pool_id, validator, validator);
                return;
            }

            // Generate two random indices for validator selection
            // Use the pool_id and timestamp to create randomness
            let timestamp = get_block_timestamp();
            let seed1 = pool_id + timestamp.into();
            let seed2 = pool_id + (timestamp * 2).into();

            // Use modulo to get indices within the range of available validators
            let index1 = seed1 % validator_count.into();
            // Ensure the second index is different from the first
            let mut index2 = seed2 % validator_count.into();
            if index1 == index2 && validator_count > 1 {
                index2 = (index2 + 1) % validator_count.into();
            }

            // Get the selected validators
            let validator1 = self.validators.at(index1.try_into().unwrap()).read();
            let validator2 = self.validators.at(index2.try_into().unwrap()).read();

            // Assign the selected validators to the pool
            self.assign_validators(pool_id, validator1, validator2);
        }

        /// @notice Assigns specific validators to a pool.
        /// @dev Internal function.
        /// @param pool_id The pool ID.
        /// @param validator1 The first validator.
        /// @param validator2 The second validator.
        fn assign_validators(
            ref self: ContractState,
            pool_id: u256,
            validator1: ContractAddress,
            validator2: ContractAddress,
        ) {
            self.pool_validator_assignments.write(pool_id, (validator1, validator2));
            let timestamp = get_block_timestamp();
            self
                .emit(
                    Event::ValidatorsAssigned(
                        ValidatorsAssigned { pool_id, validator1, validator2, timestamp },
                    ),
                );
        }

        /// @notice Adds a validator.
        /// @dev Only callable by admin.
        /// @param address The validator's address.
        fn add_validator(ref self: ContractState, address: ContractAddress) {
            self.accesscontrol.assert_only_role(DEFAULT_ADMIN_ROLE);

            if (self.is_validator(address)) {
                return;
            }
            self.accesscontrol.grant_role(VALIDATOR_ROLE, address);
            self.validators.push(address);

            self.emit(ValidatorAdded { account: address, caller: get_caller_address() });
        }

        /// @notice Removes a validator.
        /// @dev Only callable by admin.
        /// @param address The validator's address.
        fn remove_validator(ref self: ContractState, address: ContractAddress) {
            self.accesscontrol.assert_only_role(DEFAULT_ADMIN_ROLE);

            if (!self.is_validator(address)) {
                return;
            }

            self.accesscontrol.revoke_role(VALIDATOR_ROLE, address);

            let num_validators = self.validators.len();
            for i in 0..num_validators {
                if (self.validators.at(i).read() == address) {
                    // Pop last element from validators list
                    let last_validator = self.validators.pop().unwrap();

                    // If revoked address isn't last element, overwrite with popped element
                    if (i < (num_validators - 1)) {
                        self.validators.at(i).write(last_validator);
                    }

                    self.emit(ValidatorRemoved { account: address, caller: get_caller_address() });
                    return;
                }
            }
        }

        /// @notice Checks if an address is a validator.
        /// @param address The address to check.
        /// @return True if validator, false otherwise.
        fn is_validator(self: @ContractState, address: ContractAddress) -> bool {
            self.assert_non_zero_address(address);
            self.accesscontrol.has_role(VALIDATOR_ROLE, address)
        }

        /// @notice Returns all validators.
        /// @return Array of validator addresses.
        fn get_all_validators(self: @ContractState) -> Array<ContractAddress> {
            let mut validators = array![];

            for i in 0..self.validators.len() {
                let validator = self.validators.at(i).read();
                validators.append(validator);
            }
            validators
        }

        /// @notice Calculates the validator fee for a pool.
        /// @param pool_id The pool ID.
        /// @param total_amount The total amount to calculate fee from.
        /// @return The validator fee.
        fn calculate_validator_fee(
            ref self: ContractState, pool_id: u256, total_amount: u256,
        ) -> u256 {
            self.assert_greater_than_zero(pool_id);
            self.assert_greater_than_zero(total_amount);
            // Validator fee is fixed at 10%
            let validator_fee_percentage = 5_u8;
            let mut validator_fee = (total_amount * validator_fee_percentage.into()) / 100_u256;

            self.validator_fee.write(pool_id, validator_fee);
            validator_fee
        }

        /// @notice Distributes validator fees for a pool.
        /// @param pool_id The pool ID.
        fn distribute_validator_fees(ref self: ContractState, pool_id: u256) {
            self.assert_greater_than_zero(pool_id);
            let total_validator_fee = self.validator_fee.read(pool_id);

            let validator_count = self.validators.len();

            // Convert validator_count to u256 for the division
            let validator_count_u256: u256 = validator_count.into();
            let fee_per_validator = total_validator_fee / validator_count_u256;

            let strk_token = IERC20Dispatcher { contract_address: self.token_addr.read() };

            // Distribute to each validator
            let mut i: u64 = 0;
            while i != validator_count {
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

        /// @notice Retrieves the validator fee for a pool.
        /// @param pool_id The pool ID.
        /// @return The validator fee.
        fn retrieve_validator_fee(self: @ContractState, pool_id: u256) -> u256 {
            self.assert_greater_than_zero(pool_id);
            self.validator_fee.read(pool_id)
        }

        /// @notice Gets the validator fee percentage for a pool.
        /// @param pool_id The pool ID.
        /// @return The validator fee percentage.
        fn get_validator_fee_percentage(self: @ContractState, pool_id: u256) -> u8 {
            self.assert_greater_than_zero(pool_id);
            10_u8
        }

        /// @notice Pauses all state-changing operations in the contract.
        /// @dev Can only be called by admin. Emits Paused event on success.
        fn pause(ref self: ContractState) {
            // Check if caller has appropriate role (admin)
            self.accesscontrol.assert_only_role(DEFAULT_ADMIN_ROLE);

            self.pausable.pause();

            // Emit custom contract paused event
            self.emit(
                Event::ContractPaused(
                    ContractPaused {
                        admin: get_caller_address(),
                        timestamp: get_block_timestamp(),
                    }
                )
            );
        }

        /// @notice Unpauses the contract and resumes normal operations
        /// @dev Can only be called by admin. Emits Unpaused event on success.
        fn unpause(ref self: ContractState) {
            // Check if caller has appropriate role (admin)
            self.accesscontrol.assert_only_role(DEFAULT_ADMIN_ROLE);

            self.pausable.unpause();

            // Emit custom contract unpaused event
            self.emit(
                Event::ContractUnpaused(
                    ContractUnpaused {
                        admin: get_caller_address(),
                        timestamp: get_block_timestamp(),
                    }
                )
            );
        }

        /// @notice Upgrades the contract implementation
        /// @param new_class_hash The class hash of the new implementation
        /// @dev Can only be called by admin when contract is not paused
        fn upgrade(ref self: ContractState, new_class_hash: ClassHash) {
            self.pausable.assert_not_paused();
            // This function can only be called by the admin
            self.accesscontrol.assert_only_role(DEFAULT_ADMIN_ROLE);

            // Replace the class hash, hence upgrading the contract
            self.upgradeable.upgrade(new_class_hash);

            // Emit contract upgraded event
            self.emit(
                Event::ContractUpgraded(
                    ContractUpgraded {
                        admin: get_caller_address(),
                        new_class_hash,
                        timestamp: get_block_timestamp(),
                    }
                )
            );
        }
    }

    #[generate_trait]
    impl Private of PrivateTrait {
        /// @notice Generates a deterministic `u256` with 6 decimal places.
        /// @dev Combines block number, timestamp, and sender address for uniqueness.
        /// @return A deterministic u256 value.
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

        /// @notice Saves user data using Pedersen hash.
        /// @param salt The salt value.
        /// @return The Pedersen hash.
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

        /// @notice Calculates shares for a bet.
        /// @param amount The bet amount.
        /// @param total_stake_selected_option Total stake for selected option.
        /// @param total_stake_other_option Total stake for other option.
        /// @return The calculated shares.
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

        /// @notice Calculates odds for a pool.
        /// @param pool_id The pool ID.
        /// @param total_stake_option1 Total stake for option 1.
        /// @param total_stake_option2 Total stake for option 2.
        /// @return The PoolOdds struct.
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

        /// @notice Tracks user participation in a pool.
        /// @dev Called when a user votes or stakes in a pool.
        /// @param user The user's address.
        /// @param pool_id The pool ID.
        fn track_user_participation(ref self: ContractState, user: ContractAddress, pool_id: u256) {
            // Check if this is a new participation
            if !self.user_participated_pools.read((user, pool_id)) {
                // Mark this pool as participated
                self.user_participated_pools.write((user, pool_id), true);

                // Increment the user's pool count
                let current_count = self.user_pool_count.read(user);
                self.user_pool_count.write(user, current_count + 1);

                // Add this pool_id to the user's list of participated pools
                let user_pool_ids_count = self.user_pool_ids_count.read(user);
                self.user_pool_ids.write((user, user_pool_ids_count), pool_id);
                self.user_pool_ids_count.write(user, user_pool_ids_count + 1);
            }
        }

        /// @notice Returns pools by status.
        /// @param status The pool status.
        /// @return Array of PoolDetails.
        fn get_pools_by_status(self: @ContractState, status: Status) -> Array<PoolDetails> {
            let mut result = array![];
            let len = self.pool_ids.len();

            let mut i: u64 = 0;
            while i != len {
                let pool_id = self.pool_ids.at(i).read();
                let pool = self.pools.read(pool_id);
                if pool.status == status {
                    result.append(pool);
                }
                i += 1;
            }
            result
        }

        /// @notice Calculates the validation consensus for a pool.
        /// @param pool_id The pool ID.
        /// @param total_validations The total number of validations.
        /// @return True if option2 wins, false if option1 wins.
        fn calculate_validation_consensus(
            self: @ContractState, pool_id: u256, total_validations: u256,
        ) -> bool {
            let mut option1_votes = 0_u256;
            let mut option2_votes = 0_u256;

            // Get all validators and count their votes
            let validators = self.get_all_validators();
            let mut i = 0;

            while i != validators.len() {
                let validator = *validators.at(i);
                let has_validated = self.pool_validator_confirmations.read((pool_id, validator));

                if has_validated {
                    let selected_option = self.pool_validation_results.read((pool_id, validator));
                    if selected_option {
                        option2_votes += 1;
                    } else {
                        option1_votes += 1;
                    }
                }
                i += 1;
            }

            // Return true (option2) if option2 has more votes, false (option1) otherwise
            // In case of tie, default to option1 (false)
            option2_votes > option1_votes
        }

        /// @notice Calculates the total payout for a pool.
        /// @param pool_id The pool ID.
        /// @param winning_option The winning option.
        /// @return The total payout amount.
        fn calculate_total_payout(
            self: @ContractState, pool_id: u256, winning_option: bool,
        ) -> u256 {
            let pool = self.pools.read(pool_id);

            // Calculate fees
            let creator_fee_amount = (pool.totalBetAmountStrk * pool.creatorFee.into()) / 100_u256;
            let validator_fee_amount = self.validator_fee.read(pool_id);
            let protocol_fee_amount = (pool.totalBetAmountStrk * 5_u256)
                / 100_u256; // 5% protocol fee

            // Total payout is total bet amount minus all fees
            let total_fees = creator_fee_amount + validator_fee_amount + protocol_fee_amount;
            let total_payout = pool.totalBetAmountStrk - total_fees;

            total_payout
        }

        /// @notice Collects the pool creation fee from the creator.
        /// @dev Transfers 1 STRK from creator to contract.
        /// @param creator The creator's address.
        fn collect_pool_creation_fee(ref self: ContractState, creator: ContractAddress) {
            // Retrieve the STRK token contract
            let strk_token = IERC20Dispatcher { contract_address: self.token_addr.read() };

            // Check if the creator has sufficient balance for pool creation fee
            let creator_balance = strk_token.balance_of(creator);
            assert(creator_balance >= ONE_STRK, Errors::INSUFFICIENT_BALANCE);

            // Check allowance to ensure the contract can transfer tokens
            let contract_address = get_contract_address();
            let allowed_amount = strk_token.allowance(creator, contract_address);
            assert(allowed_amount >= ONE_STRK, Errors::INSUFFICIENT_ALLOWANCE);

            // Transfer the pool creation fee from creator to the contract
            strk_token.transfer_from(creator, contract_address, ONE_STRK);
        }

        /// @notice Calculates the validator fee for a pool.
        /// @param pool_id The pool ID.
        /// @param total_amount The total amount to calculate fee from.
        /// @return The validator fee.
        fn calculate_validator_fee(
            ref self: ContractState, pool_id: u256, total_amount: u256,
        ) -> u256 {
            // Validator fee is fixed at 5%
            let validator_fee_percentage = 5_u8;
            let mut validator_fee = (total_amount * validator_fee_percentage.into()) / 100_u256;

            self.validator_fee.write(pool_id, validator_fee);
            validator_fee
        }

        /// @notice Distributes validator fees for a pool.
        /// @param pool_id The pool ID.
        fn distribute_validator_fees(ref self: ContractState, pool_id: u256) {
            let total_validator_fee = self.validator_fee.read(pool_id);

            let validator_count = self.validators.len();

            // Convert validator_count to u256 for the division
            let validator_count_u256: u256 = validator_count.into();
            let fee_per_validator = total_validator_fee / validator_count_u256;

            let strk_token = IERC20Dispatcher { contract_address: self.token_addr.read() };

            // Distribute to each validator
            let mut i: u64 = 0;
            while i != validator_count {
                // Safe access to validator - check bounds first
                if i < self.validators.len() {
                    let validator_address = self.validators.at(i).read();
                    strk_token.transfer(validator_address, fee_per_validator);
                }
                i += 1;
            }
            // Reset the validator fee for this pool after distribution
            self.validator_fee.write(pool_id, 0);
        }


        /// @notice Returns whether a pool is in emergency state.
        /// @param pool_id The pool ID.
        /// @return True if pool is in emergency state, false otherwise.
        fn is_pool_emergency_state(self: @ContractState, pool_id: u256) -> bool {
            let emergency_state = self.emergency_pool_states.read(pool_id);
            emergency_state.is_emergency
        }

        /// @notice Returns all pools in emergency state.
        /// @return Array of PoolDetails for emergency pools.
        fn get_emergency_pools(self: @ContractState) -> Array<PoolDetails> {
            let mut emergency_pools = array![];
            let len = self.pool_ids.len();

            let mut i: u64 = 0;
            loop {
                if i >= len {
                    break;
                }
                let pool_id = self.pool_ids.at(i).read();
                let pool = self.pools.read(pool_id);
                if pool.exists {
                    let emergency_state = self.emergency_pool_states.read(pool_id);
                    if emergency_state.is_emergency {
                        emergency_pools.append(pool);
                    }
                }
                i += 1;
            }
            emergency_pools
        }

        /// @notice Internal emergency pool freeze function.
        /// @dev Only callable internally by the timelocked executor. Freezes a pool in emergency
        /// state.
        /// @param pool_id The pool ID to freeze.
        fn emergency_freeze_pool_internal(ref self: ContractState, pool_id: u256) {
            // Check if pool exists
            let pool = self.pools.read(pool_id);
            assert(pool.exists, Errors::POOL_DOES_NOT_EXIST);

            // Check if pool is not already in emergency state
            let emergency_state = self.emergency_pool_states.read(pool_id);
            assert(!emergency_state.is_emergency, Errors::POOL_ALREADY_IN_EMERGENCY_STATE);

            // Set pool to emergency state
            let new_emergency_state = EmergencyPoolState {
                is_emergency: true,
                emergency_timestamp: get_block_timestamp(),
                emergency_reason: 'Admin emergency freeze',
                emergency_admin: get_caller_address(),
                allow_emergency_withdrawals: true,
            };
            self.emergency_pool_states.write(pool_id, new_emergency_state);

            // Emit emergency freeze event
            self
                .emit(
                    Event::PoolEmergencyFrozen(
                        PoolEmergencyFrozen {
                            pool_id,
                            admin: get_caller_address(),
                            reason: 'Admin emergency freeze',
                            timestamp: get_block_timestamp(),
                        },
                    ),
                );
        }

        /// @notice Internal emergency pool unfreeze function.
        /// @dev Only callable internally by the timelocked executor. Unfreezes a pool from
        /// emergency state.
        /// @param pool_id The pool ID to unfreeze.
        fn emergency_unfreeze_pool_internal(ref self: ContractState, pool_id: u256) {
            // Check if pool exists
            let pool = self.pools.read(pool_id);
            assert(pool.exists, Errors::POOL_DOES_NOT_EXIST);

            // Check if pool is in emergency state
            let emergency_state = self.emergency_pool_states.read(pool_id);
            assert(emergency_state.is_emergency, Errors::POOL_NOT_IN_EMERGENCY_STATE);

            // Clear emergency state
            let new_emergency_state = EmergencyPoolState {
                is_emergency: false,
                emergency_timestamp: 0,
                emergency_reason: 0,
                emergency_admin: get_caller_address(),
                allow_emergency_withdrawals: false,
            };
            self.emergency_pool_states.write(pool_id, new_emergency_state);

            // Emit emergency unfreeze event
            self
                .emit(
                    Event::PoolEmergencyUnfrozen(
                        PoolEmergencyUnfrozen {
                            pool_id, admin: get_caller_address(), timestamp: get_block_timestamp(),
                        },
                    ),
                );
        }

        // Helper function to convert u8 to EmergencyActionType
        fn u8_to_emergency_action_type(
            self: @ContractState, action_type: u8,
        ) -> EmergencyActionType {
            match action_type {
                0 => EmergencyActionType::FreezePool,
                1 => EmergencyActionType::ResolvePool,
                2 => EmergencyActionType::UnfreezePool,
                3 => EmergencyActionType::EmergencyWithdrawal,
                _ => panic!("Invalid emergency action type"),
            }
        }

        /// @notice Extracts and validates a user address from action data.
        /// @dev Helper function for emergency withdrawal actions.
        /// @param action_data The action data containing the user address as felt252.
        /// @return The validated user address as ContractAddress.
        fn extract_user_address_from_action_data(
            self: @ContractState, action_data: felt252,
        ) -> ContractAddress {
            // Validate that the address is not zero
            assert(action_data != 0, Errors::INVALID_ADDRESS);

            // Convert felt252 to ContractAddress with proper error handling
            let user_address: ContractAddress = match action_data.try_into() {
                Option::Some(addr) => addr,
                Option::None => panic_with_felt252(Errors::ACTION_DATA_DECODE_FAILED),
            };

            // Additional validation: ensure the address is not the zero address
            let zero_address: ContractAddress = match 0.try_into() {
                Option::Some(addr) => addr,
                Option::None => panic_with_felt252(Errors::ACTION_DATA_DECODE_FAILED),
            };
            assert(user_address != zero_address, Errors::INVALID_ADDRESS);

            user_address
        }


        /// @notice Internal emergency pool resolution function.
        /// @dev Only callable internally by the timelocked executor. Resolves a disputed pool with
        /// emergency outcome.
        /// @param pool_id The pool ID to resolve.
        /// @param winning_option The winning option (true = option2, false = option1).
        fn emergency_resolve_pool_internal(
            ref self: ContractState, pool_id: u256, winning_option: bool,
        ) {
            // Check if pool exists
            let pool = self.pools.read(pool_id);
            assert(pool.exists, Errors::POOL_DOES_NOT_EXIST);

            // Check if pool is in emergency state
            let emergency_state = self.emergency_pool_states.read(pool_id);
            assert(emergency_state.is_emergency, Errors::POOL_NOT_IN_EMERGENCY_STATE);

            // Set pool outcome and mark as resolved
            self.pool_outcomes.write(pool_id, winning_option);
            self.pool_resolved.write(pool_id, true);

            // Update pool status to settled
            let mut updated_pool = pool;
            updated_pool.status = Status::Settled;
            self.pools.write(pool_id, updated_pool);

            // Clear emergency state after resolution
            let new_emergency_state = EmergencyPoolState {
                is_emergency: false,
                emergency_timestamp: 0,
                emergency_reason: 0,
                emergency_admin: get_caller_address(),
                allow_emergency_withdrawals: false,
            };
            self.emergency_pool_states.write(pool_id, new_emergency_state);

            // Emit emergency resolution event
            self
                .emit(
                    Event::PoolEmergencyResolved(
                        PoolEmergencyResolved {
                            pool_id,
                            admin: get_caller_address(),
                            winning_option,
                            timestamp: get_block_timestamp(),
                        },
                    ),
                );
        }
    }
}

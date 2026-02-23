#![no_std]

mod safe_math;
#[cfg(test)]
mod safe_math_examples;

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, Env,
    IntoVal, String, Symbol, Vec,
};

pub use safe_math::{RoundingMode, SafeMath};

// ═══════════════════════════════════════════════════════════════════════════
// PROTOCOL INVARIANTS (for formal verification)
// ═══════════════════════════════════════════════════════════════════════════
//
// INV-1: Pool.total_stake = Σ(OutcomeStake(pool_id, outcome)) for all outcomes
// INV-2: Pool.state transitions: Active → {Resolved | Canceled}, never reversed
// INV-3: HasClaimed(user, pool) is write-once (prevents double-claim)
// INV-4: Winnings ≤ Pool.total_stake (no value creation)
// INV-5: For resolved pools: Σ(claimed_winnings) ≤ Pool.total_stake
// INV-6: Config.fee_bps ≤ 10_000 (max 100%)
// INV-7: Prediction.amount > 0 (no zero-stakes)
// INV-8: Pool.end_time > creation_time (pools must have future end)
//
// ═══════════════════════════════════════════════════════════════════════════

const DAY_IN_LEDGERS: u32 = 17280;
const BUMP_THRESHOLD: u32 = 14 * DAY_IN_LEDGERS;
const BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;

/// Minimum pool duration in seconds (1 hour)
const MIN_POOL_DURATION: u64 = 3600;
/// Maximum number of options allowed in a pool
const MAX_OPTIONS_COUNT: u32 = 100;
/// Stake amount (in base token units) above which a `HighValuePredictionEvent`
/// is emitted so off-chain monitors can apply extra scrutiny.
/// At 7 decimal places (e.g. USDC on Stellar) this equals 100 USDC.
const HIGH_VALUE_THRESHOLD: i128 = 1_000_000;

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PredifiError {
    Unauthorized = 10,
    PoolNotResolved = 22,
    InvalidPoolState = 24,
    AlreadyClaimed = 60,
}

#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarketState {
    Active = 0,
    Resolved = 1,
    Canceled = 2,
}

#[contracttype]
#[derive(Clone)]
pub struct Pool {
    pub end_time: u64,
    pub state: MarketState,
    pub outcome: u32,
    pub token: Address,
    pub total_stake: i128,
    /// A short human-readable description of the event being predicted.
    pub description: String,
    /// A URL (e.g. IPFS CIDv1) pointing to extended metadata for this pool.
    pub metadata_url: String,
    /// Number of options/outcomes for this pool (must be <= MAX_OPTIONS_COUNT)
    pub options_count: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct Config {
    pub fee_bps: u32,
    pub treasury: Address,
    pub access_control: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct UserPredictionDetail {
    pub pool_id: u64,
    pub amount: i128,
    pub user_outcome: u32,
    pub pool_end_time: u64,
    pub pool_state: MarketState,
    pub pool_outcome: u32,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Pool(u64),
    Prediction(Address, u64),
    PoolIdCounter,
    HasClaimed(Address, u64),
    OutcomeStake(u64, u32),
    UserPredictionCount(Address),
    UserPredictionIndex(Address, u32),
    Config,
    Paused,
}

#[contracttype]
#[derive(Clone)]
pub struct Prediction {
    pub amount: i128,
    pub outcome: u32,
}

// ── Events ───────────────────────────────────────────────────────────────────

#[contractevent(topics = ["init"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitEvent {
    pub access_control: Address,
    pub treasury: Address,
    pub fee_bps: u32,
}

#[contractevent(topics = ["pause"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseEvent {
    pub admin: Address,
}

#[contractevent(topics = ["unpause"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnpauseEvent {
    pub admin: Address,
}

#[contractevent(topics = ["fee_update"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeUpdateEvent {
    pub admin: Address,
    pub fee_bps: u32,
}

#[contractevent(topics = ["treasury_update"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreasuryUpdateEvent {
    pub admin: Address,
    pub treasury: Address,
}

#[contractevent(topics = ["pool_created"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolCreatedEvent {
    pub pool_id: u64,
    pub end_time: u64,
    pub token: Address,
    /// Number of options/outcomes for this pool.
    pub options_count: u32,
    /// Metadata URL included so off-chain indexers can immediately fetch context.
    pub metadata_url: String,
}

#[contractevent(topics = ["pool_resolved"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolResolvedEvent {
    pub pool_id: u64,
    pub operator: Address,
    pub outcome: u32,
}

#[contractevent(topics = ["pool_canceled"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolCanceledEvent {
    pub pool_id: u64,
    pub operator: Address,
}

#[contractevent(topics = ["prediction_placed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredictionPlacedEvent {
    pub pool_id: u64,
    pub user: Address,
    pub amount: i128,
    pub outcome: u32,
}

#[contractevent(topics = ["winnings_claimed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WinningsClaimedEvent {
    pub pool_id: u64,
    pub user: Address,
    pub amount: i128,
}

// ── Monitoring & Alert Events ─────────────────────────────────────────────────
// These events are classified by severity and are intended for consumption by
// off-chain monitoring tools (Horizon event streaming, Grafana, SIEM, etc.).
// See MONITORING.md at the repo root for scraping patterns and alert rules.

/// 🔴 HIGH ALERT — emitted when `resolve_pool` is called by an address that
/// does not hold the Operator role.  Indicates a potential attack or
/// misconfigured access-control contract.
#[contractevent(topics = ["unauthorized_resolution"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnauthorizedResolveAttemptEvent {
    /// The address that attempted to resolve without authorization.
    pub caller: Address,
    /// The pool that was targeted.
    pub pool_id: u64,
    /// Ledger timestamp at the time of the attempt.
    pub timestamp: u64,
}

/// 🔴 HIGH ALERT — emitted when an admin-restricted operation (`set_fee_bps`,
/// `set_treasury`, `pause`, `unpause`) is called by an address that does not
/// hold the Admin role.
#[contractevent(topics = ["unauthorized_admin_op"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnauthorizedAdminAttemptEvent {
    /// The address that attempted the restricted operation.
    pub caller: Address,
    /// Short name of the operation that was attempted.
    pub operation: Symbol,
    /// Ledger timestamp at the time of the attempt.
    pub timestamp: u64,
}

/// 🔴 HIGH ALERT — emitted when `claim_winnings` is called after winnings have
/// already been claimed for the same (user, pool) pair.  Repeated attempts may
/// indicate a re-entrancy probe or a front-end bug worth investigating.
#[contractevent(topics = ["double_claim_attempt"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuspiciousDoubleClaimEvent {
    /// The address that attempted to double-claim.
    pub user: Address,
    /// The pool for which the claim was already made.
    pub pool_id: u64,
    /// Ledger timestamp at the time of the attempt.
    pub timestamp: u64,
}

/// 🔴 HIGH ALERT — emitted alongside `PauseEvent` whenever the contract is
/// successfully paused.  Having a dedicated alert topic makes it easy to set
/// a zero-tolerance PagerDuty rule that fires on any pause.
#[contractevent(topics = ["contract_paused_alert"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractPausedAlertEvent {
    /// The admin that triggered the pause.
    pub admin: Address,
    /// Ledger timestamp at pause time.
    pub timestamp: u64,
}

/// 🟡 MEDIUM ALERT — emitted in `place_prediction` when the staked amount
/// meets or exceeds `HIGH_VALUE_THRESHOLD`.  Useful for liquidity monitoring
/// and detecting unusual betting patterns.
#[contractevent(topics = ["high_value_prediction"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HighValuePredictionEvent {
    pub pool_id: u64,
    pub user: Address,
    pub amount: i128,
    pub outcome: u32,
    /// The threshold that was breached (aids display in dashboards).
    pub threshold: i128,
}

/// 🟢 INFO — emitted alongside `PoolResolvedEvent` with enriched numeric
/// context so monitors can calculate implied payouts and flag anomalies
/// (e.g., winning_stake == 0 meaning no winners).
#[contractevent(topics = ["pool_resolved_diag"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolResolvedDiagEvent {
    pub pool_id: u64,
    pub outcome: u32,
    /// Total stake across all outcomes at resolution time.
    pub total_stake: i128,
    /// Stake on the winning outcome (0 ⟹ no winners — notable anomaly).
    pub winning_stake: i128,
    /// Ledger timestamp at resolution time.
    pub timestamp: u64,
}

// ─────────────────────────────────────────────────────────────────────────────

#[contract]
pub struct PredifiContract;

#[contractimpl]
impl PredifiContract {
    // ── Pure Helper Functions (side-effect free, verifiable) ──────────────────

    /// Pure: Calculate winnings for a user given pool state
    /// PRE: winning_stake > 0
    /// POST: result ≤ total_stake (INV-4)
    fn calculate_winnings(user_stake: i128, winning_stake: i128, total_stake: i128) -> i128 {
        if winning_stake == 0 {
            return 0;
        }
        // (user_stake / winning_stake) * total_stake
        user_stake
            .checked_mul(total_stake)
            .expect("overflow in winnings calculation")
            .checked_div(winning_stake)
            .expect("division by zero")
    }

    /// Pure: Check if pool state transition is valid
    /// PRE: current_state is valid MarketState
    /// POST: returns true only for valid transitions (INV-2)
    fn is_valid_state_transition(current: MarketState, next: MarketState) -> bool {
        matches!(
            (current, next),
            (MarketState::Active, MarketState::Resolved)
                | (MarketState::Active, MarketState::Canceled)
        )
    }

    /// Pure: Validate fee basis points
    /// POST: returns true iff fee_bps ≤ 10_000 (INV-6)
    fn is_valid_fee_bps(fee_bps: u32) -> bool {
        fee_bps <= 10_000
    }

    // ── Storage & Side-Effect Functions ───────────────────────────────────────

    fn extend_instance(env: &Env) {
        env.storage()
            .instance()
            .extend_ttl(BUMP_THRESHOLD, BUMP_AMOUNT);
    }

    fn extend_persistent(env: &Env, key: &DataKey) {
        env.storage()
            .persistent()
            .extend_ttl(key, BUMP_THRESHOLD, BUMP_AMOUNT);
    }

    fn has_role(env: &Env, contract: &Address, user: &Address, role: u32) -> bool {
        env.invoke_contract(
            contract,
            &Symbol::new(env, "has_role"),
            soroban_sdk::vec![env, user.into_val(env), role.into_val(env)],
        )
    }

    fn require_role(env: &Env, user: &Address, role: u32) -> Result<(), PredifiError> {
        let config = Self::get_config(env);
        if !Self::has_role(env, &config.access_control, user, role) {
            return Err(PredifiError::Unauthorized);
        }
        Ok(())
    }

    fn get_config(env: &Env) -> Config {
        let config = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .expect("Config not set");
        Self::extend_instance(env);
        config
    }

    fn is_paused(env: &Env) -> bool {
        let paused = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        Self::extend_instance(env);
        paused
    }

    fn require_not_paused(env: &Env) {
        if Self::is_paused(env) {
            panic!("Contract is paused");
        }
    }

    // ── Public interface ──────────────────────────────────────────────────────

    /// Initialize the contract. Idempotent — safe to call multiple times.
    pub fn init(env: Env, access_control: Address, treasury: Address, fee_bps: u32) {
        if !env.storage().instance().has(&DataKey::Config) {
            let config = Config {
                fee_bps,
                treasury: treasury.clone(),
                access_control: access_control.clone(),
            };
            env.storage().instance().set(&DataKey::Config, &config);
            env.storage().instance().set(&DataKey::PoolIdCounter, &0u64);
            Self::extend_instance(&env);

            InitEvent {
                access_control,
                treasury,
                fee_bps,
            }
            .publish(&env);
        }
    }

    /// Pause the contract. Only callable by Admin (role 0).
    pub fn pause(env: Env, admin: Address) {
        admin.require_auth();
        if Self::require_role(&env, &admin, 0).is_err() {
            UnauthorizedAdminAttemptEvent {
                caller: admin,
                operation: Symbol::new(&env, "pause"),
                timestamp: env.ledger().timestamp(),
            }
            .publish(&env);
            panic!("Unauthorized: missing required role");
        }
        env.storage().instance().set(&DataKey::Paused, &true);
        Self::extend_instance(&env);

        // Emit dedicated pause-alert event so monitors can apply zero-tolerance
        // rules independently of the generic PauseEvent.
        ContractPausedAlertEvent {
            admin: admin.clone(),
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);
        PauseEvent { admin }.publish(&env);
    }

    /// Unpause the contract. Only callable by Admin (role 0).
    pub fn unpause(env: Env, admin: Address) {
        admin.require_auth();
        if Self::require_role(&env, &admin, 0).is_err() {
            UnauthorizedAdminAttemptEvent {
                caller: admin,
                operation: Symbol::new(&env, "unpause"),
                timestamp: env.ledger().timestamp(),
            }
            .publish(&env);
            panic!("Unauthorized: missing required role");
        }
        env.storage().instance().set(&DataKey::Paused, &false);
        Self::extend_instance(&env);

        UnpauseEvent { admin }.publish(&env);
    }

    /// Set fee in basis points. Caller must have Admin role (0).
    /// PRE: admin has role 0
    /// POST: Config.fee_bps ≤ 10_000 (INV-6)
    pub fn set_fee_bps(env: Env, admin: Address, fee_bps: u32) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        admin.require_auth();
        if let Err(e) = Self::require_role(&env, &admin, 0) {
            UnauthorizedAdminAttemptEvent {
                caller: admin,
                operation: Symbol::new(&env, "set_fee_bps"),
                timestamp: env.ledger().timestamp(),
            }
            .publish(&env);
            return Err(e);
        }
        assert!(Self::is_valid_fee_bps(fee_bps), "fee_bps exceeds 10000");
        let mut config = Self::get_config(&env);
        config.fee_bps = fee_bps;
        env.storage().instance().set(&DataKey::Config, &config);
        Self::extend_instance(&env);

        FeeUpdateEvent { admin, fee_bps }.publish(&env);
        Ok(())
    }

    /// Set treasury address. Caller must have Admin role (0).
    pub fn set_treasury(env: Env, admin: Address, treasury: Address) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        admin.require_auth();
        if let Err(e) = Self::require_role(&env, &admin, 0) {
            UnauthorizedAdminAttemptEvent {
                caller: admin,
                operation: Symbol::new(&env, "set_treasury"),
                timestamp: env.ledger().timestamp(),
            }
            .publish(&env);
            return Err(e);
        }
        let mut config = Self::get_config(&env);
        config.treasury = treasury.clone();
        env.storage().instance().set(&DataKey::Config, &config);
        Self::extend_instance(&env);

        TreasuryUpdateEvent { admin, treasury }.publish(&env);
        Ok(())
    }

    /// Create a new prediction pool. Returns the new pool ID.
    ///
    /// PRE: end_time > current_time (INV-8)
    /// POST: Pool.state = Active, Pool.total_stake = 0
    ///
    /// # Arguments
    /// * `end_time`      - Unix timestamp after which no more predictions are accepted.
    /// * `token`        - The Stellar token contract address used for staking.
    /// * `options_count` - Number of possible outcomes (must be >= 2 and <= MAX_OPTIONS_COUNT).
    /// * `description`  - Short human-readable description of the event (max 256 bytes).
    /// * `metadata_url` - URL pointing to extended metadata, e.g. an IPFS link (max 512 bytes).
    pub fn create_pool(
        env: Env,
        end_time: u64,
        token: Address,
        options_count: u32,
        description: String,
        metadata_url: String,
    ) -> u64 {
        Self::require_not_paused(&env);

        let current_time = env.ledger().timestamp();

        // Validate: end_time must be in the future
        assert!(
            end_time > current_time,
            "end_time must be in the future"
        );

        // Validate: minimum pool duration (1 hour)
        assert!(
            end_time >= current_time + MIN_POOL_DURATION,
            "end_time must be at least 1 hour in the future"
        );

        // Validate: options_count must be at least 2 (binary or more outcomes)
        assert!(
            options_count >= 2,
            "options_count must be at least 2"
        );

        // Validate: options_count must not exceed maximum limit
        assert!(
            options_count <= MAX_OPTIONS_COUNT,
            "options_count exceeds maximum allowed value"
        );

        // Note: Token address validation is deferred to when the token is actually used.
        // This is the standard pattern in Soroban - invalid tokens will fail when
        // transfers are attempted during place_prediction.

        assert!(description.len() <= 256, "description exceeds 256 bytes");
        assert!(metadata_url.len() <= 512, "metadata_url exceeds 512 bytes");

        let pool_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PoolIdCounter)
            .unwrap_or(0);
        Self::extend_instance(&env);

        let pool = Pool {
            end_time,
            state: MarketState::Active,
            outcome: 0,
            token: token.clone(),
            total_stake: 0,
            description,
            metadata_url: metadata_url.clone(),
            options_count,
        };

        let pool_key = DataKey::Pool(pool_id);
        env.storage().persistent().set(&pool_key, &pool);
        Self::extend_persistent(&env, &pool_key);

        env.storage()
            .instance()
            .set(&DataKey::PoolIdCounter, &(pool_id + 1));
        Self::extend_instance(&env);

        PoolCreatedEvent {
            pool_id,
            end_time,
            token,
            options_count,
            metadata_url,
        }
        .publish(&env);

        pool_id
    }

    /// Resolve a pool with a winning outcome. Caller must have Operator role (1).
    /// PRE: pool.state = Active, operator has role 1
    /// POST: pool.state = Resolved, state transition valid (INV-2)
    pub fn resolve_pool(
        env: Env,
        operator: Address,
        pool_id: u64,
        outcome: u32,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        operator.require_auth();
        if let Err(e) = Self::require_role(&env, &operator, 1) {
            // 🔴 HIGH ALERT: unauthorized attempt to resolve a pool.
            UnauthorizedResolveAttemptEvent {
                caller: operator,
                pool_id,
                timestamp: env.ledger().timestamp(),
            }
            .publish(&env);
            return Err(e);
        }

        let pool_key = DataKey::Pool(pool_id);
        let mut pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");

        if pool.state != MarketState::Active {
            return Err(PredifiError::InvalidPoolState);
        }

        // Validate: outcome must be within the valid options range
        // Verify state transition validity (INV-2)
        assert!(
            outcome < pool.options_count
                && Self::is_valid_state_transition(pool.state, MarketState::Resolved),
            "outcome exceeds options_count or invalid state transition"
        );

        pool.state = MarketState::Resolved;
        pool.outcome = outcome;

        env.storage().persistent().set(&pool_key, &pool);
        Self::extend_persistent(&env, &pool_key);

        // Retrieve winning-outcome stake for the diagnostic event.
        let outcome_key = DataKey::OutcomeStake(pool_id, outcome);
        let winning_stake: i128 = env.storage().persistent().get(&outcome_key).unwrap_or(0);

        PoolResolvedEvent {
            pool_id,
            operator,
            outcome,
        }
        .publish(&env);

        // 🟢 INFO: enriched diagnostics alongside the standard resolved event.
        PoolResolvedDiagEvent {
            pool_id,
            outcome,
            total_stake: pool.total_stake,
            winning_stake,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(())
    }

    /// Cancel an active pool. Caller must have Operator role (1).
    /// PRE: pool.state = Active, operator has role 1
    /// POST: pool.state = Canceled, state transition valid (INV-2)
    pub fn cancel_pool(env: Env, operator: Address, pool_id: u64) -> Result<(), PredifiError> {
        Self::require_not_paused(&env);
        operator.require_auth();
        Self::require_role(&env, &operator, 1)?;

        let pool_key = DataKey::Pool(pool_id);
        let mut pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");

        if pool.state != MarketState::Active {
            return Err(PredifiError::InvalidPoolState);
        }

        // Verify state transition validity (INV-2)
        assert!(
            Self::is_valid_state_transition(pool.state, MarketState::Canceled),
            "Invalid state transition"
        );

        pool.state = MarketState::Canceled;

        env.storage().persistent().set(&pool_key, &pool);
        Self::extend_persistent(&env, &pool_key);

        PoolCanceledEvent { pool_id, operator }.publish(&env);
        Ok(())
    }

    /// Place a prediction on a pool.
    /// PRE: amount > 0 (INV-7), pool.state = Active, current_time < pool.end_time
    /// POST: pool.total_stake increases by amount, OutcomeStake increases by amount (INV-1)
    #[allow(clippy::needless_borrows_for_generic_args)]
    pub fn place_prediction(env: Env, user: Address, pool_id: u64, amount: i128, outcome: u32) {
        Self::require_not_paused(&env);
        user.require_auth();
        assert!(amount > 0, "amount must be positive");

        let pool_key = DataKey::Pool(pool_id);
        let mut pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");

        assert!(pool.state == MarketState::Active, "Pool is not active");
        assert!(env.ledger().timestamp() < pool.end_time, "Pool has ended");

        // Validate: outcome must be within the valid options range
        assert!(
            outcome < pool.options_count,
            "outcome exceeds options_count"
        );

        let token_client = token::Client::new(&env, &pool.token);
        token_client.transfer(&user, &env.current_contract_address(), &amount);

        let pred_key = DataKey::Prediction(user.clone(), pool_id);
        env.storage()
            .persistent()
            .set(&pred_key, &Prediction { amount, outcome });
        Self::extend_persistent(&env, &pred_key);

        // Update total stake (INV-1)
        pool.total_stake = pool.total_stake.checked_add(amount).expect("overflow");
        env.storage().persistent().set(&pool_key, &pool);
        Self::extend_persistent(&env, &pool_key);

        // Update outcome stake (INV-1)
        let outcome_key = DataKey::OutcomeStake(pool_id, outcome);
        let current_stake: i128 = env.storage().persistent().get(&outcome_key).unwrap_or(0);
        env.storage()
            .persistent()
            .set(&outcome_key, &(current_stake + amount));
        Self::extend_persistent(&env, &outcome_key);

        let count_key = DataKey::UserPredictionCount(user.clone());
        let count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);

        let index_key = DataKey::UserPredictionIndex(user.clone(), count);
        env.storage().persistent().set(&index_key, &pool_id);
        Self::extend_persistent(&env, &index_key);

        env.storage().persistent().set(&count_key, &(count + 1));
        Self::extend_persistent(&env, &count_key);

        PredictionPlacedEvent {
            pool_id,
            user: user.clone(),
            amount,
            outcome,
        }
        .publish(&env);

        // 🟡 MEDIUM ALERT: large stake detected — emit supplementary event.
        if amount >= HIGH_VALUE_THRESHOLD {
            HighValuePredictionEvent {
                pool_id,
                user,
                amount,
                outcome,
                threshold: HIGH_VALUE_THRESHOLD,
            }
            .publish(&env);
        }
    }

    /// Claim winnings from a resolved pool. Returns the amount paid out (0 for losers).
    /// PRE: pool.state ≠ Active
    /// POST: HasClaimed(user, pool) = true (INV-3), payout ≤ pool.total_stake (INV-4)
    #[allow(clippy::needless_borrows_for_generic_args)]
    pub fn claim_winnings(env: Env, user: Address, pool_id: u64) -> Result<i128, PredifiError> {
        Self::require_not_paused(&env);
        user.require_auth();

        let pool_key = DataKey::Pool(pool_id);
        let pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");
        Self::extend_persistent(&env, &pool_key);

        if pool.state == MarketState::Active {
            return Err(PredifiError::PoolNotResolved);
        }

        let claimed_key = DataKey::HasClaimed(user.clone(), pool_id);
        if env.storage().persistent().has(&claimed_key) {
            // 🔴 HIGH ALERT: repeated claim attempt on an already-claimed pool.
            SuspiciousDoubleClaimEvent {
                user: user.clone(),
                pool_id,
                timestamp: env.ledger().timestamp(),
            }
            .publish(&env);
            return Err(PredifiError::AlreadyClaimed);
        }

        // Mark as claimed immediately to prevent re-entrancy (INV-3)
        env.storage().persistent().set(&claimed_key, &true);
        Self::extend_persistent(&env, &claimed_key);

        let pred_key = DataKey::Prediction(user.clone(), pool_id);
        let prediction: Option<Prediction> = env.storage().persistent().get(&pred_key);

        if env.storage().persistent().has(&pred_key) {
            Self::extend_persistent(&env, &pred_key);
        }

        let prediction = match prediction {
            Some(p) => p,
            None => return Ok(0),
        };

        if pool.state == MarketState::Canceled {
            // Refunds: user gets exactly what they put in.
            let token_client = token::Client::new(&env, &pool.token);
            token_client.transfer(&env.current_contract_address(), &user, &prediction.amount);

            WinningsClaimedEvent {
                pool_id,
                user: user.clone(),
                amount: prediction.amount,
            }
            .publish(&env);

            return Ok(prediction.amount);
        }

        if prediction.outcome != pool.outcome {
            return Ok(0);
        }

        let outcome_key = DataKey::OutcomeStake(pool_id, pool.outcome);
        let winning_stake: i128 = env.storage().persistent().get(&outcome_key).unwrap_or(0);
        if env.storage().persistent().has(&outcome_key) {
            Self::extend_persistent(&env, &outcome_key);
        }

        if winning_stake == 0 {
            return Ok(0);
        }

        // Use pure function for winnings calculation (verifiable)
        let winnings = Self::calculate_winnings(prediction.amount, winning_stake, pool.total_stake);

        // Verify invariant: winnings ≤ total_stake (INV-4)
        assert!(winnings <= pool.total_stake, "Winnings exceed total stake");

        let token_client = token::Client::new(&env, &pool.token);
        token_client.transfer(&env.current_contract_address(), &user, &winnings);

        WinningsClaimedEvent {
            pool_id,
            user,
            amount: winnings,
        }
        .publish(&env);

        Ok(winnings)
    }

    /// Get a paginated list of a user's predictions.
    pub fn get_user_predictions(
        env: Env,
        user: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<UserPredictionDetail> {
        let count_key = DataKey::UserPredictionCount(user.clone());
        let count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);
        if env.storage().persistent().has(&count_key) {
            Self::extend_persistent(&env, &count_key);
        }

        let mut results = Vec::new(&env);

        if offset >= count || limit == 0 {
            return results;
        }

        let end = core::cmp::min(offset.saturating_add(limit), count);

        for i in offset..end {
            let index_key = DataKey::UserPredictionIndex(user.clone(), i);
            let pool_id: u64 = env
                .storage()
                .persistent()
                .get(&index_key)
                .expect("index not found");
            Self::extend_persistent(&env, &index_key);

            let pred_key = DataKey::Prediction(user.clone(), pool_id);
            let prediction: Prediction = env
                .storage()
                .persistent()
                .get(&pred_key)
                .expect("prediction not found");
            Self::extend_persistent(&env, &pred_key);

            let pool_key = DataKey::Pool(pool_id);
            let pool: Pool = env
                .storage()
                .persistent()
                .get(&pool_key)
                .expect("pool not found");
            Self::extend_persistent(&env, &pool_key);

            results.push_back(UserPredictionDetail {
                pool_id,
                amount: prediction.amount,
                user_outcome: prediction.outcome,
                pool_end_time: pool.end_time,
                pool_state: pool.state,
                pool_outcome: pool.outcome,
            });
        }

        results
    }
}

mod test;

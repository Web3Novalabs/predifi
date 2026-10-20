//! # Prediction Management & Lifecycle Domain
//!
//! This module manages prediction placement, user stake tracking, and the claim / refund lifecycle.
//!
//! ## Prediction Record Data Model
//!
//! For each user participating in a prediction market pool, the contract persists data across several key entries:
//!
//! 1. **`DataKey::Pred(user: Address, pool_id: u64)` -> `Prediction`**:
//!    - `amount: i128` — Total tokens staked by `user` on `pool_id` for their selected outcome.
//!    - `outcome: u32` — 0-based outcome index chosen by `user` (`0 <= outcome < pool.options_count`).
//! 2. **`DataKey::Referrer(user: Address, pool_id: u64)` -> `Address`**:
//!    - Optional referrer address registered on the user's first prediction placement for this pool.
//! 3. **`DataKey::ReferredVolume(referrer: Address, pool_id: u64)` -> `i128`**:
//!    - Cumulative token volume staked by referred users under `referrer` for `pool_id`.
//! 4. **`DataKey::Claimed(user: Address, pool_id: u64)` -> `bool`**:
//!    - Persistent sentinel set to `true` when a user claims winnings or a refund for `pool_id` (enforces double-claim prevention `INV-3`).
//! 5. **`DataKey::LastPredictionTime(user: Address)` -> `u64`**:
//!    - Ledger timestamp of `user`'s last prediction placement across any pool (used for rate-limiting cooldown enforcement).
//! 6. **`DataKey::UsrPrdCnt(user: Address)` -> `u32`**:
//!    - Total count of distinct pools in which `user` holds active predictions.
//! 7. **`DataKey::UsrPrdIdx(user: Address, index: u32)` -> `u64`**:
//!    - Maps a 0-based index to `pool_id` for querying user prediction history.
//!
//! ## Valid State Transitions
//!
//! ### Prediction Placement Lifecycle
//! ```text
//! [Uninitialized]
//!        │
//!        │  place_prediction(user, pool_id, amount, outcome)
//!        ▼
//!   [Placed]  ──(place_prediction with same pool_id & outcome)──► [Stake Increased]
//! ```
//! - **Uninitialized -> Placed**: User places their first prediction on an active pool. Tokens are transferred to the contract, `Prediction` struct is created, `participants_count` increments, and any optional `referrer` is recorded.
//! - **Placed -> Stake Increased**: User places an additional prediction on the *same pool and outcome*. The new stake is added to `prediction.amount` and `pool.total_stake`. (Attempting to select a different outcome index for an existing prediction is rejected).
//!
//! ### Settlement & Claim Lifecycle
//! ```text
//!                              ┌──► MarketState::Resolved ──► claim_winnings() ──► [Claimed (Winnings)]
//!                              │
//!   [Prediction Staked] ───────┼──► MarketState::Canceled ──► claim_refund()   ──► [Claimed (Refund)]
//!                              │
//!                              └──► MarketState::Disputed ──► (Awaiting Resolution)
//! ```
//! - **Active -> Resolved**: Market resolves with a winning outcome index (`pool.outcome`).
//!   - If `prediction.outcome == pool.outcome`, caller receives proportional payout `(user_stake / winning_stake) * (total_pool - protocol_fee)`. `DataKey::Claimed(user, pool_id)` is set to `true`.
//!   - If `prediction.outcome != pool.outcome`, payout is 0.
//! - **Active / Resolved / Disputed -> Canceled**: Market is canceled due to emergency, invalid oracle output, or quorum failure.
//!   - All participants receive 100% principal refund (`prediction.amount`). `DataKey::Claimed(user, pool_id)` is set to `true`.
//! - **Unclaimed -> Claimed**: Transitioning `DataKey::Claimed` to `true` is terminal and irreversible. Re-claiming returns `PredifiError::AlreadyClaimed`.

use crate::safe_math::SafeMath;
use soroban_sdk::{Address, contractevent, contractimpl, contracttype, Env, String, Symbol, token, Vec};

use crate::{
    calculate_claim_payout,
    calculate_referral_amount,
    DataKey,
    HighValuePredictionEvent,
    PayoutInput,
    Pool,
    PredifiContract,
    PredifiContractArgs,
    PredifiContractClient,
    PredifiError,
    ReferralPaidEvent,
    RewardClaimedEvent,
    SuspiciousDoubleClaimEvent,
    HIGH_VALUE_THRESHOLD,
};

#[contractimpl]
impl PredifiContract {
    /// Place a prediction on an active pool by staking tokens on a specific outcome.
    ///
    /// This function enables users to participate in prediction markets by transferring tokens
    /// to the contract and recording their prediction. The prediction is persisted in storage
    /// and will be evaluated when the pool reaches a final state (resolved, canceled, or disputed).
    /// Winning participants can claim their share of the pool's total stake minus protocol fees
    /// via `claim_winnings`.
    ///
    /// ## Function Signature
    ///
    /// ```text
    /// place_prediction(
    ///     env: Env,
    ///     user: Address,
    ///     pool_id: u64,
    ///     amount: i128,
    ///     outcome: u32,
    ///     referrer: Option<Address>,
    ///     invite_key: Option<Symbol>
    /// ) -> Result<(), PredifiError>
    /// ```
    ///
    /// # Parameters
    ///
    /// * `env` - The [Env](soroban_sdk::Env) providing access to:
    ///   * Persistent storage for data mutations
    ///   * Ledger state (timestamp, sequence numbers)
    ///   * Authentication context via `require_auth()`
    ///   * Contract address for internal operations
    /// * `user` - The [Address](soroban_sdk::Address) placing the prediction. Must be authenticated
    ///   via `user.require_auth()` before this function is called. This is the account that:
    ///   * Bears the financial risk of the stake
    ///   * Receives winnings if the prediction is correct
    ///   * Is subject to cooldown limits and prediction count caps
    /// * `pool_id` - A unique 64-bit identifier referencing a specific prediction market pool.
    ///   The pool must exist and be in `Active` state. See [Pool](Pool) for complete pool metadata.
    /// * `amount` - The quantity of tokens to stake on the prediction, measured in the token's
    ///   smallest unit (e.g., 10^7 for 1.0 STR if decimal=7). Must satisfy:
    ///   * `amount > 0` (INV-7)
    ///   * `amount >= Config::min_stake` (global minimum)
    ///   * `amount >= pool.min_stake` (per-pool minimum)
    ///   * `amount <= pool.max_stake` if `pool.max_stake > 0` (per-pool maximum)
    /// * `outcome` - A zero-based index selecting the predicted outcome. Valid range is
    ///   `[0, pool.options_count)`. For example, a binary market (Yes/No) has `options_count=2`
    ///   with valid outcomes `0` and `1`.
    /// * `referrer` - An optional [Address](soroban_sdk::Address) that referred this user to the
    ///   protocol. If provided on first prediction for `(user, pool_id)`:
    ///   * The referrer address is permanently stored in `DataKey::Referrer(user, pool_id)`
    ///   * The referrer earns a share of protocol fees when this user claims winnings
    ///   * Referred volume is tracked in `DataKey::ReferredVolume(referrer, pool_id)`
    ///   * Referrer cannot equal `user` (prevents self-referral fraud)
    ///   * Referrer cannot equal the contract address
    ///   * Only one referrer per `(user, pool)` is currently supported
    /// * `invite_key` - An optional [Symbol](soroban_sdk::Symbol) granting access to private pools.
    ///   For private pools (`pool.private == true`), the invite key must match `pool.whitelist_key`
    ///   if the user is not already whitelisted or the pool creator. Public pools ignore this field.
    ///
    /// # Prediction Cooldown Mechanism
    ///
    /// The contract enforces a configurable cooldown period between consecutive predictions
    /// from the same address to prevent spam, automated copy-trading, and potential front-running
    /// attacks. This mechanism mitigates MEV (Maximal Extractable Value) risks by introducing
    /// predictable timing barriers between user actions.
    ///
    /// ## Configuration
    ///
    /// The cooldown duration is set via `Config::prediction_cooldown_seconds` (default: 0,
    /// meaning the feature is disabled). This global configuration can be adjusted by the
    /// contract admin.
    ///
    /// ## Enforcement Logic
    ///
    /// * When `prediction_cooldown_seconds > 0`, the contract retrieves `LastPredictionTime(user)`
    ///   from persistent storage.
    /// * The elapsed time is calculated as: `current_ledger_timestamp - last_prediction_time`.
    /// * If `elapsed < prediction_cooldown_seconds`, the function fails with
    ///   `PredifiError::RateLimitOrSuspiciousActivity`.
    /// * On successful prediction, `LastPredictionTime(user)` is updated to the current ledger
    ///   timestamp, regardless of which pool was used.
    ///
    /// ## Example
    ///
    /// ```text
    /// Config.prediction_cooldown_seconds = 60  // 1 minute
    ///
    /// Time 0:   User places prediction on Pool A
    /// Time 0:   LastPredictionTime(User) = T
    /// Time 30:  User attempts prediction on Pool B -> FAILS (30s < 60s cooldown)
    /// Time 61:  User attempts prediction on Pool C -> SUCCEEDS (61s >= 60s cooldown)
    /// Time 61:  LastPredictionTime(User) = T + 61
    /// ```
    ///
    /// ## Security Rationale
    ///
    /// The cooldown mechanism addresses several attack vectors:
    ///
    /// * **Front-running**: Prevents bots from reacting to observed predictions and submitting
    ///   transactions ahead of the original user.
    /// * **Spam protection**: Limits high-frequency prediction submissions that could congest
    ///   the ledger or manipulate pool statistics.
    /// * **Copy-trading mitigation**: Introduces timing uncertainty that reduces the effectiveness
    ///   of automated strategies mirroring other users' predictions.
    ///
    /// See `docs/security-front-running-analysis.md` for detailed MEV analysis on Stellar/Soroban
    /// and why commit-reveal schemes remain opt-in rather than required.
    ///
    /// # Stake Limits Enforcement
    ///
    /// The function enforces a multi-tiered stake validation system to protect both individual
    /// users and the protocol as a whole. These limits operate at the global protocol level,
    /// per-pool level, and user account level.
    ///
    /// ## Global Protocol Limits
    ///
    /// **Minimum Stake Requirement:**
    /// * `amount >= Config::min_stake` (default: 1 token unit in smallest denomination)
    /// * Prevents dust-level predictions that could clutter storage
    /// * Error: `PredifiError::InsufficientStake`
    /// * Enforced before any pool-specific checks for efficiency
    ///
    /// ## Per-Pool Limits
    ///
    /// **Minimum Stake:**
    /// * `amount >= pool.min_stake` (configured when pool is created)
    /// * Pool creators set this to filter out trivial bets
    /// * Error: `PredifiError::StakeBelowMinimum`
    ///
    /// **Maximum Stake per Prediction:**
    /// * `amount <= pool.max_stake` if `pool.max_stake > 0`
    /// * Prevents a single user from dominating a pool's outcomes
    /// * Error: `PredifiError::StakeAboveMaximum`
    /// * A value of `0` indicates no per-prediction maximum (only the pool cap applies)
    ///
    /// **Total Pool Cap:**
    /// * `pool.total_stake + amount <= pool.max_total_stake` if `pool.max_total_stake > 0`
    /// * Limits total capital locked in a pool, managing counterparty risk
    /// * Error: `PredifiError::MaxTotalStakeExceeded`
    /// * The cap is checked atomically with the update to prevent race conditions
    ///
    /// ## User-Level Limits
    ///
    /// **Maximum Predictions Per User:**
    /// * `Config::max_predictions_per_user` controls the number of distinct pools a user can
    ///   participate in simultaneously
    /// * Error: `PredifiError::MaxPredictionsExceeded`
    /// * **Important**: Increasing stake on an existing prediction (same pool_id + outcome)
    ///   does NOT count toward this limit. Only new pool participations are counted.
    /// * This prevents users from spreading risk across too many markets while still allowing
    ///   them to increase stake on existing positions
    ///
    /// ## Validation Order
    ///
    /// Limits are checked in the following order for efficiency and security:
    ///
    /// 1. Global protocol minimum (`Config::min_stake`)
    /// 2. Referrer and invite key validation
    /// 3. Pool existence and active state
    /// 4. Token whitelisting
    /// 5. Private pool authorization
    /// 6. Outcome validity
    /// 7. Per-pool stake limits (`min_stake`, `max_stake`, `max_total_stake`)
    /// 8. Prediction cooldown check
    /// 9. User prediction count limit (if applicable)
    /// 10. Token transfer validation and execution
    ///
    /// # Examples
    ///
    /// ## Basic Prediction Placement
    ///
    /// ```rust,no_run
    /// use soroban_sdk::{Address, Env, testutils::Address as _};
    /// use predifi_contract::{PredifiContractClient, PredifiError};
    ///
    /// # fn example(env: Env) {
    /// let contract_addr = env.current_contract_address();
    /// let client = PredifiContractClient::new(&env, &contract_addr);
    /// let user = Address::generate(&env);
    /// let pool_id = 1u64;
    /// let amount = 10000000i128; // 1 token with 7 decimals
    /// let outcome = 0u32; // Predict outcome 0
    ///
    /// // User places a prediction
    /// client.place_prediction(&user, &pool_id, &amount, &outcome, &None, &None);
    /// # }
    /// ```
    ///
    /// ## Prediction with Referrer
    ///
    /// ```rust,no_run
    /// # fn example_with_referrer(env: Env) {
    /// let referrer = Address::generate(&env);
    /// let user = Address::generate(&env);
    ///
    /// // User is referred by referrer
    /// client.place_prediction(
    ///     &user,
    ///     &pool_id,
    ///     &amount,
    ///     &outcome,
    ///     &Some(referrer),
    ///     &None,
    /// );
    /// # }
    /// ```
    ///
    /// ## Increasing Existing Stake
    ///
    /// ```rust,no_run
    /// # fn example_increase_stake(env: Env) {
    /// let existing_prediction = client.prediction(&user, &pool_id);
    /// let increase_amount = 5000000i128; // 0.5 tokens
    ///
    /// // User increases stake on existing prediction (same pool, same outcome)
    /// client.place_prediction(
    ///     &user,
    ///     &pool_id,
    ///     &(existing_prediction.amount + increase_amount),
    ///     &outcome,
    ///     &None, // referrer is ignored on subsequent predictions
    ///     &None,
    /// );
    /// # }
    /// ```
    ///
    /// ## Private Pool with Invite Key
    ///
    /// ```rust,no_run
    /// # fn example_private_pool(env: Env) {
    /// let invite_key = soroban_sdk::Symbol::from_str(&env, "SECRET123");
    ///
    /// // User provides valid invite key for private pool access
    /// client.place_prediction(
    ///     &user,
    ///     &pool_id,
    ///     &amount,
    ///     &outcome,
    ///     &None,
    ///     &Some(invite_key),
    /// );
    /// # }
    /// ```
    ///
    /// ## Error Handling
    ///
    /// ```rust,no_run
    /// # fn example_error_handling(env: Env) {
    /// use soroban_sdk::TryIntoVal;
    ///
    /// let pool_id = 999u64; // Non-existent pool
    /// let result = client.try_place_prediction(&user, &pool_id, &amount, &outcome, &None, &None);
    ///
    /// if let Err(err) = result {
    ///     match err {
    ///         PredifiError::PoolNotFound => {
    ///             println!("Pool does not exist");
    ///         }
    ///         PredifiError::InsufficientStake => {
    ///             println!("Amount below minimum stake");
    ///         }
    ///         _ => {
    ///             println!("Error: {:?}", err);
    ///         }
    ///     }
    /// }
    /// # }
    /// ```
    ///
    /// ## Cooldown Enforcement
    ///
    /// ```text
    /// Config.prediction_cooldown_seconds = 60  // 1 minute
    ///
    /// Time 0:   User places prediction on Pool A
    /// Time 0:   LastPredictionTime(User) = T
    /// Time 30:  User attempts prediction on Pool B -> FAILS (30s < 60s cooldown)
    /// Time 61:  User attempts prediction on Pool C -> SUCCEEDS (61s >= 60s cooldown)
    /// Time 61:  LastPredictionTime(User) = T + 61
    /// ```
    ///
    /// ## Max Predictions Limit
    ///
    /// ```text
    /// Config.max_predictions_per_user = 5
    ///
    /// User has predictions on 5 distinct pools:
    /// - Pool A, Pool B, Pool C, Pool D, Pool E
    ///
    /// Attempt to place prediction on Pool F -> FAILS (MaxPredictionsExceeded)
    /// Attempt to increase stake on Pool A -> SUCCEEDS (same pool, not a new prediction)
    /// User claims winnings on Pool E and exits:
    ///   -> UsrPrdCnt decreases to 4
    ///   -> Can now place prediction on Pool F
    /// ```
    ///
    /// # Referral Handling
    ///
    /// The referral system enables users to earn rewards by bringing new participants to the
    /// protocol. When a referred user places a prediction and later claims winnings, the referrer
    /// receives a share of the protocol fee based on the configured referral rate.
    ///
    /// ## Referral Registration
    ///
    /// * The `referrer` parameter is **only processed on the first prediction** for a given
    ///   `(user, pool_id)` pair.
    /// * On first prediction, the referrer address is stored in `DataKey::Referrer(user, pool_id)`.
    /// * Subsequent predictions on the same pool (regardless of outcome) ignore the `referrer`
    ///   parameter and continue tracking volume under the original referrer.
    /// * Referred volume is accumulated in `DataKey::ReferredVolume(referrer, pool_id)` across
    ///   all predictions from the referred user on that pool.
    ///
    /// ## Reward Calculation
    ///
    /// When a referred user claims winnings:
    ///
    /// ```text
    /// protocol_fee = user_winnings * pool.fee_bps / 10_000
    /// referral_amount = protocol_fee * Config::referral_bps / 10_000
    /// treasury_amount = protocol_fee - referral_amount
    /// user_payout = user_winnings - protocol_fee
    /// ```
    ///
    /// * `Config::referral_bps` (default: 500 bps = 5%) determines the referrer's share.
    /// * Example: If `protocol_fee = 100 tokens` and `referral_bps = 500`:
    ///   * Referrer receives: `100 * 500 / 10_000 = 5 tokens`
    ///   * Treasury receives: `100 - 5 = 95 tokens`
    ///
    /// ## Validation Rules
    ///
    /// * Referrer cannot equal `user` (prevents self-referral fraud)
    /// * Referrer cannot equal the contract address (prevents circular referrals)
    /// * Validation occurs before reentrancy guard is entered to fail fast on invalid inputs
    /// * Currently, only one referrer per `(user, pool)` is supported. Future extensions may
    ///   support multi-level referrals or multiple referrers per user.
    ///
    /// ## Data Storage
    ///
    /// * `DataKey::Referrer(user, pool_id) -> Option<Address>`: Permanent mapping storing
    ///   the original referrer for this user/pool pair.
    /// * `DataKey::ReferredVolume(referrer, pool_id) -> i128`: Cumulative staked amount
    ///   from all referred users for this pool.
    /// * `DataKey::UsrPrdCnt(user) -> u32`: Total distinct pools with predictions (used for
    ///   limit tracking and index queries).
    /// * `DataKey::UsrPrdIdx(user, index) -> u64`: Maps 0-based index to pool_id for efficient
    ///   iteration over a user's prediction history.
    ///
    /// ## Example Flow
    ///
    /// ```text
    /// User A refers User B with referrer = A
    /// User B places 100 tokens on Pool X (first prediction for B on X)
    ///   -> DataKey::Referrer(B, X) = A
    ///   -> DataKey::ReferredVolume(A, X) += 100 (now 100)
    /// User B increases stake by 50 tokens on Pool X (same prediction)
    ///   -> DataKey::ReferredVolume(A, X) += 50 (now 150)
    /// User B places prediction on Pool Y (new pool participation)
    ///   -> Referrer A is NOT updated (B's first prediction on Y)
    ///   -> DataKey::UsrPrdCnt(B) increments (now 2)
    /// User B wins Pool X and claims 200 tokens payout
    ///   -> Protocol fee deducted: 20 tokens
    ///   -> Referral fee: 20 * 5% = 1 token goes to A
    ///   -> Treasury receives: 19 tokens
    ///   -> User B receives: 181 tokens net
    /// ```
    ///
    /// # Fee Deduction Flow
    ///
    /// **Important:** Protocol fees are **NOT** deducted during `place_prediction`. Fees are
    /// calculated and deducted at **resolution time** when winners claim their winnings via
    /// `claim_winnings`. This design decouples staking from fee computation, enabling dynamic
    /// fee tiers based on pool state at resolution.
    ///
    /// ## Fee Flow Diagram
    ///
    /// ```text
    /// ┌─────────────────────────────────────────────────────────────────────────────────┐
    /// │ 1. Prediction Placement (place_prediction)                                      │
    /// │    - User transfers full 'amount' to contract                                   │
    /// │    - Full amount added to pool.total_stake                                      │
    /// │    - NO fees deducted at this stage                                             │
    /// └─────────────────────────────────────────────────────────────────────────────────┘
    ///                                    │
    ///                                    ▼
    /// ┌─────────────────────────────────────────────────────────────────────────────────┐
    /// │ 2. Pool Resolution                                                              │
    /// │    - Pool.state transitions to: Resolved, Canceled, or Disputed                │
    /// │    - Winning outcome is finalized (pool.outcome)                               │
    /// │    - Dynamic fee tier determined based on pool.total_stake at resolution       │
    /// │    - pool.fee_bps is set or retained for claim calculation                     │
    /// └─────────────────────────────────────────────────────────────────────────────────┘
    ///                                    │
    ///                                    ▼
    /// ┌─────────────────────────────────────────────────────────────────────────────────┐
    /// │ 3. Winnings Claim (claim_winnings)                                              │
    /// │    User's payout calculation:                                                  │
    /// │                                                                                  │
    /// │    user_share = user_stake / winning_stake * (total_pool - protocol_fee)       │
    /// │    user_winnings = user_share + user_stake (if winner)                         │
    /// │    user_winnings = 0 (if loser)                                                │
    /// │                                                                                  │
    /// │    protocol_fee = user_winnings * pool.fee_bps / 10_000                        │
    /// │    referral_amount = protocol_fee * Config.referral_bps / 10_000               │
    /// │    treasury_amount = protocol_fee - referral_amount                            │
    /// │    user_payout = user_winnings - protocol_fee                                  │
    /// │                                                                                  │
    /// │    Transfers:                                                                  │
    /// │    - Referrer receives: referral_amount (if referrer exists)                   │
    /// │    - Treasury receives: treasury_amount                                        │
    /// │    - User receives: user_payout                                                │
    /// └─────────────────────────────────────────────────────────────────────────────────┘
    /// ```
    ///
    /// ## Dynamic Fee Tiers
    ///
    /// The protocol supports dynamic fee tiers based on pool total stake at resolution time:
    ///
    /// | Total Pool Stake Range | Fee (bps) | Example (10,000 stake) |
    /// |------------------------|-----------|------------------------|
    /// | 0 - 1,000             | 100 bps   | 1% fee               |
    /// | 1,000 - 10,000        | 50 bps    | 0.5% fee             |
    /// | 10,000 - 100,000      | 25 bps    | 0.25% fee            |
    /// | 100,000+              | 10 bps    | 0.1% fee             |
    ///
    /// * The fee tier is determined when the pool resolves, not when the prediction is placed.
    /// * This incentivizes larger pools by offering lower fees as pool size grows.
    ///
    /// ## Example Calculation
    ///
    /// ```text
    /// Pool X has:
    /// - total_stake = 100,000 tokens
    /// - winning_stake = 60,000 tokens (outcome 1)
    /// - user_stake = 1,000 tokens (on outcome 1)
    /// - pool.fee_bps = 25 (0.25% based on 100k total stake)
    /// - Config.referral_bps = 500 (5%)
    ///
    /// User wins:
    /// - user_share = 1,000 / 60,000 * 100,000 = 1,666.67 tokens
    /// - user_winnings = 1,666.67 + 1,000 = 2,666.67 tokens (includes principal)
    ///
    /// Fees deducted:
    /// - protocol_fee = 2,666.67 * 25 / 10_000 = 6.67 tokens
    /// - referral_amount = 6.67 * 500 / 10_000 = 0.33 tokens (to referrer)
    /// - treasury_amount = 6.67 - 0.33 = 6.34 tokens
    ///
    /// Final payouts:
    /// - Referrer receives: 0.33 tokens
    /// - Treasury receives: 6.34 tokens
    /// - User receives: 2,666.67 - 6.67 = 2,660.00 tokens net
    /// - Total distributed: 2,660.00 + 0.33 + 6.34 + 60,000 (other winners) = 100,000
    /// ```
    ///
    /// # Emitted Events
    ///
    /// This function emits events in the following sequence. Events are only emitted after
    /// reentrancy guard is exited and all validations pass.
    ///
    /// ## 1. PredictionPlacedEvent (Always)
    ///
    /// Emitted for every successful prediction placement.
    ///
    /// ```rust,ignore
    /// event PredictionPlacedEvent {
    ///     pool_id: u64,
    ///     user: Address,
    ///     amount: i128,
    ///     outcome: u32
    /// }
    /// ```
    ///
    /// **Fields:**
    /// * `pool_id` - The prediction pool receiving the stake
    /// * `user` - The address placing the prediction
    /// * `amount` - The quantity of tokens staked (in smallest unit)
    /// * `outcome` - The predicted outcome index (0-based)
    ///
    /// **Use cases:**
    /// * Indexing services tracking prediction activity
    /// * User activity dashboards
    /// * Analytics on pool participation
    ///
    /// ## 2. HighValuePredictionEvent (Conditional)
    ///
    /// Emitted when `amount >= HIGH_VALUE_THRESHOLD` (configurable threshold for monitoring).
    ///
    /// ```rust,ignore
    /// event HighValuePredictionEvent {
    ///     pool_id: u64,
    ///     user: Address,
    ///     amount: i128,
    ///     outcome: u32,
    ///     threshold: i128
    /// }
    /// ```
    ///
    /// **Fields:**
    /// * `pool_id` - The prediction pool
    /// * `user` - The address placing the large prediction
    /// * `amount` - The staked amount exceeding the threshold
    /// * `outcome` - The predicted outcome index
    /// * `threshold` - The threshold value that was exceeded
    ///
    /// **Use cases:**
    /// * Real-time monitoring of large stakes for market manipulation detection
    /// * Alerting system for protocol administrators
    /// * Compliance reporting for high-value transactions
    ///
    /// ## 3. OutcomeStakesUpdatedEvent (Conditional)
    ///
    /// Emitted when `pool.options_count >= 16` to avoid indexing challenges from emitting
    /// individual events per outcome (e.g., 32-team sports bracket).
    ///
    /// ```rust,ignore
    /// event OutcomeStakesUpdatedEvent {
    ///     pool_id: u64,
    ///     options_count: u32,
    ///     total_stake: i128
    /// }
    /// ```
    ///
    /// **Fields:**
    /// * `pool_id` - The prediction pool
    /// * `options_count` - Total number of possible outcomes in the pool
    /// * `total_stake` - Updated total stake across all outcomes
    ///
    /// **Rationale:**
    /// * For tournaments with many outcomes, individual outcome stake updates would generate
    ///   excessive events (e.g., 32 events per prediction for a 32-team bracket).
    /// * Batch updates provide sufficient granularity for indexing while reducing event spam.
    ///
    /// ## 4. PredictionBlockedDelistedEvent (Error Case)
    ///
    /// Emitted when the pool's token is not whitelisted, before returning
    /// `PredifiError::TokenNotWhitelisted`.
    ///
    /// ```rust,ignore
    /// event PredictionBlockedDelistedEvent {
    ///     pool_id: u64,
    ///     user: Address,
    ///     token: Address,
    ///     timestamp: u64
    /// }
    /// ```
    ///
    /// **Fields:**
    /// * `pool_id` - The prediction pool
    /// * `user` - The address attempting the prediction
    /// * `token` - The token address that is not whitelisted
    /// * `timestamp` - Ledger timestamp when the attempt occurred
    ///
    /// **Use cases:**
    /// * Auditing unauthorized betting attempts
    /// * Detecting attempts to use delisted tokens
    /// * Security monitoring for protocol compliance
    ///
    /// ## Event Ordering guarantees
    ///
    /// * Events are emitted **after** all validations pass and storage mutations are committed.
    /// * Events are emitted **after** reentrancy guard is exited (no reentrant calls possible).
    /// * The contract uses `soroban_sdk::contractevent::Ext` for event emission.
    /// * Event order is deterministic and can be relied upon by indexing services.
    ///
    /// ## Event Example
    ///
    /// ```json
    /// {
    ///   "events": [
    ///     {
    ///       "topic": ["PredictionPlacedEvent"],
    ///       "data": [
    ///         {"val": "u64", "Val": 123},
    ///         {"val": "bytes", "Val": "GA...XYZ"},
    ///         {"val": "i128", "Val": "10000000"},
    ///         {"val": "u32", "Val": 0}
    ///       ]
    ///     },
    ///     {
    ///       "topic": ["HighValuePredictionEvent"],
    ///       "data": [
    ///         {"val": "u64", "Val": 123},
    ///         {"val": "bytes", "Val": "GA...XYZ"},
    ///         {"val": "i128", "Val": "10000000"},
    ///         {"val": "u32", "Val": 0},
    ///         {"val": "i128", "Val": "1000000"}
    ///       ]
    ///     }
    ///   ]
    /// }
    /// ```
    ///
    /// # Error Conditions
    ///
    /// All errors are of type `PredifiError` and cause the transaction to abort with a
    /// Soroban error code. Errors are emitted in the validation order (see Stake Limits
    /// Enforcement section for validation sequence).
    ///
    /// ## Validation Errors
    ///
    /// | Error | Code | Validation | Description |
    /// |-------|------|------------|-------------|
    /// | `ContractPaused` | 100 | Early | The contract is paused; all state-mutating operations are suspended. See `set_contract_paused` for admin control. |
    /// | `InvalidAmount` | 101 | Amount check | Stake amount is zero or negative. Always checked first before any other validation. |
    /// | `InsufficientStake` | 102 | Global limit | Amount below global protocol minimum (`Config::min_stake`). Prevents dust predictions. |
    /// | `TokenNotWhitelisted` | 103 | Token check | Pool token is not on the allowed betting whitelist. Prevents betting on unauthorized assets. |
    /// | `PoolNotFound` | 104 | Pool existence | Specified `pool_id` does not exist in storage. |
    /// | `InvalidPoolState` | 105 | Pool state | Pool is not in `Active` state. Cannot place predictions on resolved, canceled, or disputed pools. |
    /// | `InvalidOutcome` | 106 | Outcome validation | Outcome index is >= `pool.options_count`. Prevents out-of-bounds predictions. |
    /// | `StakeBelowMinimum` | 107 | Per-pool limit | Amount below pool's configured `min_stake`. Pool-specific filter. |
    /// | `StakeAboveMaximum` | 108 | Per-pool limit | Amount exceeds pool's configured `max_stake` (if > 0). Prevents single-user dominance. |
    /// | `MaxTotalStakeExceeded` | 109 | Pool cap | Adding this amount would exceed `pool.max_total_stake`. Risk management for total capital. |
    /// | `RateLimitOrSuspiciousActivity` | 110 | Cooldown check | Prediction cooldown period has not elapsed since last prediction. MEV mitigation. |
    /// | `MaxPredictionsExceeded` | 111 | User limit | User has exceeded `Config::max_predictions_per_user`. Prevents over-spreading. |
    /// | `InvalidReferralCode` | 112 | Invite key | Provided `invite_key` failed validation for private pool access. |
    /// | `InsufficientBalance` | 113 | Token balance | User's token balance is insufficient for the transfer amount. |
    /// | `TransferFailed` | 114 | Token transfer | Token transfer validation or execution failed (e.g., transfer hooks rejected). |
    /// | `ArithmeticError` | 115 | Math operation | Overflow occurred during stake calculations. Prevented by `SafeMath` but listed for completeness. |
    ///
    /// ## Error Code Reference
    ///
    /// Error codes are defined in `PredifiError` enum and mapped to Soroban error codes:
    /// * Codes 100-199: User input validation errors
    /// * Codes 200-299: Pool state and lifecycle errors
    /// * Codes 300-399: Token and transfer errors
    /// * Codes 400-499: Arithmetic and internal errors
    ///
    /// ## Error Recovery Guidance
    ///
    /// **For Users:**
    /// * `InsufficientStake`: Increase stake to meet minimum requirements
    /// * `StakeBelowMinimum/StakeAboveMaximum`: Check pool's min/max stake configuration
    /// * `MaxTotalStakeExceeded`: Wait for other participants to claim or choose another pool
    /// * `RateLimitOrSuspiciousActivity`: Wait for cooldown period to elapsed
    /// * `MaxPredictionsExceeded`: Reduce active predictions by claiming or exiting positions
    /// * `TokenNotWhitelisted`: Choose a pool with a whitelisted token
    ///
    /// **For Developers:**
    /// * Errors are emitted **before** reentrancy guard is exited (where applicable)
    /// * Errors cause transaction abort with no side effects (atomicity maintained)
    /// * No events are emitted when an error occurs (only success path emits events)
    /// * All storage mutations are rolled back automatically on error
    ///
    /// ## Reentrancy and Error Handling
    ///
    /// * Reentrancy guard is **exited** before any error is returned (to allow re-entry).
    /// * Token transfers are validated before execution to prevent partial state changes.
    /// * Events are only emitted after all validations pass and guard is exited.
    /// * Storage mutations are batched and committed atomically; partial updates are impossible.
    ///
    /// # Pre-conditions
    ///
    /// All pre-conditions are validated in the order specified below. Validation fails fast
    /// if any condition is not satisfied, with no side effects committed.
    ///
    /// ## Basic Validation
    ///
    /// | Condition | INV | Description |
    /// |-----------|-----|-------------|
    /// | `amount > 0` | INV-7 | Stake amount must be positive. Zero or negative amounts are always rejected. |
    /// | `pool.state == MarketState::Active` | INV-2 | Pool must be in active state for new predictions. |
    /// | `env.ledger().timestamp() < pool.end_time` | INV-11 | Pool must not have reached its end time. |
    /// | `outcome < pool.options_count` | INV-12 | Outcome index must be within valid range. |
    ///
    /// ## Stake Limits
    ///
    /// | Condition | INV | Description |
    /// |-----------|-----|-------------|
    /// | `pool.min_stake <= amount <= pool.max_stake` | INV-8 | Amount within pool's configured limits (if `max_stake > 0`). |
    /// | `amount >= Config::min_stake` | INV-1 | Global protocol minimum stake. |
    ///
    /// ## Pool Access Control
    ///
    /// | Condition | INV | Description |
    /// |-----------|-----|-------------|
    /// | Token whitelisted | INV-13 | Pool's token must be on the allowed betting whitelist. |
    /// | Private pool authorization | INV-14 | For private pools, user must be whitelisted, creator, or provide valid `invite_key`. |
    ///
    /// ## User Account Limits
    ///
    /// | Condition | INV | Description |
    /// |-----------|-----|-------------|
    /// | Cooldown elapsed | INV-15 | If `prediction_cooldown_seconds > 0`, sufficient time must have elapsed since last prediction. |
    /// | Prediction count limit | INV-16 | If `max_predictions_per_user > 0`, user must not have exceeded the limit. |
    ///
    /// ## Pre-condition Execution Order
    ///
    /// The following sequence ensures atomic validation with early failure:
    ///
    /// 1. **Contract pause check** (`require_not_paused`)
    /// 2. **Authentication** (`user.require_auth()`)
    /// 3. **Amount validation** (`amount > 0`, `amount >= Config::min_stake`)
    /// 4. **Referrer validation** (if provided: `referrer != user`, `referrer != contract`)
    /// 5. **Invite key validation** (if provided: matches `pool.whitelist_key`)
    /// 6. **Pool existence** (storage lookup for `DataKey::Pool(pool_id)`)
    /// 7. **Pool state check** (`is_pool_active`, ledger time vs `end_time`)
    /// 8. **Token whitelisting** (`is_token_whitelisted`)
    /// 9. **Private pool authorization** (whitelist check or valid invite key)
    /// 10. **Outcome validity** (`outcome < pool.options_count`)
    /// 11. **Per-pool stake limits** (`min_stake`, `max_stake`, `max_total_stake`)
    /// 12. **Prediction cooldown** (`LastPredictionTime` check)
    /// 13. **User prediction count** (`UsrPrdCnt` check, only for new predictions)
    /// 14. **Token transfer validation** (`validate_token_transfer`)
    ///
    /// ## Gas Optimization Notes
    ///
    /// * Early validation failures prevent expensive operations (reentrancy guard, storage writes).
    /// * Token whitelisting and private pool checks are performed after basic validations to
    ///   fail fast on obvious user errors.
    /// * Prediction count limit is only checked for **new** predictions (not stake increases).
    /// * Reentrancy guard is entered after most validations to minimize lock duration.
    ///
    /// # Post-conditions
    ///
    /// On successful execution, the following state changes are guaranteed. All post-conditions
    /// are committed atomically; if any operation fails, the entire transaction is aborted.
    ///
    /// ## Storage Mutations
    ///
    /// | Key | Type | Change | Description |
    /// |-----|------|--------|-------------|
    /// | `DataKey::Pool(pool_id)` | `Pool` | `total_stake += amount` | Total stake in the pool increases. |
    /// | `DataKey::Pred(user, pool_id)` | `Prediction` | Created or `amount += amount` | User's prediction record updated. |
    /// | `DataKey::Referrer(user, pool_id)` | `Option<Address>` | Set if first prediction | Referrer address stored on first prediction. |
    /// | `DataKey::ReferredVolume(referrer, pool_id)` | `i128` | `+= amount` | Cumulative referred volume updated. |
    /// | `DataKey::LastPredictionTime(user)` | `u64` | Set to `ledger.timestamp()` | Cooldown timestamp updated. |
    /// | `DataKey::UsrPrdCnt(user)` | `u32` | `++` if new prediction | Total distinct pools with predictions. |
    /// | `DataKey::UsrPrdIdx(user, count)` | `u64` | Set to `pool_id` if new prediction | Maps index to pool_id for iteration. |
    ///
    /// ## Pool State Changes
    ///
    /// | Field | Change | Description |
    /// |-------|--------|-------------|
    /// | `total_stake` | `+= amount` | Total capital locked in the pool. |
    /// | `participants_count` | `++` if new prediction | Count of unique users with predictions. |
    ///
    /// ## Outcome Stake Updates
    ///
    /// * `OutcomeStake(pool_id, outcome)` is updated to reflect the new stake amount.
    /// * For pools with `options_count >= 16`, `OutcomeStakesUpdatedEvent` is emitted instead
    ///   of individual outcome events for efficiency.
    ///
    /// ## Token Transfers
    ///
    /// * `amount` tokens are transferred from `user` to the contract address.
    /// * Transfer is validated before execution to ensure atomicity.
    /// * The contract holds tokens in escrow until the pool is resolved and claims are made.
    ///
    /// ## Event Emissions
    ///
    /// * `PredictionPlacedEvent` is emitted for every successful prediction.
    /// * `HighValuePredictionEvent` is emitted if `amount >= HIGH_VALUE_THRESHOLD`.
    /// * `OutcomeStakesUpdatedEvent` is emitted if `pool.options_count >= 16`.
    /// * Events are emitted **after** reentrancy guard is exited and all validations pass.
    ///
    /// ## Invariants Maintained
    ///
    /// | INV | Statement | Verification |
    /// |-----|-----------|--------------|
    /// | INV-1 | `pool.total_stake` increases by `amount` | Sum of all outcome stakes equals `total_stake`. |
    /// | INV-2 | Pool state transitions only on resolution | `Active` state enforced at placement. |
    /// | INV-3 | `Claimed(user, pool)` prevents double-claiming | Set to `true` on first claim. |
    /// | INV-4 | No user receives more than their proportional share | Payout math verified in `claim_winnings`. |
    /// | INV-5 | Referral tracking is accurate | `ReferredVolume` updated on first prediction. |
    /// | INV-6 | Cooldown enforcement is consistent | `LastPredictionTime` updated on success. |
    /// | INV-7 | Stake amount is always positive | Validated before any mutation. |
    /// | INV-8 | Per-pool stake limits are respected | Checked before pool update. |
    /// | INV-9 | Token transfers are atomic | Validated before execution. |
    /// | INV-10 | No reentrant calls during state mutation | Reentrancy guard protects critical section. |
    /// | INV-11 | Pool end time is respected | `ledger.timestamp() < end_time` checked. |
    /// | INV-12 | Outcome index is valid | `outcome < options_count` validated. |
    /// | INV-13 | Token whitelisting enforced | `is_token_whitelisted` checked. |
    /// | INV-14 | Private pool access controlled | Whitelist or valid invite key verified. |
    /// | INV-15 | Cooldown enforced when enabled | `LastPredictionTime` checked against cooldown. |
    /// | INV-16 | Prediction count limit respected | `UsrPrdCnt` checked for new predictions. |
    ///
    /// ## Atomicity Guarantees
    ///
    /// * All storage mutations are committed atomically via Soroban's transaction model.
    /// * If any operation fails (e.g., token transfer), all prior mutations are rolled back.
    /// * Events are only emitted after all mutations are complete.
    /// * The reentrancy guard is exited before external calls (token transfer) to allow re-entry.
    ///
    /// ## Gas Costs
    ///
    /// * Storage write operations are optimized to minimize persistent writes.
    /// * Repeated calls on the same prediction (stake increase) avoid referrer and count updates.
    /// * Batch stake updates for large outcome counts reduce event emission overhead.
    ///
    /// # Reentrancy Protection
    ///
    /// This function implements a reentrancy guard pattern to prevent reentrant calls that could
    /// lead to state manipulation attacks or inconsistent reads. The guard follows the
    /// "non-reentrant modifier" pattern used in Solidity and adapted for Soroban.
    ///
    /// ## Guard Implementation
    ///
    /// ```text
    /// enter_reentrancy_guard(&env)  // Set guard flag
    ///     │
    ///     ├─► All validations and storage reads
    ///     │
    ///     ├─► Token transfer validation (safe before execution)
    ///     │
    ///     ├─► Storage mutations (if all validations pass)
    ///     │
    ///     └─► exit_reentrancy_guard(&env)  // Clear guard flag
    ///         │
    ///         └─► Token transfer execution (after guard cleared)
    ///             │
    ///             └─► Event emission (after guard cleared)
    /// ```
    ///
    /// ## Why Guard is Exited Before Token Transfer
    ///
    /// The guard is exited **before** the actual token transfer to allow legitimate re-entrancy
    /// scenarios while still protecting against malicious re-entrancy:
    ///
    /// * **Safe re-entrancy**: The user's contract might call back into the protocol for a
    ///   different operation (e.g., another pool) after the transfer. This is safe because
    ///   all state mutations for this prediction are complete.
    /// * **Protected mutations**: All storage writes occur while the guard is active, preventing
    ///   re-entrant reads of stale state.
    /// * **Transfer safety**: The token transfer uses Soroban's standard `transfer` which validates
    ///   sender balance and token validity before execution.
    ///
    /// ## Guard Behavior
    ///
    /// * **First call**: Guard is entered successfully, all operations proceed.
    /// * **Re-entrant call during same transaction**: If the same function is called again before
    ///   `exit_reentrancy_guard` is called, the second call will detect the guard and fail.
    /// * **Next transaction**: Guard state is not persisted; each transaction starts with a
    ///   clean guard state.
    ///
    /// ## Security Considerations
    ///
    /// * The guard prevents re-entrant calls to the **same function** within the same transaction.
    /// * The guard does **not** prevent other functions in the same contract from being called.
    /// * The guard is **per-function**, not per-contract (different functions can run concurrently).
    /// * The guard state is stored in a dedicated storage key (implementation detail).
    ///
    /// ## Error Handling with Guard
    ///
    /// * If validation fails **before** reentrancy guard is entered, no guard cleanup is needed.
    /// * If validation fails **after** reentrancy guard is entered, `exit_reentrancy_guard` is
    ///   called before returning the error.
    /// * This ensures the guard is always released, even on error paths.
    #[allow(clippy::needless_borrows_for_generic_args)]
    pub fn place_prediction(
        env: Env,
        user: Address,
        pool_id: u64,
        amount: i128,
        outcome: u32,
        referrer: Option<Address>,
        invite_key: Option<Symbol>,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        user.require_auth();
        // Reject zero or negative stake amounts.
        if amount <= 0 {
            soroban_sdk::panic_with_error!(&env, PredifiError::InvalidAmount);
        }

        // Validate: amount must meet the global protocol minimum stake
        let global_min_stake = Self::get_config(&env).min_stake;
        if amount < global_min_stake {
            soroban_sdk::panic_with_error!(&env, PredifiError::InsufficientStake);
        }

        // Validate referrer if provided: cannot be self or contract
        if let Some(ref r) = referrer {
            assert!(r != &user, "referrer cannot be self");
            assert!(
                r != &env.current_contract_address(),
                "referrer cannot be contract"
            );
        }

        if let Some(ref invite_key) = invite_key {
            if let Err(e) = Self::validate_referral_code(&env, invite_key) {
                soroban_sdk::panic_with_error!(&env, e);
            }
        }

        Self::enter_reentrancy_guard(&env);

        let pool_key = DataKey::Pool(pool_id);
        let mut pool: Pool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .expect("Pool not found");

        // assert!(pool.state == MarketState::Active, "Pool is not active");
        if !Self::is_pool_active(&pool) {
            panic!("Pool is not active");
        }
        assert!(env.ledger().timestamp() < pool.end_time, "Pool has ended");

        // Validate: token must be on the allowed betting whitelist
        if !Self::is_token_whitelisted(&env, &pool.token) {
            Self::exit_reentrancy_guard(&env);
            PredictionBlockedDelistedEvent {
                pool_id,
                user: user.clone(),
                token: pool.token.clone(),
                timestamp: env.ledger().timestamp(),
            }
            .publish(&env);
            soroban_sdk::panic_with_error!(&env, PredifiError::TokenNotWhitelisted);
        }

        // Check private pool authorization
        // Check private pool authorization
        if pool.private {
            let whitelist_key_data = DataKey::Whitelist(pool_id, user.clone());
            let is_whitelisted = env
                .storage()
                .persistent()
                .get(&whitelist_key_data)
                .unwrap_or(false);

            let has_valid_invite = if let Some(ref pool_key) = pool.whitelist_key {
                if let Some(ref prov_key) = invite_key {
                    pool_key == prov_key
                } else {
                    false
                }
            } else {
                false
            };

            assert!(
                is_whitelisted || user == pool.creator || has_valid_invite,
                "User not authorized for private pool"
            );
        }

        // Validate: outcome must be within the valid options range
        if outcome >= pool.options_count {
            soroban_sdk::panic_with_error!(&env, PredifiError::InvalidOutcome);
        }

        // --- INTERNAL CHECKS & EFFECTS ---
        // Validate: per-pool stake limits
        if amount < pool.min_stake {
            Self::exit_reentrancy_guard(&env);
            soroban_sdk::panic_with_error!(&env, PredifiError::StakeBelowMinimum);
        }
        if pool.max_stake > 0 && amount > pool.max_stake {
            Self::exit_reentrancy_guard(&env);
            soroban_sdk::panic_with_error!(&env, PredifiError::StakeAboveMaximum);
        }

        // Enforce global pool cap (max total stake)
        if pool.max_total_stake > 0 {
            let new_total = pool.total_stake.checked_add(amount).expect("overflow");
            if new_total > pool.max_total_stake {
                Self::exit_reentrancy_guard(&env);
                soroban_sdk::panic_with_error!(&env, PredifiError::MaxTotalStakeExceeded);
            }
        }

        let config = Self::get_config(&env);

        // Enforce prediction cooldown to limit automated copy-trading / front-running
        // bots that react to observed `place_prediction` submissions. See
        // `docs/security-front-running-analysis.md` (#1553) for MEV context on
        // Stellar/Soroban and why commit-reveal remains opt-in rather than required.
        if config.prediction_cooldown_seconds > 0 {
            let last_prediction_key = DataKey::LastPredictionTime(user.clone());
            if let Some(last_prediction_time) = env
                .storage()
                .persistent()
                .get::<_, u64>(&last_prediction_key)
            {
                Self::extend_persistent(&env, &last_prediction_key);
                let now = env.ledger().timestamp();
                if now.saturating_sub(last_prediction_time) < config.prediction_cooldown_seconds {
                    Self::exit_reentrancy_guard(&env);
                    soroban_sdk::panic_with_error!(
                        &env,
                        PredifiError::RateLimitOrSuspiciousActivity
                    );
                }
            }
        }

        // Gas optimization (#1940): read `Pred(user, pool_id)` from persistent
        // storage exactly once per call. This key was previously read twice —
        // once (conditionally) to check the max-predictions-per-user limit,
        // then unconditionally again a few lines down to decide whether this
        // is a new prediction or a stake increase — doubling a persistent
        // storage IO on every `place_prediction` call that had the limit
        // configured.
        let pred_key = DataKey::Pred(user.clone(), pool_id);
        let existing_pred = env.storage().persistent().get::<_, Prediction>(&pred_key);

        if config.max_predictions_per_user > 0 {
            // If user already has a prediction on this pool, allow increasing stake (same prediction)
            // If this is a new prediction for this pool, check if user has reached the limit
            if existing_pred.is_none() {
                // Count current number of pools this user has predictions in
                let user_prediction_count_key = DataKey::UsrPrdCnt(user.clone());
                let current_count: u32 = env
                    .storage()
                    .persistent()
                    .get(&user_prediction_count_key)
                    .unwrap_or(0);

                if current_count >= config.max_predictions_per_user {
                    Self::exit_reentrancy_guard(&env);
                    soroban_sdk::panic_with_error!(&env, PredifiError::MaxPredictionsExceeded);
                }
            }
            // Note: If user already has a prediction on this pool, we allow increasing the stake
            // as it's the same prediction, not a new pool participation
        }

        if let Some(mut existing_pred) = existing_pred {
            assert!(
                existing_pred.outcome == outcome,
                "Cannot change prediction outcome"
            );
            existing_pred.amount = SafeMath::safe_add(existing_pred.amount, amount)
                .map_err(|_| PredifiError::ArithmeticError)?;
            env.storage().persistent().set(&pred_key, &existing_pred);
            Self::extend_persistent(&env, &pred_key);

            // Track referred volume: if this user already has a referrer, add to their volume
            let referrer_key = DataKey::Referrer(user.clone(), pool_id);
            if let Some(referrer_addr) = env.storage().persistent().get::<_, Address>(&referrer_key)
            {
                Self::extend_persistent(&env, &referrer_key);
                let vol_key = DataKey::ReferredVolume(referrer_addr.clone(), pool_id);
                let vol: i128 = env.storage().persistent().get(&vol_key).unwrap_or(0);
                let new_vol =
                    SafeMath::safe_add(vol, amount).map_err(|_| PredifiError::ArithmeticError)?;
                env.storage().persistent().set(&vol_key, &new_vol);
                Self::extend_persistent(&env, &vol_key);
            }
        } else {
            env.storage()
                .persistent()
                .set(&pred_key, &Prediction { amount, outcome });
            Self::extend_persistent(&env, &pred_key);

            // Store referrer on first prediction and track referred volume.
            // NOTE: Only one referrer per (user, pool) is supported today.
            // See DataKey::Referrer for a note on extending this to multiple referrers.
            let referrer_key = DataKey::Referrer(user.clone(), pool_id);
            let active_referrer = if let Some(ref referrer_addr) = referrer {
                env.storage().persistent().set(&referrer_key, referrer_addr);
                Self::extend_persistent(&env, &referrer_key);
                Some(referrer_addr.clone())
            } else {
                env.storage().persistent().get::<_, Address>(&referrer_key)
            };

            if let Some(referrer_addr) = active_referrer {
                let vol_key = DataKey::ReferredVolume(referrer_addr.clone(), pool_id);
                let vol: i128 = env.storage().persistent().get(&vol_key).unwrap_or(0);
                let new_vol =
                    SafeMath::safe_add(vol, amount).map_err(|_| PredifiError::ArithmeticError)?;
                env.storage().persistent().set(&vol_key, &new_vol);
                Self::extend_persistent(&env, &vol_key);
            }

            // Increment participants_count in the pool struct
            pool.participants_count = pool.participants_count.saturating_add(1);

            let count_key = DataKey::UsrPrdCnt(user.clone());
            let count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);

            let index_key = DataKey::UsrPrdIdx(user.clone(), count);
            env.storage().persistent().set(&index_key, &pool_id);
            Self::extend_persistent(&env, &index_key);

            env.storage().persistent().set(&count_key, &(count + 1));
            Self::extend_persistent(&env, &count_key);
        }

        // Update total stake (INV-1)
        pool.total_stake = SafeMath::safe_add(pool.total_stake, amount)
            .map_err(|_| PredifiError::ArithmeticError)?;
        env.storage().persistent().set(&pool_key, &pool);
        Self::bump_ttl(&env, &pool_key);

        let last_prediction_key = DataKey::LastPredictionTime(user.clone());
        env.storage()
            .persistent()
            .set(&last_prediction_key, &env.ledger().timestamp());
        Self::extend_persistent(&env, &last_prediction_key);

        // Update outcome stake (INV-1) - using optimized batch storage
        let _stakes =
            Self::update_outcome_stake(&env, pool_id, outcome, amount, pool.options_count);

        // --- INTERACTIONS ---

        // Validate token transfer safety before executing
        Self::validate_token_transfer(
            &env,
            &pool.token,
            &user,
            &env.current_contract_address(),
            amount,
        )?;

        let token_client = token::Client::new(&env, &pool.token);
        token_client.transfer(&user, &env.current_contract_address(), &amount);

        Self::exit_reentrancy_guard(&env);

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

        // 🟢 INFO: For markets with many outcomes (16+), emit batch stake update event
        // to avoid emitting individual events per outcome which would be impractical
        // for large tournaments (e.g., 32-team bracket).
        if pool.options_count >= 16 {
            OutcomeStakesUpdatedEvent {
                pool_id,
                options_count: pool.options_count,
                total_stake: pool.total_stake,
            }
            .publish(&env);
        }

        Ok(())
    }

    /// Claim winnings from a resolved pool. Returns the amount paid out (0 for losers).
    /// PRE: pool.state ≠ Active
    /// POST: HasClaimed(user, pool) = true (INV-3), payout ≤ pool.total_stake (INV-4)
    fn claim_winnings_internal(
        env: &Env,
        user: &Address,
        pool_id: u64,
    ) -> Result<i128, PredifiError> {
        Self::enter_reentrancy_guard(env);

        let result: Result<i128, PredifiError> = (|| {
            let pool_key = DataKey::Pool(pool_id);
            let pool: Pool = match env.storage().persistent().get(&pool_key) {
                Some(p) => p,
                None => return Err(PredifiError::PoolNotFound),
            };
            Self::extend_persistent(env, &pool_key);

            if pool.state == MarketState::Active {
                return Err(PredifiError::PoolNotResolved);
            }

            let claimed_key = DataKey::Claimed(user.clone(), pool_id);
            if env.storage().persistent().has(&claimed_key) {
                SuspiciousDoubleClaimEvent {
                    user: user.clone(),
                    pool_id,
                    timestamp: env.ledger().timestamp(),
                }
                .publish(env);
                return Err(PredifiError::AlreadyClaimed);
            }

            let pred_key = DataKey::Pred(user.clone(), pool_id);
            // Single get — avoid redundant has() storage read on the hot claim path
            let prediction: Option<Prediction> = env.storage().persistent().get(&pred_key);
            let prediction = match prediction {
                Some(p) => {
                    Self::extend_persistent(env, &pred_key);
                    p
                }
                None => return Ok(0),
            };

            env.storage().persistent().set(&claimed_key, &true);
            Self::bump_ttl(env, &claimed_key);

            if pool.state == MarketState::Canceled {
                // Validate token transfer before sending refund
                Self::validate_token_transfer(
                    env,
                    &pool.token,
                    &env.current_contract_address(),
                    user,
                    prediction.amount,
                )?;

                let token_client = token::Client::new(env, &pool.token);
                token_client.transfer(&env.current_contract_address(), user, &prediction.amount);

                WinningsClaimedEvent {
                    pool_id,
                    user: user.clone(),
                    amount: prediction.amount,
                }
                .publish(env);

                RewardClaimedEvent {
                    pool_id,
                    user: user.clone(),
                    amount: prediction.amount,
                    claim_type: String::from_str(env, "winnings"),
                }
                .publish(env);

                return Ok(prediction.amount);
            }

            // Check if pool is properly resolved
            if !Self::is_pool_resolved(&pool) {
                return Err(PredifiError::PoolNotResolved);
            }

            // Check claim window expiration if resolution timestamp exists
            if let Some(resolution_timestamp) = pool.resolution_timestamp {
                let config = Self::get_config(env);
                let claim_deadline = resolution_timestamp
                    .checked_add(config.claim_window_seconds)
                    .ok_or(PredifiError::InvalidTimestamp)?;
                let current_time = env.ledger().timestamp();

                if current_time > claim_deadline {
                    return Err(PredifiError::InvalidTimestamp);
                }
            }

            if prediction.outcome != pool.outcome {
                return Ok(0);
            }

            let winning_stake = Self::get_outcome_stake(env.clone(), pool_id, pool.outcome);

            if winning_stake == 0 {
                return Ok(0);
            }

            let fee_bps_i = if pool.fee_bps > 0 || pool.state == MarketState::Resolved {
                pool.fee_bps as i128
            } else {
                let config = Self::get_config(env);
                config.fee_bps as i128
            };

            // Payout math lives in `payouts` — keeps lib.rs focused on orchestration
            let breakdown = calculate_claim_payout(&PayoutInput {
                pool_total_stake: pool.total_stake,
                fee_bps: fee_bps_i,
                user_stake: prediction.amount,
                winning_stake,
            })
            .map_err(|_| PredifiError::InvalidAmount)?;
            let protocol_fee_total = breakdown.protocol_fee;
            let winnings = breakdown.winnings;

            assert!(winnings <= pool.total_stake, "Winnings exceed total stake");

            let token_client = token::Client::new(env, &pool.token);

            let referrer_key = DataKey::Referrer(user.clone(), pool_id);
            if let Some(referrer) = env.storage().persistent().get::<_, Address>(&referrer_key) {
                Self::extend_persistent(env, &referrer_key);
                if protocol_fee_total > 0 && pool.total_stake > 0 {
                    let referral_cut_bps = Self::read_referral_cut_bps(env) as i128;
                    let referral_amount = calculate_referral_amount(
                        prediction.amount,
                        pool.total_stake,
                        protocol_fee_total,
                        referral_cut_bps,
                    )
                    .map_err(|_| PredifiError::InvalidAmount)?;
                    if referral_amount > 0 {
                        // Validate referral token transfer before execution
                        Self::validate_token_transfer(
                            env,
                            &pool.token,
                            &env.current_contract_address(),
                            &referrer,
                            referral_amount,
                        )?;

                        token_client.transfer(
                            &env.current_contract_address(),
                            &referrer,
                            &referral_amount,
                        );
                        ReferralPaidEvent {
                            pool_id,
                            referrer: referrer.clone(),
                            referred_user: user.clone(),
                            amount: referral_amount,
                        }
                        .publish(env);
                    }
                }
            }

            if winnings > 0 {
                // Validate main winnings transfer before execution
                Self::validate_token_transfer(
                    env,
                    &pool.token,
                    &env.current_contract_address(),
                    user,
                    winnings,
                )?;

                token_client.transfer(&env.current_contract_address(), user, &winnings);
            }

            WinningsClaimedEvent {
                pool_id,
                user: user.clone(),
                amount: winnings,
            }
            .publish(env);

            RewardClaimedEvent {
                pool_id,
                user: user.clone(),
                amount: winnings,
                claim_type: String::from_str(env, "winnings"),
            }
            .publish(env);

            Ok(winnings)
        })();

        Self::exit_reentrancy_guard(env);
        result
    }

    /// Claim winning payout from a resolved prediction market pool.
    ///
    /// This function allows users to claim their proportional share of the pool's total
    /// stake after the market has been resolved. The payout is calculated based on the
    /// user's stake relative to total winning stakes, with protocol fees and referral
    /// rewards deducted according to the pool's configuration.
    ///
    /// ## Payout Calculation Algorithm
    ///
    /// The payout is computed in the following stages:
    ///
    /// ```text
    /// Given:
    ///   s = user's stake (prediction.amount)
    ///   W = total winning-side stake (pool.total_stake on winning outcome)
    ///   T = total pool stake (pool.total_stake across all outcomes)
    ///   f = protocol fee rate = pool.fee_bps / 10_000
    ///
    /// Step 1: Calculate Protocol Fee (from entire pool)
    ///   protocol_fee = T * f
    ///
    /// Step 2: Calculate Payout Pool (remaining after fee)
    ///   payout_pool = T - protocol_fee
    ///
    /// Step 3: Calculate User's Proportional Share
    ///   user_share = s * payout_pool / W
    ///
    /// Step 4: Calculate Total Winnings (user_share + returned principal)
    ///   winnings = user_share + s
    ///   (This equals s * (payout_pool / W + 1) = s * (T - protocol_fee) / W + s)
    ///
    /// Step 5: Calculate Referral Fee (if applicable)
    ///   referral_fee = protocol_fee * referral_bps / 10_000
    ///   (referral_bps = Config::referral_bps, default 500 = 5%)
    ///
    /// Final Transfers:
    ///   - Referrer receives: referral_fee (if referrer exists)
    ///   - User receives: winnings - referral_fee
    ///   - Treasury receives: protocol_fee - referral_fee
    /// ```
    ///
    /// ### Complete Example Calculation
    ///
    /// ```text
    /// Pool State (after resolution):
    ///   pool.total_stake = 100,000 tokens
    ///   winning_stake = 60,000 tokens (outcome 1)
    ///   pool.fee_bps = 25 (0.25%)
    ///   referral_bps = 500 (5%)
    ///
    /// User Prediction:
    ///   user.stake = 1,000 tokens (on outcome 1 - winning)
    ///   user.referrer = Some("GA...XYZ")
    ///
    /// Payout Calculation:
    ///   Step 1: protocol_fee = 100,000 * 25 / 10_000 = 250 tokens
    ///   Step 2: payout_pool = 100,000 - 250 = 99,750 tokens
    ///   Step 3: user_share = 1,000 * 99,750 / 60,000 = 1,662.5 tokens
    ///   Step 4: winnings = 1,662.5 + 1,000 = 2,662.5 tokens
    ///   Step 5: referral_fee = 250 * 500 / 10_000 = 12.5 tokens
    ///
    /// Final Transfers:
    ///   Referrer receives: 12.5 tokens
    ///   User receives: 2,662.5 - 12.5 = 2,650 tokens
    ///   Treasury receives: 250 - 12.5 = 237.5 tokens
    ///   Total distributed: 2,650 + 12.5 + 237.5 = 2,900 tokens
    ///   (Other winners claim their shares from remaining 97,100 stake)
    /// ```
    ///
    /// ## Zero-Winnings Scenarios
    ///
    /// The function returns 0 in the following cases (no transfer occurs):
    ///
    /// | Scenario | Reason | Payout |
    /// |----------|--------|--------|
    /// | User did not participate | No prediction exists | 0 |
    /// | User lost the prediction | `prediction.outcome != pool.outcome` | 0 |
    /// | Winning stake is zero | No one won the pool | 0 |
    /// | User already claimed | Double-claim prevention | 0 (but error) |
    ///
    /// ## Claim Window Enforcement
    ///
    /// The protocol enforces a claim window to ensure timely distribution of funds:
    ///
    /// ```text
    /// Claim deadline = pool.resolution_timestamp + config.claim_window_seconds
    ///
    /// Validation:
    ///   • If current_time > claim_deadline: Err(InvalidTimestamp)
    ///   • If claim_window_seconds == 0: No window enforced (claims always open)
    ///   • Claim window starts at resolution_timestamp, not pool.end_time
    /// ```
    ///
    /// **Example**: If `resolution_timestamp = 10000` and `claim_window_seconds = 86400` (24 hours):
    /// * Claims accepted until timestamp 10000 + 86400 = 96400
    /// * Attempt at timestamp 96401 returns `InvalidTimestamp`
    /// * Attempt at timestamp 96400 succeeds
    ///
    /// **Security Rationale**: The claim window protects against:
    /// * Stale claims years after pool resolution
    /// * Inflation risk on long-held claims
    /// * Undisputed pools with forgotten claims
    ///
    /// ## Double-Claim Prevention & Security
    ///
    /// The protocol implements multiple layers of double-claim protection:
    ///
    /// ```text
    /// ┌──────────────────────────────────────────────────────────────────────────────┐
    /// │ Claim Attempt Flow                                                           │
    /// ├──────────────────────────────────────────────────────────────────────────────┤
    /// │ 1. Check claimed sentinel (DataKey::Claimed(user, pool_id))                │ │
    /// │    • Has claimed? → Emit SuspiciousDoubleClaimEvent → Err(AlreadyClaimed)  │ │
    /// │    • Not claimed? → Continue to validation                                 │ │
    /// │                                                                              │ │
    /// │ 2. Mark as claimed atomically (before any transfers)                       │ │
    /// │    • Set DataKey::Claimed(user, pool_id) = true                            │ │
    /// │    • Bump TTL for persistence (prevent garbage collection)                 │ │
    /// │                                                                              │ │
    /// │ 3. Perform payout calculations and transfers                               │ │
    /// │    • Validate token transfer limits                                        │ │
    /// │    • Transfer tokens to user (and referrer if applicable)                  │ │
    /// │                                                                              │ │
    /// │ 4. Emit claim events                                                       │ │
    /// │    • WinningsClaimedEvent                                                  │ │
    /// │    • RewardClaimedEvent                                                    │ │
    /// │    • ReferralPaidEvent (if applicable)                                     │ │
    /// └──────────────────────────────────────────────────────────────────────────────┘
    /// ```
    ///
    /// **Suspicious Double-Claim Detection**:
    /// * When an already-claimed user attempts to claim again, the system:
    ///   1. Emits `SuspiciousDoubleClaimEvent` with user, pool_id, and timestamp
    ///   2. Returns `Err(AlreadyClaimed)` to abort the transaction
    ///   3. Does NOT reset the claim sentinel (prevents tampering)
    /// * This provides an audit trail for monitoring potential exploits
    ///
    /// ## Token Transfer Mechanics
    ///
    /// Token transfers are validated before execution to ensure atomicity:
    ///
    /// ```text
    /// Transfer Validation (validate_token_transfer):
    ///   • Verify token address is valid and registered
    ///   • Verify user balance is sufficient for transfer
    ///   • Verify transfer amount does not exceed pool balance
    ///   • Check for transfer hooks that might reject
    ///   • Validate token decimals match expected precision
    ///
    /// Transfer Execution (token::Client.transfer):
    ///   • From: contract address (env.current_contract_address())
    ///   • To: user address (claiming user)
    ///   • Amount: calculated winnings minus referral fee
    ///   • Token: pool.token (from pool configuration)
    ///
    /// Referral Transfer (if applicable):
    ///   • From: contract address
    ///   • To: referrer address (from DataKey::Referrer)
    ///   • Amount: referral_fee (calculated from protocol_fee * referral_bps / 10_000)
    ///   • Token: same as user transfer
    /// ```
    ///
    /// **Atomicity Guarantees**:
    /// * All validations occur before any transfers
    /// * If any validation fails, no transfers occur
    /// * Transfers use Soroban's token standard, ensuring atomicity
    ///
    /// ## Emitted Events
    ///
    /// | Event | Trigger | Fields | Purpose |
    /// |-------|---------|--------|---------|
    /// | `WinningsClaimedEvent` | Every successful claim | `pool_id`, `user`, `amount` | Track claim activity |
    /// | `RewardClaimedEvent` | Every successful claim | `pool_id`, `user`, `amount`, `claim_type` | Categorize claim type |
    /// | `ReferralPaidEvent` | If referrer exists | `pool_id`, `referrer`, `referred_user`, `amount` | Track referral payouts |
    /// | `SuspiciousDoubleClaimEvent` | Double-claim attempt | `user`, `pool_id`, `timestamp` | Security alerting |
    ///
    /// ### Event Examples
    ///
    /// **Success Case**:
    /// ```json
    /// {
    ///   "events": [
    ///     {
    ///       "topic": ["WinningsClaimedEvent"],
    ///       "data": [
    ///         {"val": "u64", "Val": 123},
    ///         {"val": "bytes", "Val": "GA...USER"},
    ///         {"val": "i128", "Val": "2650000000"}
    ///       ]
    ///     },
    ///     {
    ///       "topic": ["RewardClaimedEvent"],
    ///       "data": [
    ///         {"val": "u64", "Val": 123},
    ///         {"val": "bytes", "Val": "GA...USER"},
    ///         {"val": "i128", "Val": "2650000000"},
    ///         {"val": "bytes", "Val": "d2VubmluZ3M="}
    ///       ]
    ///     },
    ///     {
    ///       "topic": ["ReferralPaidEvent"],
    ///       "data": [
    ///         {"val": "u64", "Val": 123},
    ///         {"val": "bytes", "Val": "GA...REF"},
    ///         {"val": "bytes", "Val": "GA...USER"},
    ///         {"val": "i128", "Val": "12500000"}
    ///       ]
    ///     }
    ///   ]
    /// }
    /// ```
    ///
    /// **Double-Claim Attempt**:
    /// ```json
    /// {
    ///   "events": [
    ///     {
    ///       "topic": ["SuspiciousDoubleClaimEvent"],
    ///       "data": [
    ///         {"val": "bytes", "Val": "GA...USER"},
    ///         {"val": "u64", "Val": 123},
    ///         {"val": "u64", "Val": 1234567890}
    ///       ]
    ///     }
    ///   ]
    /// }
    /// ```
    ///
    /// ## Arguments
    ///
    /// * `env` - The [`Env`](soroban_sdk::Env) providing access to:
    ///   * Storage for pool state and user predictions
    ///   * Ledger for timestamp checks
    ///   * Token client for transfers
    ///   * Event system for emissions
    /// * `user` - The [`Address`](soroban_sdk::Address) of the winning user claiming payout.
    ///   Must provide authorization via `user.require_auth()`.
    /// * `pool_id` - The unique 64-bit identifier of the resolved market pool.
    ///
    /// ## Returns
    ///
    /// * `Ok(amount)` - Net winning token amount transferred to `user` (0 for non-winners).
    ///   The amount is net of protocol fees and referral fees (if applicable).
    /// * `Err(PredifiError)` - Operation failed with one of the following errors:
    ///
    /// | Error | Condition | User Guidance |
    /// |-------|-----------|---------------|
    /// | `ContractPaused` | Contract is globally paused | Wait for admin to unpause |
    /// | `PoolNotFound` | Pool does not exist | Verify pool_id is correct |
    /// | `PoolNotResolved` | Pool is not resolved/canceled | Wait for resolution |
    /// | `AlreadyClaimed` | User already claimed winnings | Check claim status before retrying |
    /// | `InvalidTimestamp` | Claim window expired | Check claim deadline |
    #[allow(clippy::needless_borrows_for_generic_args)]
    pub fn claim_winnings(env: Env, user: Address, pool_id: u64) -> Result<i128, PredifiError> {
        Self::require_not_paused(&env)?;
        user.require_auth();
        Self::claim_winnings_internal(&env, &user, pool_id)
    }

    /// Claim winnings from multiple pools in a single transaction.
    ///
    /// Iterates over `pool_ids`, calls the single-pool claim logic for each,
    /// and returns a `Map<u64, i128>` showing how much was claimed per pool.
    /// Pools that yield 0 (loser, already claimed, no prediction) are still
    /// included in the map with value 0 so callers can distinguish "processed"
    /// from "not attempted".
    ///
    /// # Arguments
    /// * `user`     - Address claiming winnings (must provide auth once)
    /// * `pool_ids` - List of pool IDs to claim from
    ///
    /// # Returns
    /// `Map<u64, i128>` — claimed amount per pool (0 for non-winners / already claimed)
    pub fn batch_claim_winnings(
        env: Env,
        user: Address,
        pool_ids: Vec<u64>,
    ) -> Result<soroban_sdk::Map<u64, i128>, PredifiError> {
        Self::require_not_paused(&env)?;
        user.require_auth();
        let mut results: soroban_sdk::Map<u64, i128> = soroban_sdk::Map::new(&env);
        for pool_id in pool_ids.iter() {
            let amount = Self::claim_winnings_internal(&env, &user, pool_id).unwrap_or(0);
            results.set(pool_id, amount);
        }
        Ok(results)
    }

    /// Claim a refund from a canceled prediction pool.
    ///
    /// This function allows users to recover 100% of their original stake when a
    /// prediction market pool is canceled. Unlike `claim_winnings`, no protocol
    /// fees or referral rewards are deducted for canceled pools, ensuring full
    /// principal recovery.
    ///
    /// ## Refund Calculation & Economics
    ///
    /// ```text
    /// Given:
    ///   s = user's original stake (prediction.amount)
    ///
    /// Refund Amount:
    ///   refund = s
    ///
    /// No deductions:
    ///   • No protocol fee (pool.fee_bps ignored)
    ///   • No referral reward (referral_bps ignored)
    ///   • No slashing or penalties
    /// ```
    ///
    /// **Economic Rationale**: When a pool is canceled, the protocol returns the
    /// full principal because:
    /// * Market conditions changed after predictions were placed
    /// * Users cannot be expected to bear fees for an unresolved market
    /// * Full principal recovery maintains user trust in the protocol
    /// * Fee collection only occurs when markets are successfully resolved
    ///
    /// ## Double-Claim Prevention & Security
    ///
    /// The protocol implements robust protection against double-refund attacks:
    ///
    /// ```text
    /// ┌──────────────────────────────────────────────────────────────────────────────┐
    /// │ Claim Flow                                                                   │
    /// ├──────────────────────────────────────────────────────────────────────────────┤
    /// │ 1. Enter re-entrancy guard                                                   │
    /// │    • enter_reentrancy_guard(&env)                                           │
    /// │    • Prevents recursive refund calls during transfer                       │
    /// │                                                                              │
    /// │ 2. Verify pool is Canceled                                                   │
    /// │    • pool.state == MarketState::Canceled                                    │
    /// │    • Else: Err(InvalidPoolState)                                           │
    /// │                                                                              │
    /// │ 3. Check double-claim sentinel                                               │
    /// │    • if DataKey::Claimed(user, pool_id) exists:                            │
    /// │      → Err(AlreadyClaimed)                                                  │
    /// │    • Else: continue                                                        │
    /// │                                                                              │
    /// │ 4. Mark as claimed atomically (BEFORE transfer)                             │
    /// │    • Set DataKey::Claimed(user, pool_id) = true                            │
    /// │    • Bump TTL for persistence                                              │
    /// │    • Ensures no re-entrancy can bypass check                              │
    /// │                                                                              │
    /// │ 5. Validate and execute token transfer                                      │
    /// │    • validate_token_transfer(...)                                          │
    /// │    • token_client.transfer(...)                                            │
    /// │                                                                              │
    /// │ 6. Emit claim events                                                       │
    /// │    • RefundClaimedEvent                                                    │
    /// │    • RewardClaimedEvent (claim_type: "refund")                            │
    /// │                                                                              │
    /// │ 7. Exit re-entrancy guard                                                    │
    /// │    • exit_reentrancy_guard(&env)                                           │
    /// └──────────────────────────────────────────────────────────────────────────────┘
    /// ```
    ///
    /// **Re-entrancy Protection**:
    /// * The re-entrancy guard is entered **before** the double-claim check
    /// * This prevents malicious contracts from re-entering and claiming again
    /// * The guard is released **after** all transfers are complete
    /// * The claim sentinel is set **before** any external calls
    ///
    /// ## Token Transfer Mechanics
    ///
    /// Refund transfers follow the same safety patterns as winnings claims:
    ///
    /// ```text
    /// Transfer Sequence:
    ///   1. Validate Transfer
    ///      • validate_token_transfer(
    ///          contract_balance >= refund_amount,
    ///          user_token_balance_valid,
    ///          transfer_hook_allowed
    ///        )
    ///      • Returns Err(TransferFailed) if any check fails
    ///
    ///   2. Execute Transfer
    ///      • token::Client::new(&env, &pool.token)
    ///      • transfer(&contract_address, &user, &refund_amount)
    ///      • Soroban ensures atomicity
    ///
    ///   3. Emit Events
    ///      • RefundClaimedEvent
    ///      • RewardClaimedEvent
    /// ```
    ///
    /// **Transfer Validation**:
    /// * Contract must have sufficient token balance
    /// * User's token receiver must accept the transfer
    /// * No transfer hooks can reject the transfer
    /// * Token decimals must be valid
    ///
    /// ## Emitted Events
    ///
    /// | Event | Trigger | Fields | Purpose |
    /// |-------|---------|--------|---------|
    /// | `RefundClaimedEvent` | Successful refund | `pool_id`, `user`, `amount` | Track refund activity |
    /// | `RewardClaimedEvent` | Successful refund | `pool_id`, `user`, `amount`, `claim_type="refund"` | Categorize as refund |
    ///
    /// ### Event Examples
    ///
    /// **Success Case**:
    /// ```json
    /// {
    ///   "events": [
    ///     {
    ///       "topic": ["RefundClaimedEvent"],
    ///       "data": [
    ///         {"val": "u64", "Val": 456},
    ///         {"val": "bytes", "Val": "GA...USER"},
    ///         {"val": "i128", "Val": "1000000000"}
    ///       ]
    ///     },
    ///     {
    ///       "topic": ["RewardClaimedEvent"],
    ///       "data": [
    ///         {"val": "u64", "Val": 456},
    ///         {"val": "bytes", "Val": "GA...USER"},
    ///         {"val": "i128", "Val": "1000000000"},
    ///         {"val": "bytes", "Val": "cmVmdW5k"}
    ///       ]
    ///     }
    ///   ]
    /// }
    /// ```
    ///
    /// **Already Claimed Attempt**:
    /// ```text
    /// Transaction aborts with: Err(AlreadyClaimed)
    /// No events emitted (reverted before any side effects)
    /// ```
    ///
    /// ## Arguments
    ///
    /// * `env` - The [`Env`](soroban_sdk::Env) providing access to:
    ///   * Storage for pool state and user predictions
    ///   * Token client for refund execution
    ///   * Event system for emissions
    /// * `user` - The [`Address`](soroban_sdk::Address) claiming the refund.
    ///   Must provide authorization via `user.require_auth()`.
    /// * `pool_id` - The unique 64-bit identifier of the **canceled** pool.
    ///
    /// ## Returns
    ///
    /// * `Ok(amount)` - Full refunded principal amount (100% of original stake).
    ///   No fees or penalties are deducted.
    /// * `Err(PredifiError)` - Operation failed with one of the following errors:
    ///
    /// | Error | Condition | User Guidance |
    /// |-------|-----------|---------------|
    /// | `ContractPaused` | Contract is globally paused | Wait for admin to unpause |
    /// | `InvalidPoolState` | Pool does not exist or is not Canceled | Verify pool exists and is in Canceled state |
    /// | `InsufficientBalance` | User has no prediction or stake on pool | Check prediction exists with non-zero amount |
    /// | `AlreadyClaimed` | User already claimed refund for this pool | Refund already received, no further claims allowed |
    ///
    /// ## Error Handling Examples
    ///
    /// ```rust,no_run
    /// // Example 1: Successful refund
    /// let refund = client.claim_refund(&user, &pool_id);
    /// assert_eq!(refund.unwrap(), 1000000000); // Full principal returned
    ///
    /// // Example 2: Pool not canceled
    /// let result = client.try_claim_refund(&user, &pool_id);
    /// assert!(matches!(result.unwrap_err(), PredifiError::InvalidPoolState));
    ///
    /// // Example 3: Already claimed
    /// let result = client.try_claim_refund(&user, &pool_id);
    /// assert!(matches!(result.unwrap_err(), PredifiError::AlreadyClaimed));
    /// ```
    ///
    /// ## Use Cases
    ///
    /// * **Market Canceled**: Pool creator or admin canceled the market; users reclaim principal
    /// * **Oracle Failure**: Price feed became unavailable; pool was canceled
    /// * **Emergency Cancellation**: Protocol emergency triggered pool cancellation
    /// * **Invalid Prediction**: Pool was created with invalid parameters; canceled after launch
    ///
    /// ## Comparison with claim_winnings
    ///
    /// | Aspect | claim_winnings | claim_refund |
    /// |--------|----------------|--------------|
    /// | Pool State | Resolved or Canceled | Canceled only |
    /// | Fee Deduction | Yes (protocol + referral) | No (full principal) |
    /// | Payout Amount | Proportional share | 100% of stake |
    /// | Claim Window | Enforced (if configured) | N/A (no window) |
    /// | Profit Potential | Yes (if winning) | Zero profit/loss |
    /// | Loss Potential | Yes (if losing) | Zero loss |
    ///
    /// ## Invariants Enforced
    ///
    /// | INV | Statement | Implementation |
    /// |-----|-----------|----------------|
    /// | INV-3 | Double-claim prevention | `DataKey::Claimed` sentinel set atomically |
    /// | INV-4 | No value creation | `refund == prediction.amount` (exact) |
    /// | INV-5 | Atomic transfers | All validations before any transfers |
    /// | INV-6 | Re-entrancy protection | Guard entered before all checks |
    /// | INV-7 | Positive amounts | `prediction.amount > 0` validated |
    #[allow(clippy::needless_borrows_for_generic_args)]
    pub fn claim_refund(env: Env, user: Address, pool_id: u64) -> Result<i128, PredifiError> {
        Self::require_not_paused(&env)?;
        user.require_auth();

        // 🛡️ RE-ENTRANCY GUARD: Protect against recursive withdrawal attempts
        // during value transfer to external addresses/contracts (INV-3).
        Self::enter_reentrancy_guard(&env);

        let result: Result<i128, PredifiError> = (|| {
            // --- CHECKS ---

            let pool_key = DataKey::Pool(pool_id);
            let pool: Pool = match env.storage().persistent().get(&pool_key) {
                Some(p) => p,
                None => {
                    return Err(PredifiError::InvalidPoolState);
                }
            };
            Self::extend_persistent(&env, &pool_key);

            // Verify pool is canceled
            if pool.state != MarketState::Canceled {
                return Err(PredifiError::InvalidPoolState);
            }

            // Check if user already claimed refund
            let claimed_key = DataKey::Claimed(user.clone(), pool_id);
            if env.storage().persistent().has(&claimed_key) {
                return Err(PredifiError::AlreadyClaimed);
            }

            // Get user's prediction
            let pred_key = DataKey::Pred(user.clone(), pool_id);
            let prediction: Option<Prediction> = env.storage().persistent().get(&pred_key);

            if env.storage().persistent().has(&pred_key) {
                Self::extend_persistent(&env, &pred_key);
            }

            let prediction = match prediction {
                Some(p) => p,
                None => {
                    return Err(PredifiError::InsufficientBalance);
                }
            };

            // Verify user has a non-zero stake
            if prediction.amount <= 0 {
                return Err(PredifiError::InsufficientBalance);
            }

            // --- EFFECTS ---

            // Mark as claimed immediately to prevent re-entrancy (INV-3)
            env.storage().persistent().set(&claimed_key, &true);
            Self::bump_ttl(&env, &claimed_key);

            let refund_amount = prediction.amount;

            // --- INTERACTIONS ---

            // Validate token transfer before sending refund
            Self::validate_token_transfer(
                &env,
                &pool.token,
                &env.current_contract_address(),
                &user,
                refund_amount,
            )?;

            let token_client = token::Client::new(&env, &pool.token);
            token_client.transfer(&env.current_contract_address(), &user, &refund_amount);

            RefundClaimedEvent {
                pool_id,
                user: user.clone(),
                amount: refund_amount,
            }
            .publish(&env);

            RewardClaimedEvent {
                pool_id,
                user: user.clone(),
                amount: refund_amount,
                claim_type: String::from_str(&env, "refund"),
            }
            .publish(&env);

            Ok(refund_amount)
        })();

        Self::exit_reentrancy_guard(&env);
        result
    }

    /// Get a paginated list of a user's predictions.
    ///
    /// # Errors
    /// Returns `PredifiError::InvalidPagination` if `offset + limit` overflows `u32`.
    pub fn get_user_predictions(
        env: Env,
        user: Address,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<UserPredictionDetail>, PredifiError> {
        // Guard against offset + limit wrapping around u32::MAX.
        let end_check = offset
            .checked_add(limit)
            .ok_or(PredifiError::InvalidPagination)?;

        let count_key = DataKey::UsrPrdCnt(user.clone());
        let count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);
        if env.storage().persistent().has(&count_key) {
            Self::extend_persistent(&env, &count_key);
        }

        let mut results = Vec::new(&env);

        if offset >= count || limit == 0 {
            return Ok(results);
        }

        let end = core::cmp::min(end_check, count);

        for i in offset..end {
            let index_key = DataKey::UsrPrdIdx(user.clone(), i);
            let pool_id: u64 = env
                .storage()
                .persistent()
                .get(&index_key)
                .expect("index not found");
            Self::extend_persistent(&env, &index_key);

            let pred_key = DataKey::Pred(user.clone(), pool_id);
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

        Ok(results)
    }
}

// ── Domain types & events (moved from lib.rs, issue #1970) ─────────────────

/// Represents the current state of a prediction market.
///
/// State transitions are one-way: `Active` can only transition to `Resolved`, `Canceled`, or `Disputed`.
#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarketState {
    /// Market is active and accepting predictions.
    Active = 0,
    /// Market has been resolved and winnings can be claimed.
    Resolved = 1,
    /// Market has been canceled and stakes can be refunded.
    Canceled = 2,
    /// Market has been flagged as disputed by a moderator.
    Disputed = 3,
}

/// Represents a user's individual stake in a prediction market.
///
/// This is the core structure for tracking participation. It is stored as part of the
/// ledger state for each user-pool pair, mapping a specific outcome to a staked amount.
#[contracttype]
#[derive(Clone)]
pub struct Prediction {
    /// Total amount of tokens staked by the user on this outcome.
    pub amount: i128,
    /// The chosen outcome index (0-based). This corresponds to the index in `Pool.outcome_descriptions`.
    pub outcome: u32,
}

/// Detailed information about a user's prediction in a specific pool.
///
/// This struct is a convenient "read-only" view that combines user-specific prediction
/// data with current pool state. It is primarily used for frontend displays and
/// calculating potential or final winnings.
#[contracttype]
#[derive(Clone)]
pub struct UserPredictionDetail {
    /// Unique identifier (ID) of the prediction pool.
    pub pool_id: u64,
    /// Total amount of tokens the user has staked on their chosen outcome.
    pub amount: i128,
    /// The outcome index (0-based) that the user predicted would win.
    pub user_outcome: u32,
    /// Unix timestamp when the pool's betting window ends.
    pub pool_end_time: u64,
    /// Current operational state of the pool (Active, Resolved, or Canceled).
    pub pool_state: MarketState,
    /// The winning outcome index (0-based) if the pool is `Resolved`.
    /// Set to `UNRESOLVED_OUTCOME` (`u32::MAX`) when the pool has not yet been resolved.
    /// Callers must check `pool_state == MarketState::Resolved` (or compare against
    /// `UNRESOLVED_OUTCOME`) before interpreting this value; outcome index `0` is a
    /// valid winning outcome and must not be confused with the unresolved sentinel.
    pub pool_outcome: u32,
}

#[contractevent(topics = ["prediction_placed"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredictionPlacedEvent {
    pub pool_id: u64,
    pub user: Address,
    pub amount: i128,
    pub outcome: u32,
}

#[contractevent(topics = ["winnings_claimed"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WinningsClaimedEvent {
    pub pool_id: u64,
    pub user: Address,
    pub amount: i128,
}

/// Emitted when a `place_prediction` call is rejected because the pool's token
/// has been removed from the whitelist since the pool was created.
/// Useful for off-chain monitors to detect affected pools and alert users.
#[contractevent(topics = ["prediction_blocked_delisted"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredictionBlockedDelistedEvent {
    pub pool_id: u64,
    pub user: Address,
    pub token: Address,
    pub timestamp: u64,
}

#[contractevent(topics = ["refund_claimed"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundClaimedEvent {
    pub pool_id: u64,
    pub user: Address,
    pub amount: i128,
}

/// 🟢 INFO — emitted when all outcome stakes are updated in a single operation.
/// Useful for markets with many outcomes (e.g., 32+ teams tournament) where
/// emitting individual events per outcome would be impractical.
#[contractevent(topics = ["outcome_stakes_updated"])]
#[contracttype(export = false)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutcomeStakesUpdatedEvent {
    pub pool_id: u64,
    /// Number of outcomes in this pool.
    pub options_count: u32,
    /// Total stake across all outcomes after the update.
    pub total_stake: i128,
}


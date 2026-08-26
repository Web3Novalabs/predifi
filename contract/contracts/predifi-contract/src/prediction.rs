//! Prediction domain: placing predictions and claiming winnings or refunds.

use soroban_sdk::{contractimpl, token, Address, Env, String, Symbol, Vec};

use crate::{
    calculate_claim_payout, calculate_referral_amount, DataKey, HighValuePredictionEvent,
    MarketState, OutcomeStakesUpdatedEvent, PayoutInput, Pool, Prediction,
    PredictionBlockedDelistedEvent, PredictionPlacedEvent, PredifiContract, PredifiContractArgs,
    PredifiContractClient, PredifiError, ReferralPaidEvent, RefundClaimedEvent, RewardClaimedEvent,
    SuspiciousDoubleClaimEvent, UserPredictionDetail, WinningsClaimedEvent, HIGH_VALUE_THRESHOLD,
};

#[contractimpl]
impl PredifiContract {
    /// Place a prediction on an active pool by staking tokens on a specific outcome.
    ///
    /// This function allows users to participate in prediction markets by transferring tokens
    /// to the contract and recording their prediction. The prediction is stored and will be
    /// evaluated when the pool is resolved. Winners can claim their share of the pool's total
    /// stake minus protocol fees via `claim_winnings`.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment, providing access to storage, ledger, and auth
    /// - `user` - The address placing the prediction (must authenticate via `require_auth`)
    /// - `pool_id` - The unique identifier of the prediction pool to bet on
    /// - `amount` - The amount of tokens to stake (must be > 0 and meet minimum requirements)
    /// - `outcome` - The predicted outcome index (0-based, must be < `pool.options_count`)
    /// - `referrer` - Optional address that referred this user. If set, the referrer receives
    ///   a share of the protocol fee when the user claims winnings. Only stored on the first
    ///   prediction for a given `(user, pool_id)` pair. Cannot be the user or the contract.
    /// - `invite_key` - Optional symbol used to access private pools. Must match the pool's
    ///   `whitelist_key` if the pool is private and the user is not whitelisted.
    ///
    /// # Prediction Cooldown Mechanism
    ///
    /// The contract enforces a cooldown period between consecutive predictions from the same
    /// address to prevent spam and potential front-running attacks. The cooldown duration is
    /// configured via `Config::prediction_cooldown_seconds` (default: 0, meaning disabled).
    ///
    /// - When `prediction_cooldown_seconds > 0`, the contract checks `LastPredictionTime(user)`
    ///   to ensure sufficient time has elapsed since the user's last successful prediction.
    /// - If the cooldown has not elapsed, the function returns `RateLimitOrSuspiciousActivity`.
    /// - The timestamp is updated after each successful prediction, regardless of pool.
    ///
    /// # Stake Limits Enforcement
    ///
    /// The function enforces multiple stake limits at different levels:
    ///
    /// **Global Protocol Minimum:**
    /// - `amount` must be >= `Config::min_stake` (default: 1 token unit)
    /// - Error: `InsufficientStake`
    ///
    /// **Per-Pool Limits:**
    /// - `amount` must be >= `pool.min_stake` (set at pool creation)
    /// - Error: `StakeBelowMinimum`
    /// - If `pool.max_stake > 0`, `amount` must be <= `pool.max_stake`
    /// - Error: `StakeAboveMaximum`
    ///
    /// **Total Pool Cap:**
    /// - If `pool.max_total_stake > 0`, the new `pool.total_stake + amount` must not exceed this cap
    /// - Error: `MaxTotalStakeExceeded`
    ///
    /// **User Prediction Count Limit:**
    /// - If `Config::max_predictions_per_user > 0`, a user cannot place predictions on more than
    ///   this number of distinct pools
    /// - Increasing stake on an existing prediction (same pool, same outcome) does not count
    ///   toward this limit
    /// - Error: `MaxPredictionsExceeded`
    ///
    /// # Referral Handling
    ///
    /// Referrals allow users to earn rewards by bringing new participants to the protocol:
    ///
    /// - The `referrer` parameter is only processed on the **first prediction** for a given
    ///   `(user, pool_id)` pair. Subsequent predictions on the same pool ignore this parameter.
    /// - The referrer address is stored in `DataKey::Referrer(user, pool_id)`.
    /// - Referred volume is tracked in `DataKey::ReferredVolume(referrer, pool_id)` and
    ///   accumulates across all predictions from the referred user on that pool.
    /// - When the referred user claims winnings, the referrer receives a share of the protocol
    ///   fee based on `Config::referral_bps` (default: 500 bps = 5%).
    /// - Referrer validation: Cannot be the user themselves or the contract address.
    /// - Currently, only one referrer per (user, pool) is supported. Future extensions may
    ///   support multiple referrers (see `DataKey::Referrer` documentation).
    ///
    /// # Fee Deduction Flow
    ///
    /// **Note:** Protocol fees are NOT deducted during `place_prediction`. Fees are calculated
    /// and deducted at **resolution time** when winners claim their winnings via `claim_winnings`.
    ///
    /// The fee flow works as follows:
    ///
    /// 1. **Prediction Placement (this function):**
    ///    - User transfers the full `amount` to the contract
    ///    - The full amount is added to `pool.total_stake`
    ///    - No fees are deducted at this stage
    ///
    /// 2. **Pool Resolution:**
    ///    - The pool's `fee_bps` is determined by the dynamic fee tier system based on
    ///      `pool.total_stake` at resolution time
    ///    - The winning outcome is finalized
    ///
    /// 3. **Winnings Claim (`claim_winnings`):**
    ///    - Protocol fee = `user_winnings * pool.fee_bps / 10_000`
    ///    - Referral fee = `protocol_fee * Config::referral_bps / 10_000`
    ///    - Treasury receives: `protocol_fee - referral_fee`
    ///    - Referrer receives: `referral_fee`
    ///    - User receives: `user_winnings - protocol_fee`
    ///
    /// # Emitted Events
    ///
    /// This function emits the following events in order:
    ///
    /// 1. **`PredictionPlacedEvent`** (always emitted):
    ///    - `pool_id` - The pool receiving the prediction
    ///    - `user` - The address placing the prediction
    ///    - `amount` - The staked amount
    ///    - `outcome` - The predicted outcome index
    ///
    /// 2. **`HighValuePredictionEvent`** (conditional):
    ///    - Emitted when `amount >= HIGH_VALUE_THRESHOLD`
    ///    - Used for monitoring and alerting on large stakes
    ///    - Fields: `pool_id`, `user`, `amount`, `outcome`, `threshold`
    ///
    /// 3. **`OutcomeStakesUpdatedEvent`** (conditional):
    ///    - Emitted when `pool.options_count >= 16`
    ///    - Avoids emitting individual events per outcome for large tournaments
    ///    - Fields: `pool_id`, `options_count`, `total_stake`
    ///
    /// 4. **`PredictionBlockedDelistedEvent`** (error case):
    ///    - Emitted when the pool's token is not whitelisted
    ///    - Fields: `pool_id`, `user`, `token`, `timestamp`
    ///
    /// # Error Conditions
    ///
    /// The function can return the following errors:
    ///
    /// - `ContractPaused` - The contract is currently paused; all state-mutating operations are blocked
    /// - `InvalidAmount` - The stake amount is zero or negative
    /// - `InsufficientStake` - The amount is below the global protocol minimum (`Config::min_stake`)
    /// - `TokenNotWhitelisted` - The pool's token is not on the allowed betting whitelist
    /// - `PoolNotFound` - The specified `pool_id` does not exist
    /// - `InvalidPoolState` - The pool is not in `Active` state (e.g., resolved, canceled, or disputed)
    /// - `InvalidOutcome` - The outcome index is >= `pool.options_count`
    /// - `StakeBelowMinimum` - The amount is below the pool's `min_stake`
    /// - `StakeAboveMaximum` - The amount exceeds the pool's `max_stake` (if > 0)
    /// - `MaxTotalStakeExceeded` - Adding this amount would exceed `pool.max_total_stake`
    /// - `RateLimitOrSuspiciousActivity` - The prediction cooldown period has not elapsed
    /// - `MaxPredictionsExceeded` - The user has exceeded the maximum number of pools they can participate in
    /// - `ArithmeticError` - An overflow occurred during stake calculations
    /// - `InvalidReferralCode` - The provided `invite_key` failed validation
    /// - `InsufficientBalance` - The user balance is insufficient to fulfill the token transfer
    /// - `TransferFailed` - Token transfer validation or execution failed
    ///
    /// # Pre-conditions
    ///
    /// - `amount > 0` (INV-7)
    /// - `pool.state == MarketState::Active`
    /// - `env.ledger().timestamp() < pool.end_time` (pool has not ended)
    /// - `pool.min_stake <= amount <= pool.max_stake` (unless `max_stake == 0`)
    /// - `amount >= Config::min_stake` (global minimum)
    /// - `outcome < pool.options_count`
    /// - Pool's token must be whitelisted
    /// - For private pools: user must be whitelisted, be the creator, or provide valid `invite_key`
    /// - If `prediction_cooldown_seconds > 0`: sufficient time must have elapsed since user's last prediction
    /// - If `max_predictions_per_user > 0`: user must not have exceeded the prediction count limit
    ///
    /// # Post-conditions
    ///
    /// - `pool.total_stake` increases by `amount` (INV-1)
    /// - `OutcomeStake(pool_id, outcome)` increases by `amount` (INV-1)
    /// - User's prediction record is created or updated with the new stake
    /// - If first prediction for user on this pool: `participants_count` increments
    /// - If referrer provided on first prediction: referrer is stored and referred volume is tracked
    /// - `LastPredictionTime(user)` is updated to current timestamp
    /// - Tokens are transferred from `user` to the contract
    /// - `PredictionPlacedEvent` is emitted
    ///
    /// # Reentrancy Protection
    ///
    /// This function uses a reentrancy guard (`enter_reentrancy_guard` / `exit_reentrancy_guard`)
    /// to prevent reentrant calls. The guard is entered at the start and exited before token
    /// transfer and event emission.
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

        // Enforce maximum predictions per user limit (across all pools)
        let config = Self::get_config(&env);
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

        if config.max_predictions_per_user > 0 {
            let pred_key = DataKey::Pred(user.clone(), pool_id);
            let existing_pred = env.storage().persistent().get::<_, Prediction>(&pred_key);

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

        let pred_key = DataKey::Pred(user.clone(), pool_id);
        let existing_pred = env.storage().persistent().get::<_, Prediction>(&pred_key);
        if let Some(mut existing_pred) = existing_pred {
            assert!(
                existing_pred.outcome == outcome,
                "Cannot change prediction outcome"
            );
            existing_pred.amount = existing_pred.amount.checked_add(amount).expect("overflow");
            env.storage().persistent().set(&pred_key, &existing_pred);
            Self::extend_persistent(&env, &pred_key);

            // Track referred volume: if this user already has a referrer, add to their volume
            let referrer_key = DataKey::Referrer(user.clone(), pool_id);
            if let Some(referrer_addr) = env.storage().persistent().get::<_, Address>(&referrer_key)
            {
                Self::extend_persistent(&env, &referrer_key);
                let vol_key = DataKey::ReferredVolume(referrer_addr.clone(), pool_id);
                let vol: i128 = env.storage().persistent().get(&vol_key).unwrap_or(0);
                env.storage().persistent().set(&vol_key, &(vol + amount));
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
                env.storage().persistent().set(&vol_key, &(vol + amount));
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
        pool.total_stake = pool.total_stake.checked_add(amount).expect("overflow");
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
    /// # Payout Calculation & Economics
    /// - Payout is calculated proportionally based on the caller's stake relative to total winning stakes:
    ///   `user_payout = (user_stake / winning_stake) * (total_pool_stake - protocol_fee)`
    /// - Protocol fee is deducted based on the pool's configured basis points (`fee_bps`).
    /// - If a referrer is associated with the user, a portion of the fee is transferred to the referrer.
    ///
    /// # Claim Window Enforcement
    /// - If `claim_window_seconds` is configured, claims are strictly rejected with `InvalidTimestamp`
    ///   if the current ledger timestamp exceeds `resolution_timestamp + claim_window_seconds`.
    ///
    /// # Double-Claim Prevention & Security
    /// - Prevents double-claiming by storing a `DataKey::Claimed(user, pool_id)` sentinel in persistent storage.
    /// - Re-entrancy guard (`enter_reentrancy_guard` / `exit_reentrancy_guard`) protects token transfers.
    /// - Suspicious double-claim attempts trigger `SuspiciousDoubleClaimEvent` alerts.
    ///
    /// # Token Transfer Mechanics
    /// - Validates token transfer limits via `validate_token_transfer`.
    /// - Executes token transfer directly from the contract address to the user's address via Soroban `token::Client`.
    ///
    /// # Emitted Events
    /// - Emits `WinningsClaimedEvent` and `RewardClaimedEvent` upon successful claim.
    /// - Emits `ReferralPaidEvent` if a referral reward is distributed.
    ///
    /// # Arguments
    /// * `env` - The Soroban host environment.
    /// * `user` - Address of the winning user claiming payout (must require_auth).
    /// * `pool_id` - ID of the resolved market pool.
    ///
    /// # Returns
    /// * `Ok(amount)` - Net winning token amount transferred to `user`.
    /// * `Err(PredifiError)` - Reason for claim failure (e.g. `PoolNotResolved`, `AlreadyClaimed`, `InvalidTimestamp`).
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
    /// # Refund Calculation & Economics
    /// - Returns 100% of the user's original staked principal amount (`prediction.amount`).
    /// - No protocol fees or penalties are deducted when a market pool is canceled.
    ///
    /// # Double-Claim Prevention & Security
    /// - Enforces `INV-3` double-claim prevention by writing `DataKey::Claimed(user, pool_id)` to persistent storage.
    /// - Protected by re-entrancy guard (`enter_reentrancy_guard` / `exit_reentrancy_guard`) during asset transfers.
    ///
    /// # Token Transfer Mechanics
    /// - Checks balance and validates transfer via `validate_token_transfer`.
    /// - Transfers funds from contract balance directly to `user` via Soroban `token::Client`.
    ///
    /// # Emitted Events
    /// - Emits `RefundClaimedEvent` with `pool_id`, `user`, and refunded `amount`.
    /// - Emits `RewardClaimedEvent` with `claim_type: "refund"`.
    ///
    /// PRE: pool.state = Canceled, user has an active prediction on the pool.
    /// POST: HasClaimed(user, pool) = true (INV-3), user receives full principal stake amount.
    ///
    /// # Arguments
    /// * `env` - The Soroban host environment instance.
    /// * `user` - Address claiming the refund (must provide authorization).
    /// * `pool_id` - Unique identifier of the canceled pool.
    ///
    /// # Returns
    /// * `Ok(amount)` - Refund successfully claimed, returns exact refunded principal amount.
    /// * `Err(PredifiError)` - Operation failed with specific error code.
    ///
    /// # Errors
    /// - `InvalidPoolState` if pool doesn't exist or is not in `Canceled` state.
    /// - `InsufficientBalance` if user has no stake to refund.
    /// - `AlreadyClaimed` if user has already claimed a refund for this pool.
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

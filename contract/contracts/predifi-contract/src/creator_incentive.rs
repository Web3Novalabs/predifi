//! # Pool Creator Incentive Domain (Issue #1933)
//!
//! Rewards a pool's creator with a small, admin-configurable share of that
//! pool's protocol fee once the pool clears a minimum participation bar and
//! is resolved. This mirrors the referral-fee-share mechanics in
//! [`crate::referral`], but the recipient is `Pool.creator` instead of a
//! per-user referrer, and eligibility is gated on `Pool.participants_count`
//! rather than a referred-volume threshold.
//!
//! ## Mechanics
//!
//! 1. **Configuration (admin-gated)** — `set_creator_incentive_bps` sets the
//!    share (in basis points) of the pool's protocol fee paid to the
//!    creator; `set_creator_min_participants` sets the minimum unique
//!    participant count (`Pool.participants_count`) a pool must reach.
//!    Both default to "incentive disabled" (`0` bps) / `DEFAULT_CREATOR_MIN_PARTICIPANTS`
//!    respectively until an admin opts in.
//! 2. **Eligibility** — a pool qualifies once it is `MarketState::Resolved`
//!    (canceled pools refund 100% of stake and generate no fee to share) and
//!    `participants_count >= get_creator_min_participants()`.
//! 3. **Claim** — the creator (and only the creator) calls
//!    `claim_creator_incentive(pool_id)`. The payout is computed directly
//!    from the pool's total stake and fee rate —
//!    `protocol_fee = calculate_protocol_fee(pool.total_stake, pool.fee_bps)`,
//!    `creator_amount = percentage(protocol_fee, incentive_bps)` — rather than
//!    from amounts already paid out via `claim_winnings`, so it does not
//!    depend on claim ordering and can be called at any time after
//!    resolution. It is a one-time payout per pool, guarded by
//!    `DataKey::CreatorIncentivePaid(pool_id)`.
//! 4. **Reputation tracking** — each successful claim increments the
//!    creator's `CreatorRewardedPools` counter and `CreatorTotalIncentive`
//!    accumulator, queryable via `get_creator_stats`. These act as simple
//!    on-chain pool-quality / reputation metrics: a creator who repeatedly
//!    clears the participation bar accumulates a visible track record.
//!
//! ## Funding Source
//!
//! Like referral rewards, the creator incentive is paid out of the
//! contract's own token balance (the same accrued protocol fee pool that
//! backs `withdraw_treasury`). It does not come out of user stakes or
//! winnings — it reduces the amount the treasury would otherwise sweep via
//! `withdraw_treasury` for that pool's fee.

use soroban_sdk::{contractimpl, token, Address, Env};

use crate::{
    calculate_protocol_fee, CreatorIncentiveConfigUpdatedEvent, CreatorIncentivePaidEvent,
    CreatorStats, DataKey, MarketState, Pool, PredifiContract, PredifiContractArgs,
    PredifiContractClient, PredifiError, RoundingMode, SafeMath, DEFAULT_CREATOR_MIN_PARTICIPANTS,
};

#[contractimpl]
impl PredifiContract {
    /// Set the creator incentive rate in basis points — the share of a
    /// pool's protocol fee paid to its creator once eligible. Caller must
    /// have Admin role (0). Must be <= 10_000. A value of `0` disables the
    /// incentive (the default).
    pub fn set_creator_incentive_bps(
        env: Env,
        admin: Address,
        incentive_bps: u32,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "set_creator_incentive_bps")?;

        if incentive_bps > 10_000 {
            return Err(PredifiError::InvalidFeeBps);
        }

        env.storage()
            .instance()
            .set(&DataKey::CreatorIncentiveBps, &incentive_bps);
        Self::extend_instance(&env);

        let min_participants = Self::get_creator_min_participants(env.clone());
        CreatorIncentiveConfigUpdatedEvent {
            admin,
            incentive_bps,
            min_participants,
        }
        .publish(&env);

        Ok(())
    }

    /// Current creator incentive rate in basis points. Returns `0` (disabled)
    /// until an admin opts in via `set_creator_incentive_bps`.
    pub fn get_creator_incentive_bps(env: Env) -> u32 {
        let bps = env
            .storage()
            .instance()
            .get(&DataKey::CreatorIncentiveBps)
            .unwrap_or(0u32);
        Self::extend_instance(&env);
        bps
    }

    /// Set the minimum `Pool.participants_count` a pool must reach before its
    /// creator becomes eligible to claim the incentive. Caller must have
    /// Admin role (0).
    pub fn set_creator_min_participants(
        env: Env,
        admin: Address,
        min_participants: u32,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "set_creator_min_participants")?;

        env.storage()
            .instance()
            .set(&DataKey::CreatorMinParticipants, &min_participants);
        Self::extend_instance(&env);

        let incentive_bps = Self::get_creator_incentive_bps(env.clone());
        CreatorIncentiveConfigUpdatedEvent {
            admin,
            incentive_bps,
            min_participants,
        }
        .publish(&env);

        Ok(())
    }

    /// Minimum participant count required for creator-incentive eligibility.
    /// Falls back to `DEFAULT_CREATOR_MIN_PARTICIPANTS` when no admin override
    /// has been stored yet.
    pub fn get_creator_min_participants(env: Env) -> u32 {
        let min_participants = env
            .storage()
            .instance()
            .get(&DataKey::CreatorMinParticipants)
            .unwrap_or(DEFAULT_CREATOR_MIN_PARTICIPANTS);
        Self::extend_instance(&env);
        min_participants
    }

    /// Claim the pool-creator incentive for a resolved pool.
    ///
    /// # Arguments
    /// * `creator` - Must match `Pool.creator` exactly; must provide auth.
    /// * `pool_id` - The resolved pool to claim the incentive for.
    ///
    /// # Returns
    /// * `Ok(amount)` - The incentive amount (in the pool's token) transferred
    ///   to the creator.
    ///
    /// # Errors
    /// * `PoolNotFound` - no pool exists for `pool_id`.
    /// * `Unauthorized` - caller is not `Pool.creator`.
    /// * `InvalidPoolState` - pool is not in `MarketState::Resolved` (e.g.
    ///   still active, canceled, or disputed — canceled pools refund stake in
    ///   full and generate no fee to share).
    /// * `MinParticipantsNotMet` - `Pool.participants_count` is below
    ///   `get_creator_min_participants()`.
    /// * `CreatorIncentiveAlreadyPaid` - already claimed for this pool.
    /// * `InvalidFeeBps` - the incentive rate is `0` (disabled protocol-wide).
    /// * `InvalidAmount` - the computed incentive amount is `<= 0` (e.g. the
    ///   pool's protocol fee itself rounded to zero).
    #[allow(clippy::needless_borrows_for_generic_args)]
    pub fn claim_creator_incentive(
        env: Env,
        creator: Address,
        pool_id: u64,
    ) -> Result<i128, PredifiError> {
        Self::require_not_paused(&env)?;
        creator.require_auth();

        Self::enter_reentrancy_guard(&env);

        let result: Result<i128, PredifiError> = (|| {
            // --- CHECKS ---

            let pool_key = DataKey::Pool(pool_id);
            let pool: Pool = match env.storage().persistent().get(&pool_key) {
                Some(p) => p,
                None => return Err(PredifiError::PoolNotFound),
            };
            Self::extend_persistent(&env, &pool_key);

            if pool.creator != creator {
                return Err(PredifiError::Unauthorized);
            }

            if pool.state != MarketState::Resolved {
                return Err(PredifiError::InvalidPoolState);
            }

            let min_participants = Self::get_creator_min_participants(env.clone());
            if pool.participants_count < min_participants {
                return Err(PredifiError::MinParticipantsNotMet);
            }

            let paid_key = DataKey::CreatorIncentivePaid(pool_id);
            if env.storage().persistent().has(&paid_key) {
                return Err(PredifiError::CreatorIncentiveAlreadyPaid);
            }

            let incentive_bps = Self::get_creator_incentive_bps(env.clone());
            if incentive_bps == 0 {
                return Err(PredifiError::InvalidFeeBps);
            }

            // Derived directly from pool state rather than amounts already
            // paid out via `claim_winnings`, so this does not depend on
            // claim ordering and can be called any time after resolution.
            let protocol_fee_total =
                calculate_protocol_fee(pool.total_stake, pool.fee_bps as i128)
                    .map_err(|_| PredifiError::InvalidAmount)?;

            let creator_amount = SafeMath::percentage(
                protocol_fee_total,
                incentive_bps as i128,
                RoundingMode::ProtocolFavor,
            )
            .map_err(|_| PredifiError::InvalidAmount)?;

            if creator_amount <= 0 {
                return Err(PredifiError::InvalidAmount);
            }

            // --- EFFECTS ---

            // Mark as paid before the external transfer (checks-effects-interactions).
            env.storage().persistent().set(&paid_key, &true);
            Self::bump_ttl(&env, &paid_key);

            // --- INTERACTIONS ---

            Self::validate_token_transfer(
                &env,
                &pool.token,
                &env.current_contract_address(),
                &creator,
                creator_amount,
            )?;

            let token_client = token::Client::new(&env, &pool.token);
            token_client.transfer(&env.current_contract_address(), &creator, &creator_amount);

            // --- Reputation bookkeeping ---

            let pools_key = DataKey::CreatorRewardedPools(creator.clone());
            let pools_rewarded: u32 = env.storage().persistent().get(&pools_key).unwrap_or(0);
            let pools_rewarded = pools_rewarded.saturating_add(1);
            env.storage().persistent().set(&pools_key, &pools_rewarded);
            Self::extend_persistent(&env, &pools_key);

            let total_key = DataKey::CreatorTotalIncentive(creator.clone());
            let total_earned: i128 = env.storage().persistent().get(&total_key).unwrap_or(0);
            let total_earned = total_earned.saturating_add(creator_amount);
            env.storage().persistent().set(&total_key, &total_earned);
            Self::extend_persistent(&env, &total_key);

            CreatorIncentivePaidEvent {
                pool_id,
                creator: creator.clone(),
                amount: creator_amount,
                participants_count: pool.participants_count,
            }
            .publish(&env);

            Ok(creator_amount)
        })();

        Self::exit_reentrancy_guard(&env);
        result
    }

    /// Return a pool creator's on-chain reputation / reward summary.
    ///
    /// `pools_rewarded` and `total_incentive_earned` only reflect pools for
    /// which `claim_creator_incentive` has actually been called and
    /// succeeded — this is not a count of all pools the address has created
    /// (use `get_pool` / off-chain indexing of `PoolCreatedEvent` for that).
    pub fn get_creator_stats(env: Env, creator: Address) -> CreatorStats {
        let pools_key = DataKey::CreatorRewardedPools(creator.clone());
        let pools_rewarded: u32 = env.storage().persistent().get(&pools_key).unwrap_or(0);
        if env.storage().persistent().has(&pools_key) {
            Self::extend_persistent(&env, &pools_key);
        }

        let total_key = DataKey::CreatorTotalIncentive(creator);
        let total_incentive_earned: i128 = env.storage().persistent().get(&total_key).unwrap_or(0);
        if env.storage().persistent().has(&total_key) {
            Self::extend_persistent(&env, &total_key);
        }

        CreatorStats {
            pools_rewarded,
            total_incentive_earned,
        }
    }
}

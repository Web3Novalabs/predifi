//! Referral domain: referrer tracking, referred-volume accounting and the
//! referral reward configuration.

use soroban_sdk::{contractimpl, Address, Env};

use crate::{
    DataKey, PredifiContract, PredifiContractArgs, PredifiContractClient, PredifiError,
    ReferralThresholdUpdatedEvent, ReferrerUpdatedEvent,
};

#[contractimpl]
impl PredifiContract {
    /// Set referral cut in basis points (e.g. 5000 = 50% of referrer's fee share). Caller must have Admin role (0).
    /// Must be ≤ 10_000.
    pub fn set_referral_cut_bps(
        env: Env,
        admin: Address,
        referral_cut_bps: u32,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "set_referral_cut_bps")?;
        assert!(
            referral_cut_bps <= 10_000,
            "referral_cut_bps must be at most 10000"
        );
        let mut config = Self::get_config(&env);
        config.referral_bps = referral_cut_bps;
        env.storage().instance().set(&DataKey::Config, &config);
        env.storage()
            .instance()
            .set(&DataKey::ReferralCutBps, &referral_cut_bps);
        Self::extend_instance(&env);
        Ok(())
    }

    /// Set the referral reward rate in basis points stored in the Config struct.
    ///
    /// This allows admins to run "referral seasons" (e.g. raise from 500 bps / 5%
    /// to 1000 bps / 10%) without any code changes.  The value is persisted in the
    /// `Config` instance-storage entry so it is picked up automatically by fee
    /// calculation logic.
    ///
    /// Caller must hold the Admin role. `bps` must be ≤ 10_000.
    pub fn set_referral_rate(env: Env, admin: Address, bps: u32) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "set_referral_rate")?;
        if bps > 10_000 {
            return Err(PredifiError::InvalidFeeBps);
        }
        let mut config = Self::get_config(&env);
        config.referral_bps = bps;
        env.storage().instance().set(&DataKey::Config, &config);
        Self::extend_instance(&env);
        Ok(())
    }

    /// Get referral cut in basis points (e.g. 5000 = 50% of referrer's fee share).
    pub fn get_referral_cut_bps(env: Env) -> u32 {
        Self::read_referral_cut_bps(&env)
    }

    /// Get total referred volume for a (referrer, pool_id) in base token units.
    pub fn get_referred_volume(env: Env, referrer: Address, pool_id: u64) -> i128 {
        let key = DataKey::ReferredVolume(referrer, pool_id);
        let vol = env.storage().persistent().get(&key).unwrap_or(0);
        if env.storage().persistent().has(&key) {
            Self::extend_persistent(&env, &key);
        }
        vol
    }

    /// Update or remove the referrer for a (user, pool_id) pair.
    ///
    /// Callable only by the user themselves. Allows correcting a mistaken or
    /// compromised referrer address before or after predictions are placed.
    ///
    /// # Arguments
    /// * `user`         - The user whose referrer is being updated (must provide auth).
    /// * `pool_id`      - The pool for which the referrer is being updated.
    /// * `new_referrer` - `Some(address)` to set a new referrer, `None` to remove it.
    ///
    /// # Errors
    /// * `Unauthorized` if the caller is not the user.
    pub fn update_referrer(
        env: Env,
        user: Address,
        pool_id: u64,
        new_referrer: Option<Address>,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        user.require_auth();
        let referrer_key = DataKey::Referrer(user.clone(), pool_id);
        match new_referrer {
            Some(ref addr) => {
                if addr == &user || addr == &env.current_contract_address() {
                    return Err(PredifiError::Unauthorized);
                }
                env.storage().persistent().set(&referrer_key, addr);
                Self::extend_persistent(&env, &referrer_key);
            }
            None => {
                env.storage().persistent().remove(&referrer_key);
            }
        }

        // Issue #1142: emit event so off-chain indexers stay in sync.
        ReferrerUpdatedEvent {
            user,
            pool_id,
            new_referrer,
        }
        .publish(&env);

        Ok(())
    }

    // ── Issue #1128: Referral volume threshold logic ──────────────────────────

    /// Set the minimum referred volume (in base token units) that a referrer
    /// must have accumulated in a pool before becoming eligible for a referral
    /// reward on `claim_winnings`.
    ///
    /// A value of `0` disables the threshold — every referrer qualifies
    /// regardless of volume (the original behaviour).
    ///
    /// This allows the protocol to filter out low-value or spam referrals that
    /// might be created solely to extract small fee cuts without meaningful
    /// traffic contribution.
    ///
    /// # Arguments
    /// * `admin`      - Address with Admin role (0), must provide auth.
    /// * `min_volume` - New threshold in base token units. Must be >= 0.
    ///
    /// # Errors
    /// * `Unauthorized`   — caller does not hold the Admin role.
    /// * `ContractPaused` — contract is currently paused.
    pub fn set_referral_volume_threshold(
        env: Env,
        admin: Address,
        min_volume: i128,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "set_referral_volume_threshold")?;

        if min_volume < 0 {
            return Err(PredifiError::InvalidAmount);
        }

        env.storage()
            .instance()
            .set(&DataKey::ReferralMinVolumeBps, &min_volume);
        Self::extend_instance(&env);

        ReferralThresholdUpdatedEvent { admin, min_volume }.publish(&env);

        Ok(())
    }

    /// Return the current referral volume threshold (in base token units).
    ///
    /// Returns `0` when no threshold has been configured (all referrers qualify).
    pub fn get_referral_volume_threshold(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::ReferralMinVolumeBps)
            .unwrap_or(0i128)
    }
}

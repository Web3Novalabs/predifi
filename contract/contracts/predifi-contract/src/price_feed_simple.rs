//! Simplified Price Feed Integration Module
//!
//! Lightweight adapter that mirrors the full [`price_feed`] module but uses
//! a tuple-based `PriceCondition` and a simplified oracle interface. It is
//! consumed by the `predifi-contract` when the oracle callback path is used
//! directly (e.g. in tests and when the Pyth SDK tuple encoding is desired).
//!
//! # Oracle Integration Pattern
//!
//! Follows the same **pull-based oracle integration pattern** as the full
//! module:
//!
//! 1. **Oracle Configuration** — admin calls [`PriceFeedAdapter::init_oracle`]
//!    once with the Pyth contract address, `max_price_age`, and
//!    `min_confidence_ratio`.
//! 2. **Data Ingestion** — keepers / oracle roles call
//!    [`PriceFeedAdapter::update_price_feed`] or
//!    [`PriceFeedAdapter::batch_update_price_feeds`] periodically to push
//!    fresh price data into `DataKey::PriceFeed(feed_pair)`.
//! 3. **Pool Binding** — a `PriceCondition` tuple
//!    `(feed_pair, target_price, operator, tolerance_bps)` is stored at
//!    `DataKey::PriceCondition(pool_id)` via
//!    [`PriceFeedAdapter::set_price_condition`].
//! 4. **Automated Resolution** — after `end_time + resolution_delay`,
//!    [`PriceFeedAdapter::resolve_pool_from_price`] retrieves the condition,
//!    validates the feed via [`PriceFeedAdapter::is_price_valid`], evaluates
//!    it via [`PriceFeedAdapter::evaluate_price_condition`], and returns the
//!    winning outcome (0 or 1).
//!
//! # Price Normalization
//!
//! All prices use the oracle's native decimal format (Pyth: typically 8
//! decimals). The contract does **not** convert decimals — `target_price`
//! in the condition must use the same precision as the feed's `price`.
//! `confidence` is the ± uncertainty band in the same units. The confidence
//! ratio used for validation is `(confidence * 10_000) / price` (basis
//! points), or equivalently `confidence <= price / 100` in this simplified
//! variant.
//!
//! # Staleness Checks
//!
//! [`PriceFeedAdapter::is_price_valid`] enforces three gates (all must pass):
//!
//! - **Expiration:** `current_time <= feed.expires_at`.
//! - **Age:** `current_time <= feed.timestamp + max_age` where `max_age`
//!   comes from `SimpleOracleConfig::max_price_age` or the caller's `max_age`
//!   argument.
//! - **Confidence:** `confidence <= price / 100` (≈ 1% ratio). Feeds with
//!   wider uncertainty are rejected.
//!
//! # Fallback Mechanisms
//!
//! - **Manual resolution** — if automated price resolution fails (missing
//!   feed, stale data, confidence too low) the pool can still be settled via
//!   the operator's `resolve_pool` path.
//! - **Deviation guard** — [`PriceFeedAdapter::update_price_feed`] rejects a
//!   new price that deviates by more than 5× from the previous stored price,
//!   mitigating flash-loan / oracle-manipulation spikes.
//! - **Cleanup** — [`PriceFeedAdapter::cleanup_expired_feeds`] removes
//!   expired entries from `DataKey::PriceFeedList` and storage, preventing
//!   unbounded growth. It is permissionless and can be called by any address.
//!
//! # Consumption During Pool Resolution
//!
//! ```text
//! resolve_pool_from_price(pool_id)
//!   └─ get_price_condition(pool_id)          // DataKey::PriceCondition
//!      └─ get_price_feed(feed_pair)           // DataKey::PriceFeed
//!         └─ is_price_valid(feed, max_age)    // expiration + age + confidence
//!            └─ evaluate_price_condition      // tolerance ± operator
//!               └─ return 0 or 1 → caller finalises Pool.state / Pool.outcome
//! ```
//!
//! The tolerance buffer is `tolerance_amount = target_price * tolerance_bps / 10_000`
//! and operators are: `0 = Equal` (within ± tolerance), `1 = Greater`
//! (`price > target + tolerance`), `2 = Less` (`price < target - tolerance`).

use crate::{DataKey, PredifiError, MAX_TOLERANCE};
use soroban_sdk::{contracttype, Address, Env, Symbol, Vec as SorobanVec};

/// Price feed data for external oracle integration (simplified tuple-backed form).
///
/// Mirrors [`crate::price_feed::PriceFeed`] but is the concrete type used by
/// the simplified adapter where `OracleConfig` is stored as
/// [`SimpleOracleConfig`]. Validity rules are identical — see module docs for
/// normalization and staleness checks.
///
/// # Price Data Validity
/// - `price` must be > 0
/// - `confidence` must be >= 0
/// - `timestamp` must be < current ledger time (strictly in the past)
/// - `expires_at` must be > `timestamp`
/// - Current time must be <= `expires_at` and within `max_price_age` of `timestamp`
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimplePriceFeed {
    /// The asset pair identifier (e.g., "ETH/USD").
    pub pair: Symbol,
    /// Current price in base token units (oracle native decimals, typically 8).
    pub price: i128,
    /// Confidence interval (± value) in the same decimal format as `price`.
    pub confidence: i128,
    /// Unix timestamp when the price was last updated.
    pub timestamp: u64,
    /// Unix timestamp when this price data expires.
    pub expires_at: u64,
}

/// Oracle configuration stored under `DataKey::OracleConfig`.
///
/// Controls global validation parameters for all price feeds managed by this
/// simplified adapter.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimpleOracleConfig {
    /// Pyth Network oracle contract address.
    pub pyth_contract: Address,
    /// Maximum age of price data in seconds before it is considered stale.
    pub max_price_age: u64,
    /// Minimum confidence ratio in basis points (1 bp = 0.01%).
    pub min_confidence_ratio: u32,
}

/// Price feed adapter for external oracle integration (simplified version).
///
/// Uses `DataKey::OracleConfig` for oracle configuration,
/// `DataKey::PriceFeed(feed_pair)` for price data, and
/// `DataKey::PriceCondition(pool_id)` for per-pool price conditions —
/// all defined in the canonical `DataKey` enum in `lib.rs`.
pub struct PriceFeedAdapter;

impl PriceFeedAdapter {
    /// Initialize global oracle configuration.
    ///
    /// One-time setup called by the admin to register the Pyth contract and
    /// staleness thresholds. Mirrors [`crate::price_feed::PriceFeedAdapter::init_oracle`]
    /// in the full module.
    ///
    /// # Parameters
    /// - `env` — Soroban environment.
    /// - `admin` — Admin address (must `require_auth`).
    /// - `pyth_contract` — Pyth Network oracle contract address.
    /// - `max_price_age` — Maximum age of price data in seconds (must be > 0).
    /// - `min_confidence_ratio` — Minimum confidence ratio in bps (must be <= 10_000).
    ///
    /// # Errors
    /// - `InvalidData` if `max_price_age == 0`.
    /// - `InvalidFeeBps` if `min_confidence_ratio > 10_000`.
    pub fn init_oracle(
        env: &Env,
        admin: &Address,
        pyth_contract: Address,
        max_price_age: u64,
        min_confidence_ratio: u32,
    ) -> Result<(), PredifiError> {
        admin.require_auth();

        if max_price_age == 0 {
            return Err(PredifiError::InvalidData);
        }
        if min_confidence_ratio > 10_000 {
            return Err(PredifiError::InvalidFeeBps);
        }

        let config = SimpleOracleConfig {
            pyth_contract,
            max_price_age,
            min_confidence_ratio,
        };

        env.storage()
            .persistent()
            .set(&DataKey::OracleConfig, &config);

        Ok(())
    }

    /// Get oracle configuration.
    ///
    /// Returns `None` if [`init_oracle`](Self::init_oracle) has not been called.
    pub fn get_oracle_config(env: &Env) -> Option<SimpleOracleConfig> {
        env.storage().persistent().get(&DataKey::OracleConfig)
    }

    /// Update price feed data (called by oracle keeper or by contract admin).
    ///
    /// Ingests a fresh price point for `feed_pair` into
    /// `DataKey::PriceFeed(feed_pair)` and tracks the pair in
    /// `DataKey::PriceFeedList` for later cleanup.
    ///
    /// # Oracle Integration Pattern
    /// Second step in the pull-based flow — called periodically by keepers
    /// (e.g. every 30 s) to keep on-chain data fresh.
    ///
    /// # Price Normalization
    /// `price` and `confidence` must use the same decimal precision as the
    /// oracle feed (Pyth: 8 decimals). No conversion is performed on-chain.
    ///
    /// # Validation
    /// - `price` > 0, `confidence` >= 0
    /// - `timestamp` < current ledger time, `expires_at` > `timestamp`
    /// - New `price` must be within 5× of the previous stored price
    ///   (deviation guard against flash-loan manipulation)
    ///
    /// # Errors
    /// - `InvalidAmount` if `price <= 0` or `confidence < 0`
    /// - `InvalidPoolState` / `InvalidData` for bad timestamps or excessive deviation
    pub fn update_price_feed(
        env: &Env,
        oracle: &Address,
        feed_pair: Symbol,
        price: i128,
        confidence: i128,
        timestamp: u64,
        expires_at: u64,
    ) -> Result<(), PredifiError> {
        oracle.require_auth();

        if price <= 0 || confidence < 0 {
            return Err(PredifiError::InvalidAmount);
        }

        if timestamp > env.ledger().timestamp() || expires_at <= timestamp {
            return Err(PredifiError::InvalidPoolState);
        }

        let feed_key = DataKey::PriceFeed(feed_pair.clone());

        // Price deviation protection: prevent flash loan manipulation by rejecting
        // price updates that deviate more than 5x from the previous price.
        const MAX_DEVIATION_MULTIPLIER: i128 = 5;
        if let Some(prev_feed) = env
            .storage()
            .persistent()
            .get::<DataKey, SimplePriceFeed>(&feed_key)
        {
            if prev_feed.price > 0 && price > 0 {
                let (lower, upper) = (
                    prev_feed
                        .price
                        .checked_div(MAX_DEVIATION_MULTIPLIER)
                        .unwrap_or(i128::MIN),
                    prev_feed
                        .price
                        .checked_mul(MAX_DEVIATION_MULTIPLIER)
                        .unwrap_or(i128::MAX),
                );
                if price < lower || price > upper {
                    return Err(PredifiError::InvalidData);
                }
            }
        }

        let feed = SimplePriceFeed {
            pair: feed_pair.clone(),
            price,
            confidence,
            timestamp,
            expires_at,
        };

        env.storage()
            .persistent()
            .set(&feed_key, &feed);

        // Track this feed pair in the global list for cleanup
        let mut list: SorobanVec<Symbol> = env
            .storage()
            .persistent()
            .get(&DataKey::PriceFeedList)
            .unwrap_or_else(|| SorobanVec::new(env));
        if !list.contains(feed_pair.clone()) {
            list.push_back(feed_pair);
            env.storage()
                .persistent()
                .set(&DataKey::PriceFeedList, &list);
        }

        Ok(())
    }

    /// Get current price feed data for a given pair.
    ///
    /// Returns `None` if no feed has been stored for `feed_pair`.
    pub fn get_price_feed(env: &Env, feed_pair: &Symbol) -> Option<SimplePriceFeed> {
        env.storage()
            .persistent()
            .get(&DataKey::PriceFeed(feed_pair.clone()))
    }

    /// Check if price data is valid and fresh.
    ///
    /// Performs the same three staleness gates as the full module:
    /// expiration (`expires_at`), age (`timestamp + max_age`), and confidence
    /// (`confidence <= price / 100`). Returns `true` only if all pass.
    ///
    /// # Fallback
    /// If this returns `false`, automated resolution will fail and the pool
    /// must be resolved manually via `resolve_pool`.
    pub fn is_price_valid(env: &Env, feed: &SimplePriceFeed, max_age: u64) -> bool {
        let current_time = env.ledger().timestamp();

        if current_time > feed.expires_at {
            return false;
        }

        if current_time > feed.timestamp + max_age {
            return false;
        }

        // Basic confidence check: confidence must be <= 1% of price
        if feed.confidence > feed.price / 100 {
            return false;
        }

        true
    }

    /// Set price condition for a pool.
    ///
    /// Binds `pool_id` to a price-based resolution criterion. The tuple form
    /// `(feed_pair, target_price, operator, tolerance_bps)` is stored at
    /// `DataKey::PriceCondition(pool_id)`.
    ///
    /// # Parameters
    /// - `feed_pair` — must match a symbol registered via `update_price_feed`.
    /// - `target_price` — in oracle native decimals (same as feed `price`).
    /// - `operator` — 0: Equal (within ± tolerance), 1: Greater, 2: Less.
    /// - `tolerance_bps` — buffer around target in basis points (1 bp = 0.01%).
    pub fn set_price_condition(
        env: &Env,
        pool_id: u64,
        feed_pair: Symbol,
        target_price: i128,
        operator: u32,
        tolerance_bps: u32,
    ) -> Result<(), PredifiError> {
        env.storage().persistent().set(
            &DataKey::PriceCondition(pool_id),
            &(feed_pair, target_price, operator, tolerance_bps),
        );

        Ok(())
    }

    /// Get price condition for a pool.
    ///
    /// Returns `None` if no condition has been set for `pool_id`.
    pub fn get_price_condition(env: &Env, pool_id: u64) -> Option<(Symbol, i128, u32, u32)> {
        env.storage()
            .persistent()
            .get(&DataKey::PriceCondition(pool_id))
    }

    /// Evaluate price condition against current price data.
    ///
    /// 1. Fetches `PriceFeed` for `condition.feed_pair`.
    /// 2. Validates via [`is_price_valid`](Self::is_price_valid).
    /// 3. Computes `tolerance_amount = target_price * tolerance_bps / 10_000`.
    /// 4. Applies the operator: Equal (within bounds), Greater, or Less.
    ///
    /// # Errors
    /// - `PriceFeedNotFound` if no feed exists for the pair.
    /// - `PriceDataInvalid` if the feed is stale / low-confidence.
    /// - `InvalidPoolState` if `operator` is not 0, 1, or 2.
    pub fn evaluate_price_condition(
        env: &Env,
        condition: &(Symbol, i128, u32, u32),
        max_age: u64,
    ) -> Result<bool, PredifiError> {
        let (feed_pair, target_price, operator_type, tolerance_bps) = condition;

        let feed = Self::get_price_feed(env, feed_pair).ok_or(PredifiError::PriceFeedNotFound)?;

        if !Self::is_price_valid(env, &feed, max_age) {
            return Err(PredifiError::PriceDataInvalid);
        }

        let tolerance_amount = (target_price * *tolerance_bps as i128) / MAX_TOLERANCE as i128;

        let result = match operator_type {
            0 => {
                feed.price >= target_price - tolerance_amount
                    && feed.price <= target_price + tolerance_amount
            }
            1 => feed.price > target_price + tolerance_amount,
            2 => feed.price < target_price - tolerance_amount,
            _ => return Err(PredifiError::InvalidPoolState),
        };

        Ok(result)
    }

    /// Resolve pool based on price condition.
    ///
    /// Convenience wrapper around [`evaluate_price_condition`](Self::evaluate_price_condition):
    /// returns outcome `1` (condition met) or `0` (not met). The caller is
    /// responsible for writing `Pool.state` / `Pool.outcome` and enforcing
    /// `resolution_delay`.
    ///
    /// # Consumption During Pool Resolution
    /// Called after `pool.end_time` once the configured price feed is fresh.
    /// If it returns an error (missing feed, stale data), fall back to manual
    /// `resolve_pool` so the market can still be settled.
    pub fn resolve_pool_from_price(
        env: &Env,
        pool_id: u64,
        max_age: u64,
    ) -> Result<u32, PredifiError> {
        let condition =
            Self::get_price_condition(env, pool_id).ok_or(PredifiError::PriceConditionNotSet)?;

        let condition_met = Self::evaluate_price_condition(env, &condition, max_age)?;

        Ok(if condition_met { 1 } else { 0 })
    }

    /// Batch update multiple price feeds in one transaction.
    ///
    /// Iterates `updates` and calls [`update_price_feed`](Self::update_price_feed)
    /// for each entry. The call is atomic per-fe ed validation — if any entry
    /// fails validation the whole batch reverts (Soroban host error semantics).
    pub fn batch_update_price_feeds(
        env: &Env,
        oracle: &Address,
        updates: SorobanVec<(Symbol, i128, i128, u64, u64)>,
    ) -> Result<(), PredifiError> {
        oracle.require_auth();

        for i in 0..updates.len() {
            let (feed_pair, price, confidence, timestamp, expires_at) = updates.get(i).unwrap();

            Self::update_price_feed(
                env,
                oracle,
                feed_pair.clone(),
                price,
                confidence,
                timestamp,
                expires_at,
            )?;
        }

        Ok(())
    }

    /// Clean up expired price feeds. Permissionless — callable by any address.
    ///
    /// Iterates the tracked feed list, removes entries whose `expires_at` is in
    /// the past, and returns the number of feeds removed. Prevents storage bloat
    /// when oracle feeds rotate frequently.
    pub fn cleanup_expired_feeds(env: &Env) -> u32 {
        let current_time = env.ledger().timestamp();

        let list: SorobanVec<Symbol> = env
            .storage()
            .persistent()
            .get(&DataKey::PriceFeedList)
            .unwrap_or_else(|| SorobanVec::new(env));

        let mut remaining: SorobanVec<Symbol> = SorobanVec::new(env);
        let mut removed: u32 = 0;

        for i in 0..list.len() {
            let pair = list.get(i).unwrap();
            let expired = env
                .storage()
                .persistent()
                .get::<DataKey, SimplePriceFeed>(&DataKey::PriceFeed(pair.clone()))
                .map(|feed| feed.expires_at < current_time)
                .unwrap_or(true); // missing entry counts as expired

            if expired {
                env.storage().persistent().remove(&DataKey::PriceFeed(pair));
                removed += 1;
            } else {
                remaining.push_back(pair);
            }
        }

        env.storage()
            .persistent()
            .set(&DataKey::PriceFeedList, &remaining);

        removed
    }
}

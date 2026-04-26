use crate::{DataKey, PredifiError};
use soroban_sdk::{contracttype, Address, Env, Symbol, Vec as SorobanVec};

/// Oracle configuration stored under `DataKey::OracleConfig`.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimplePriceFeed {
    /// The asset pair identifier (e.g., "ETH/USD").
    pub pair: Symbol,
    /// Current price in base token units.
    pub price: i128,
    /// Confidence interval (± value).
    pub confidence: i128,
    /// Unix timestamp when the price was last updated.
    pub timestamp: u64,
    /// Unix timestamp when this price data expires.
    pub expires_at: u64,
}

/// Oracle configuration stored under `DataKey::OracleConfig`.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimpleOracleConfig {
    /// Pyth Network oracle contract address.
    pub pyth_contract: Address,
    /// Maximum age of price data in seconds before it is considered stale.
    pub max_price_age: u64,
    /// Minimum confidence ratio in basis points.
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
    /// Initialize oracle configuration.
    pub fn init_oracle(
        env: &Env,
        admin: &Address,
        pyth_contract: Address,
        max_price_age: u64,
        min_confidence_ratio: u32,
    ) -> Result<(), PredifiError> {
        admin.require_auth();

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
    pub fn get_oracle_config(env: &Env) -> Option<SimpleOracleConfig> {
        env.storage().persistent().get(&DataKey::OracleConfig)
    }

    /// Update price feed data (called by oracle keeper).
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

        let feed = SimplePriceFeed {
            pair: feed_pair.clone(),
            price,
            confidence,
            timestamp,
            expires_at,
        };

        env.storage()
            .persistent()
            .set(&DataKey::PriceFeed(feed_pair.clone()), &feed);

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
    pub fn get_price_feed(env: &Env, feed_pair: &Symbol) -> Option<SimplePriceFeed> {
        env.storage()
            .persistent()
            .get(&DataKey::PriceFeed(feed_pair.clone()))
    }

    /// Check if price data is valid and fresh.
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
    /// Stores `(feed_pair, target_price, operator, tolerance_bps)` under
    /// `DataKey::PriceCondition(pool_id)`.
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
    pub fn get_price_condition(env: &Env, pool_id: u64) -> Option<(Symbol, i128, u32, u32)> {
        env.storage()
            .persistent()
            .get(&DataKey::PriceCondition(pool_id))
    }

    /// Evaluate price condition against current price data.
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

        let tolerance_amount = (target_price * *tolerance_bps as i128) / 10000;

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

    /// Batch update multiple price feeds.
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
    /// the past, and returns the number of feeds removed.
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
                env.storage()
                    .persistent()
                    .remove(&DataKey::PriceFeed(pair));
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

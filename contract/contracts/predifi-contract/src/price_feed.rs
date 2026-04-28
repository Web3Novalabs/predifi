//! Price Feed Integration Module
//!
//! This module provides a robust adapter for integrating external oracles (e.g., Pyth Network)
//! with PrediFi prediction pools. It enables automated, price-based market resolution
//! without requiring manual intervention from operators or oracles.
//!
//! ## Integration Flow
//! 1. **Initialize Oracle**: Call `init_oracle` with the Pyth contract address and validation parameters.
//! 2. **Update Feeds**: Oracle keepers call `update_price_feed` periodically to push fresh data.
//! 3. **Set Pool Condition**: During pool creation or setup, call `set_price_condition` to link
//!    a pool to a specific price feed and target outcome.
//! 4. **Resolve Pool**: Once the market ends, call `resolve_pool_from_price` to automatically
//!    determine the winning outcome based on the latest valid price data.

use crate::{DataKey, PredifiError};
use soroban_sdk::{contracttype, Address, Env, Symbol, Vec};

/// Price feed data structure for external oracle integration.
///
/// This struct contains real-time price data from an oracle (e.g., Pyth Network).
/// It is used for automated market resolution based on price conditions.
///
/// # Price Data Validity
/// - `price` must be positive
/// - `confidence` must be non-negative
/// - `timestamp` must be in the past
/// - `expires_at` must be greater than `timestamp`
/// - Current time must be <= `expires_at` for the price to be considered valid
#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct PriceFeed {
    /// The asset pair identifier (e.g., "ETH/USD", "BTC/USD").
    pub pair: Symbol,
    /// Current price of the asset pair in base token units.
    pub price: i128,
    /// Confidence interval representing the uncertainty of the price (± value).
    /// Lower confidence values indicate more reliable price data.
    pub confidence: i128,
    /// Unix timestamp when the price was last updated.
    pub timestamp: u64,
    /// Unix timestamp when this price data expires.
    /// Price data is considered invalid after this time.
    pub expires_at: u64,
}

/// Price condition for automated market resolution.
///
/// This struct defines a price-based condition that can be used to
/// automatically resolve a prediction pool. The condition specifies an
/// asset pair, a target price level, and a comparison operator.
///
/// # Technical Requirements
/// - `feed_pair`: Must match a symbol registered via `update_price_feed` (e.g., `symbol!("ETH/USD")`).
/// - `target_price`: Specified in the same decimal format as the oracle feed (typically 8 decimals).
/// - `operator`: Defines the winning criteria (0: Equal, 1: Greater, 2: Less).
/// - `tolerance_bps`: Defines a "buffer" around the target price to prevent resolution
///   flips due to minor noise. 100 bps = 1.0%.
///
/// # Example Usage
/// For a pool predicting "Will BTC exceed $60,000 at expiry?":
/// - `feed_pair`: "BTC/USD"
/// - `target_price`: 6000000000000
/// - `operator`: 1 (Greater Than)
/// - `tolerance_bps`: 50 (0.5% tolerance)
#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct PriceCondition {
    /// The price feed pair to monitor (e.g., "ETH/USD").
    pub feed_pair: Symbol,
    /// Target price to compare against for resolution (e.g., 3000 * 10^8).
    pub target_price: i128,
    /// Comparison operator for the price condition:
    /// - `0`: Equal (price is within `target_price ± tolerance`)
    /// - `1`: Greater than (price > `target_price + tolerance`)
    /// - `2`: Less than (price < `target_price - tolerance`)
    pub operator: u32,
    /// Tolerance for price comparison in basis points (1 bp = 0.01%).
    /// Prevents resolution issues if the price is exactly at the boundary.
    pub tolerance_bps: u32,
}

/// Oracle configuration for price feeds.
///
/// This struct contains global settings for oracle integration,
/// controlling how price data is validated and consumed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct OracleConfig {
    /// Pyth Network oracle contract address on Stellar.
    /// This contract provides decentralized price feeds.
    pub pyth_contract: Address,
    /// Maximum age of price data in seconds.
    /// Price data older than this is considered stale and invalid.
    pub max_price_age: u64,
    /// Minimum confidence ratio in basis points (1 bp = 0.01%).
    /// Lower values indicate higher confidence. If the actual confidence
    /// ratio exceeds this threshold, the price data is rejected.
    /// For example, 100 bps = 1% maximum confidence ratio.
    pub min_confidence_ratio: u32,
}

/// Storage keys for price feed data.
///
/// Deprecated: use `DataKey` from `lib.rs` directly. This type alias is kept
/// for documentation purposes only and will be removed in a future version.
///
/// All price-feed storage now uses the canonical `DataKey` variants:
/// - `DataKey::OracleConfig` — oracle configuration
/// - `DataKey::PriceFeed(feed_pair)` — price feed data
/// - `DataKey::PriceCondition(pool_id)` — per-pool price conditions
///
/// Price feed adapter for external oracle integration
#[allow(dead_code)]
pub struct PriceFeedAdapter;

#[allow(dead_code)]
impl PriceFeedAdapter {
    /// Initialize global oracle configuration.
    ///
    /// This should be called by the contract admin once during protocol setup.
    /// It registers the official Pyth contract address and sets the safety
    /// parameters for price staleness and confidence.
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

        let config = OracleConfig {
            pyth_contract: pyth_contract.clone(),
            max_price_age,
            min_confidence_ratio,
        };

        env.storage()
            .persistent()
            .set(&DataKey::OracleConfig, &config);

        Ok(())
    }

    /// Get oracle configuration
    pub fn get_oracle_config(env: &Env) -> OracleConfig {
        env.storage()
            .persistent()
            .get(&DataKey::OracleConfig)
            .expect("Oracle config not initialized")
    }

    /// Update price feed data for a specific asset pair.
    ///
    /// This is typically called by an off-chain keeper or a specialized
    /// oracle role. It updates the internal state with the latest price,
    /// confidence level, and timestamp from the oracle provider.
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

        // Validate price data
        if price <= 0 || confidence < 0 {
            return Err(PredifiError::InvalidAmount);
        }

        // Require timestamp to be strictly in the past (at least 1 second old)
        if timestamp >= env.ledger().timestamp() || expires_at <= timestamp {
            return Err(PredifiError::InvalidData);
        }

        let feed = PriceFeed {
            pair: feed_pair.clone(),
            price,
            confidence,
            timestamp,
            expires_at,
        };

        // Store price feed data
        env.storage()
            .persistent()
            .set(&DataKey::PriceFeed(feed_pair.clone()), &feed);

        // Note: last-update timestamp is embedded in PriceFeed.timestamp;
        // no separate LastUpdate key is needed.

        Ok(())
    }

    /// Get current price feed data
    pub fn get_price_feed(env: &Env, feed_pair: &Symbol) -> Option<PriceFeed> {
        let feed: Option<PriceFeed> = env
            .storage()
            .persistent()
            .get(&DataKey::PriceFeed(feed_pair.clone()));

        feed
    }

    /// Check if price feed data is valid and fresh
    pub fn is_price_valid(env: &Env, feed: &PriceFeed) -> bool {
        let current_time = env.ledger().timestamp();
        let config = Self::get_oracle_config(env);

        // Check if price data is expired
        if current_time > feed.expires_at {
            return false;
        }

        // Check if price data is too old
        if current_time > feed.timestamp + config.max_price_age {
            return false;
        }

        // Check confidence ratio
        let confidence_ratio = (feed.confidence * 10000) / feed.price;
        if confidence_ratio > config.min_confidence_ratio as i128 {
            return false;
        }

        true
    }

    /// Set price condition for a pool
    pub fn set_price_condition(
        env: &Env,
        pool_id: u64,
        condition: PriceCondition,
    ) -> Result<(), PredifiError> {
        env.storage()
            .persistent()
            .set(&DataKey::PriceCondition(pool_id), &condition);

        Ok(())
    }

    /// Get price condition for a pool
    pub fn get_price_condition(env: &Env, pool_id: u64) -> Option<PriceCondition> {
        env.storage()
            .persistent()
            .get(&DataKey::PriceCondition(pool_id))
    }

    /// Evaluate price condition against current price data
    pub fn evaluate_price_condition(
        env: &Env,
        condition: &PriceCondition,
    ) -> Result<bool, PredifiError> {
        let feed =
            Self::get_price_feed(env, &condition.feed_pair).ok_or(PredifiError::PoolNotResolved)?;

        // Validate price data
        if !Self::is_price_valid(env, &feed) {
            return Err(PredifiError::ResolutionDelayNotMet);
        }

        // Calculate tolerance amount
        let tolerance_amount = (condition.target_price * condition.tolerance_bps as i128) / 10000;

        // Evaluate condition based on operator
        let result = match condition.operator {
            0 => {
                // Equal
                feed.price >= condition.target_price - tolerance_amount
                    && feed.price <= condition.target_price + tolerance_amount
            }
            1 => {
                // Greater than
                feed.price > condition.target_price + tolerance_amount
            }
            2 => {
                // Less than
                feed.price < condition.target_price - tolerance_amount
            }
            _ => return Err(PredifiError::InvalidPoolState),
        };

        Ok(result)
    }

    /// Resolve a prediction pool using its configured price condition.
    ///
    /// This is the primary entry point for automated resolution. It retrieves
    /// the pool's condition, evaluates it against the current market price,
    /// and returns the winning outcome index.
    ///
    /// # Resolution Logic
    /// - If `evaluate_price_condition` returns `true` -> Outcome `1` (Yes/Target Met)
    /// - If `evaluate_price_condition` returns `false` -> Outcome `0` (No/Target Missed)
    pub fn resolve_pool_from_price(env: &Env, pool_id: u64) -> Result<u32, PredifiError> {
        let condition =
            Self::get_price_condition(env, pool_id).ok_or(PredifiError::PoolNotResolved)?;

        let condition_met = Self::evaluate_price_condition(env, &condition)?;

        // Return outcome: 1 if condition met, 0 if not met
        Ok(if condition_met { 1 } else { 0 })
    }

    /// Batch update multiple price feeds
    pub fn batch_update_price_feeds(
        env: &Env,
        oracle: &Address,
        updates: Vec<(Symbol, i128, i128, u64, u64)>,
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

    /// Get all available price feed pairs
    pub fn get_available_feeds(env: &Env) -> Vec<Symbol> {
        // This would typically scan storage for all PriceFeed keys
        // For now, return empty vector - implementation depends on storage scanning capabilities
        Vec::new(env)
    }

    /// Clean up expired price feeds
    pub fn cleanup_expired_feeds(env: &Env) -> Result<u32, PredifiError> {
        let _current_time = env.ledger().timestamp();
        let cleaned_count = 0u32;

        // This would typically scan all price feeds and remove expired ones
        // Implementation depends on storage scanning capabilities

        Ok(cleaned_count)
    }
}

#[cfg(test)]
mod tests {}
// #[cfg(test)]
// mod tests;

#![no_std]

use soroban_sdk::{Address, Env, Symbol, Vec, BytesN};
use crate::{PredifiError, DataKey};

/// Price feed data structure for external oracle integration
#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct PriceFeed {
    /// The asset pair (e.g., "ETH/USD")
    pub pair: Symbol,
    /// Current price
    pub price: i128,
    /// Confidence interval (±)
    pub confidence: i128,
    /// Timestamp of the price update
    pub timestamp: u64,
    /// Expiration time for this price data
    pub expires_at: u64,
}

/// Price condition for automated market resolution
#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct PriceCondition {
    /// The price feed pair to monitor
    pub feed_pair: Symbol,
    /// Target price to compare against
    pub target_price: i128,
    /// Comparison operator: 0 = equal, 1 = greater than, 2 = less than
    pub operator: u32,
    /// Tolerance for price comparison (in basis points)
    pub tolerance_bps: u32,
}

/// Oracle configuration for price feeds
#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct OracleConfig {
    /// Pyth Network contract address
    pub pyth_contract: Address,
    /// Maximum age of price data (in seconds)
    pub max_price_age: u64,
    /// Minimum confidence interval (relative to price)
    pub min_confidence_ratio: u32, // in basis points
}

/// Storage keys for price feed data
#[derive(Clone)]
#[contracttype]
pub enum PriceFeedDataKey {
    /// Oracle configuration
    OracleConfig,
    /// Registered price feeds: PriceFeed(feed_pair) -> PriceFeed
    PriceFeed(Symbol),
    /// Price conditions for pools: PriceCondition(pool_id) -> PriceCondition
    PriceCondition(u64),
    /// Last update timestamp for each feed
    LastUpdate(Symbol),
}

/// Price feed adapter for external oracle integration
pub struct PriceFeedAdapter;

impl PriceFeedAdapter {
    /// Initialize oracle configuration
    pub fn init_oracle(
        env: &Env,
        admin: &Address,
        pyth_contract: Address,
        max_price_age: u64,
        min_confidence_ratio: u32,
    ) -> Result<(), PredifiError> {
        admin.require_auth();
        
        let config = OracleConfig {
            pyth_contract: pyth_contract.clone(),
            max_price_age,
            min_confidence_ratio,
        };
        
        env.storage()
            .persistent()
            .set(&PriceFeedDataKey::OracleConfig, &config);
        
        Ok(())
    }

    /// Get oracle configuration
    pub fn get_oracle_config(env: &Env) -> OracleConfig {
        env.storage()
            .persistent()
            .get(&PriceFeedDataKey::OracleConfig)
            .expect("Oracle config not initialized")
    }

    /// Update price feed data (called by oracle keeper)
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
        
        if timestamp > env.ledger().timestamp() || expires_at <= timestamp {
            return Err(PredifiError::InvalidPoolState);
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
            .set(&PriceFeedDataKey::PriceFeed(feed_pair.clone()), &feed);
        
        env.storage()
            .persistent()
            .set(&PriceFeedDataKey::LastUpdate(feed_pair), &timestamp);
        
        Ok(())
    }

    /// Get current price feed data
    pub fn get_price_feed(env: &Env, feed_pair: &Symbol) -> Option<PriceFeed> {
        let feed: Option<PriceFeed> = env.storage()
            .persistent()
            .get(&PriceFeedDataKey::PriceFeed(feed_pair.clone()));
        
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
        if confidence_ratio > config.min_confidence_ratio {
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
            .set(&PriceFeedDataKey::PriceCondition(pool_id), &condition);
        
        Ok(())
    }

    /// Get price condition for a pool
    pub fn get_price_condition(env: &Env, pool_id: u64) -> Option<PriceCondition> {
        env.storage()
            .persistent()
            .get(&PriceFeedDataKey::PriceCondition(pool_id))
    }

    /// Evaluate price condition against current price data
    pub fn evaluate_price_condition(
        env: &Env,
        condition: &PriceCondition,
    ) -> Result<bool, PredifiError> {
        let feed = Self::get_price_feed(env, &condition.feed_pair)
            .ok_or(PredifiError::PoolNotResolved)?;
        
        // Validate price data
        if !Self::is_price_valid(env, &feed) {
            return Err(PredifiError::ResolutionDelayNotMet);
        }
        
        // Calculate tolerance amount
        let tolerance_amount = (condition.target_price * condition.tolerance_bps as i128) / 10000;
        
        // Evaluate condition based on operator
        let result = match condition.operator {
            0 => { // Equal
                feed.price >= condition.target_price - tolerance_amount &&
                feed.price <= condition.target_price + tolerance_amount
            },
            1 => { // Greater than
                feed.price > condition.target_price + tolerance_amount
            },
            2 => { // Less than
                feed.price < condition.target_price - tolerance_amount
            },
            _ => return Err(PredifiError::InvalidPoolState),
        };
        
        Ok(result)
    }

    /// Resolve pool based on price condition
    pub fn resolve_pool_from_price(
        env: &Env,
        pool_id: u64,
    ) -> Result<u32, PredifiError> {
        let condition = Self::get_price_condition(env, pool_id)
            .ok_or(PredifiError::PoolNotResolved)?;
        
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
mod tests;

use crate::{DataKey, PredifiError};
use soroban_sdk::{Address, Env, Symbol, Vec};

/// Price feed adapter for external oracle integration (simplified version)
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

        // Store oracle config using existing storage keys
        let config_key = DataKey::TokenWhitelist(pyth_contract.clone());

        // Store config values as tuple
        env.storage()
            .persistent()
            .set(&config_key, &(max_price_age, min_confidence_ratio));

        Ok(())
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

        // Store price data using existing storage pattern
        // Use OutcomeStake with a fixed pool_id for price data
        let price_key = DataKey::OutcomeStake(999999, 0); // Fixed pool_id for price feeds
        env.storage().persistent().set(
            &price_key,
            &(feed_pair, price, confidence, timestamp, expires_at),
        );

        Ok(())
    }

    /// Get current price feed data
    pub fn get_price_feed(env: &Env, feed_pair: &Symbol) -> Option<(i128, i128, u64, u64)> {
        let price_key = DataKey::OutcomeStake(999999, 0); // Fixed pool_id for price feeds

        // Get all price data and find matching feed
        if let Some(price_data) = env
            .storage()
            .persistent()
            .get::<DataKey, (Symbol, i128, i128, u64, u64)>(&price_key)
        {
            let (stored_pair, price, confidence, timestamp, expires_at) = price_data;
            if stored_pair == *feed_pair {
                return Some((price, confidence, timestamp, expires_at));
            }
        }

        None
    }

    /// Check if price data is valid and fresh
    pub fn is_price_valid(env: &Env, price_data: &(i128, i128, u64, u64), max_age: u64) -> bool {
        let (price, confidence, timestamp, expires_at) = price_data;
        let current_time = env.ledger().timestamp();

        // Check if price data is expired
        if current_time > *expires_at {
            return false;
        }

        // Check if price data is too old
        if current_time > timestamp + max_age {
            return false;
        }

        // Basic confidence check
        if *confidence > *price / 100 {
            return false;
        }

        true
    }

    /// Set price condition for a pool
    pub fn set_price_condition(
        env: &Env,
        pool_id: u64,
        feed_pair: Symbol,
        target_price: i128,
        operator: u32,
        tolerance_bps: u32,
    ) -> Result<(), PredifiError> {
        // Store condition using existing storage pattern
        let condition_key = DataKey::OutcomeStake(pool_id, 1);
        env.storage().persistent().set(
            &condition_key,
            &(feed_pair, target_price, operator, tolerance_bps),
        );

        Ok(())
    }

    /// Get price condition for a pool
    pub fn get_price_condition(env: &Env, pool_id: u64) -> Option<(Symbol, i128, u32, u32)> {
        let condition_key = DataKey::OutcomeStake(pool_id, 1);
        env.storage().persistent().get(&condition_key)
    }

    /// Evaluate price condition against current price data
    pub fn evaluate_price_condition(
        env: &Env,
        condition: &(Symbol, i128, u32, u32),
        max_age: u64,
    ) -> Result<bool, PredifiError> {
        let (feed_pair, target_price, operator_type, tolerance_bps) = condition;

        let price_data =
            Self::get_price_feed(env, feed_pair).ok_or(PredifiError::PriceFeedNotFound)?;

        // Validate price data
        if !Self::is_price_valid(env, &price_data, max_age) {
            return Err(PredifiError::PriceDataInvalid);
        }

        let (price, _confidence, _timestamp, _expires_at) = price_data;

        // Calculate tolerance amount
        let tolerance_amount = (target_price * *tolerance_bps as i128) / 10000;

        // Evaluate condition based on operator
        let result = match operator_type {
            0 => {
                // Equal
                price >= target_price - tolerance_amount && price <= target_price + tolerance_amount
            }
            1 => {
                // Greater than
                price > target_price + tolerance_amount
            }
            2 => {
                // Less than
                price < target_price - tolerance_amount
            }
            _ => return Err(PredifiError::InvalidPoolState),
        };

        Ok(result)
    }

    /// Resolve pool based on price condition
    pub fn resolve_pool_from_price(
        env: &Env,
        pool_id: u64,
        max_age: u64,
    ) -> Result<u32, PredifiError> {
        let condition =
            Self::get_price_condition(env, pool_id).ok_or(PredifiError::PriceConditionNotSet)?;

        let condition_met = Self::evaluate_price_condition(env, &condition, max_age)?;

        // Return outcome: 1 if condition met, 0 if not met
        Ok(if condition_met { 1 } else { 0 })
    }

    /// Get oracle configuration
    pub fn get_oracle_config(env: &Env, pyth_contract: &Address) -> Option<(u64, u32)> {
        let config_key = DataKey::TokenWhitelist(pyth_contract.clone());
        env.storage().persistent().get(&config_key)
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

    /// Clean up expired price feeds
    pub fn cleanup_expired_feeds(env: &Env, _max_age: u64) -> Result<u32, PredifiError> {
        let _current_time = env.ledger().timestamp();
        let cleaned_count = 0u32;

        // This would typically scan all price feeds and remove expired ones
        // For now, return count as placeholder
        // Implementation depends on storage scanning capabilities

        Ok(cleaned_count)
    }
}

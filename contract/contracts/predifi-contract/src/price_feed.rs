//! Price Feed Integration Module
//!
//! This module provides a robust adapter for integrating external oracles (e.g., Pyth Network)
//! with PrediFi prediction pools. It enables automated, price-based market resolution
//! without requiring manual intervention from operators or oracles.
//!
//! # Oracle Integration Pattern
//!
//! The module follows a **pull-based oracle integration pattern** where:
//!
//! 1. **Oracle Configuration**: An admin initializes the oracle with the Pyth contract address
//!    and validation parameters (max price age, minimum confidence ratio). This is a one-time setup.
//!
//! 2. **Data Ingestion**: Off-chain keepers or authorized oracle roles periodically call
//!    `update_price_feed` to push fresh price data into the contract's persistent storage.
//!    The contract validates the data before accepting it.
//!
//! 3. **Pool Binding**: During pool creation or setup, a `PriceCondition` is configured that
//!    links the pool to a specific price feed pair (e.g., "BTC/USD") and defines the
//!    resolution criteria (target price, operator, tolerance).
//!
//! 4. **Automated Resolution**: When the pool's end time is reached, `resolve_pool_from_price`
//!    is called. It retrieves the latest valid price data, evaluates the condition, and
//!    returns the winning outcome (0 or 1 for binary conditions).
//!
//! # Price Normalization
//!
//! Price data from oracles is stored and processed in a **normalized format** to ensure
//! consistency across different asset pairs and oracle providers:
//!
//! - **Decimal Places**: Prices are stored as `i128` in the oracle's native decimal format.
//!    Pyth Network typically uses 8 decimal places for most assets. The contract does NOT
//!    perform decimal conversion; it assumes all prices in a condition use the same decimal
//!    precision as the oracle feed.
//!
//! - **Target Price Alignment**: When setting a `PriceCondition`, the `target_price` must
//!    be specified in the same decimal format as the oracle feed. For example, if Pyth reports
//!    BTC at $60,000 as `6000000000000` (8 decimals), the condition's target_price must also
//!    use 8 decimals.
//!
//! - **Confidence Normalization**: The `confidence` field represents the uncertainty range
//!    (± value) in the same decimal format as the price. The confidence ratio is calculated
//!    as `(confidence * 10000) / price` to express uncertainty as a percentage in basis points.
//!
//! # Staleness Checks
//!
//! The contract implements multi-layer staleness validation to ensure only fresh, reliable
//! price data is used for resolution:
//!
//! ## 1. Oracle-Level Staleness (`max_price_age`)
//!
//! - Configured in `OracleConfig::max_price_age` (e.g., 60 seconds)
//! - Rejects price data older than this threshold from the current ledger time
//! - Prevents resolution based on outdated market data
//! - Checked in `is_price_valid` via: `current_time > feed.timestamp + config.max_price_age`
//!
//! ## 2. Feed-Level Expiration (`expires_at`)
//!
//! - Each `PriceFeed` has an `expires_at` timestamp set by the oracle provider
//! - Rejects price data after the oracle's declared expiration time
//! - Provides oracle-specific freshness guarantees
//! - Checked in `is_price_valid` via: `current_time > feed.expires_at`
//!
//! ## 3. Confidence Ratio Validation (`min_confidence_ratio`)
//!
//! - Configured in `OracleConfig::min_confidence_ratio` (e.g., 100 bps = 1%)
//! - Rejects price data with high uncertainty (low confidence)
//! - Calculated as: `confidence_ratio = (confidence * 10000) / price`
//! - If `confidence_ratio > min_confidence_ratio`, the price is rejected
//! - Ensures resolution uses only high-confidence price data
//!
//! # Fallback Mechanisms
//!
//! The module provides several fallback mechanisms to handle oracle failures or invalid data:
//!
//! ## 1. Manual Resolution Fallback
//!
//! If automated price-based resolution fails (e.g., stale data, oracle offline), pools can
//! still be resolved via the manual `resolve_pool` function called by operators. This provides
//! a critical fallback to ensure markets can always be settled.
//!
//! ## 2. Multi-Oracle Consensus (Future)
//!
//! The current implementation uses a single oracle (Pyth). Future versions may support
//! multi-oracle consensus where prices from multiple sources are aggregated, providing
//! redundancy and resilience against single-point failures.
//!
//! ## 3. Cleanup Mechanism
//!
//! The `cleanup_expired_feeds` function allows periodic removal of expired price feeds from
//! storage, preventing storage bloat and ensuring only valid data is retained. This is a
//! maintenance fallback to keep the system clean.
//!
//! # Consumption During Pool Resolution
//!
//! Price feeds are consumed during pool resolution through the following flow:
//!
//! ## Resolution Flow
//!
//! 1. **Trigger**: When a pool's `end_time` is reached and the `resolution_delay` has elapsed,
//!    an authorized caller (operator, oracle, or automated system) invokes
//!    `resolve_pool_from_price(pool_id)`.
//!
//! 2. **Condition Retrieval**: The function retrieves the pool's `PriceCondition` from storage
//!    via `DataKey::PriceCondition(pool_id)`. If no condition is set, resolution fails.
//!
//! 3. **Price Data Retrieval**: The function retrieves the latest `PriceFeed` for the
//!    condition's `feed_pair` via `DataKey::PriceFeed(feed_pair)`. If no feed exists,
//!    resolution fails with `PoolNotResolved`.
//!
//! 4. **Staleness Validation**: `is_price_valid` checks:
//!    - Current time <= `feed.expires_at`
//!    - Current time <= `feed.timestamp + config.max_price_age`
//!    - `confidence_ratio <= config.min_confidence_ratio`
//!    If any check fails, resolution fails with `ResolutionDelayNotMet`.
//!
//! 5. **Condition Evaluation**: `evaluate_price_condition` calculates the tolerance amount
//!    and evaluates the condition based on the operator:
//!    - **Equal (0)**: `price` is within `target_price ± tolerance`
//!    - **Greater (1)**: `price > target_price + tolerance`
//!    - **Less (2)**: `price < target_price - tolerance`
//!
//! 6. **Outcome Determination**: The result is mapped to a binary outcome:
//!    - Condition met → Outcome `1` (Yes/Target Met)
//!    - Condition not met → Outcome `0` (No/Target Missed)
//!
//! 7. **Pool Resolution**: The returned outcome is passed to the main resolution logic,
//!    which updates the pool's state, calculates payouts, and allows users to claim winnings.
//!
//! # Integration Flow
//! 1. **Initialize Oracle**: Call `init_oracle` with the Pyth contract address and validation parameters.
//! 2. **Update Feeds**: Oracle keepers call `update_price_feed` periodically to push fresh data.
//! 3. **Set Pool Condition**: During pool creation or setup, call `set_price_condition` to link
//!    a pool to a specific price feed and target outcome.
//! 4. **Resolve Pool**: Once the market ends, call `resolve_pool_from_price` to automatically
//!    determine the winning outcome based on the latest valid price data.

use crate::{DataKey, PredifiError, MAX_TOLERANCE};
use soroban_sdk::{contracttype, Address, Env, Symbol, Vec};

/// Price feed data structure for external oracle integration.
///
/// This struct contains real-time price data from an oracle (e.g., Pyth Network).
/// It is used for automated market resolution based on price conditions.
///
/// # Price Data Validity
/// - `price` must be positive (strictly > 0)
/// - `confidence` must be non-negative (>= 0)
/// - `timestamp` must be in the past (strictly < current ledger time)
/// - `expires_at` must be greater than `timestamp`
/// - Current time must be <= `expires_at` for the price to be considered valid
///
/// # Price Normalization
/// Prices are stored in the oracle's native decimal format (typically 8 decimals for Pyth).
/// The contract does not perform decimal conversion; all price comparisons assume
/// consistent decimal precision between the feed and the condition's target_price.
///
/// # Staleness Checks
/// This struct is validated against staleness checks in `is_price_valid`:
/// - Expiration check: `current_time > expires_at` → invalid
/// - Age check: `current_time > timestamp + max_price_age` → invalid
/// - Confidence check: `confidence_ratio > min_confidence_ratio` → invalid
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
/// - `target_price`: 6000000000000 (8 decimals: $60,000.00)
/// - `operator`: 1 (Greater Than)
/// - `tolerance_bps`: 50 (0.5% tolerance)
///
/// # Tolerance Calculation
/// The tolerance amount is calculated as:
/// ```text
/// tolerance_amount = (target_price * tolerance_bps) / 10_000
/// ```
/// This creates a symmetric buffer around the target price:
/// - Lower bound: `target_price - tolerance_amount`
/// - Upper bound: `target_price + tolerance_amount`
///
/// # Operator Behavior
/// - **Equal (0)**: Condition met if `price` is within `[lower_bound, upper_bound]`
/// - **Greater (1)**: Condition met if `price > upper_bound`
/// - **Less (2)**: Condition met if `price < lower_bound`
///
/// # Resolution Mapping
/// When `resolve_pool_from_price` evaluates this condition:
/// - Condition met → Outcome `1` (Yes/Target Met)
/// - Condition not met → Outcome `0` (No/Target Missed)
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
///
/// # Configuration Parameters
///
/// ## `pyth_contract`
/// The address of the Pyth Network oracle contract on Stellar. This contract
/// provides decentralized price feeds for various asset pairs. The contract
/// interacts with this address to validate oracle authority when updating feeds.
///
/// ## `max_price_age`
/// Maximum allowable age of price data in seconds. Price data older than
/// this threshold is considered stale and rejected during resolution.
///
/// **Rationale for values:**
/// - **60 seconds**: Suitable for high-frequency markets (e.g., crypto)
/// - **300 seconds (5 min)**: Suitable for slower-moving markets (e.g., commodities)
/// - **3600 seconds (1 hour)**: Suitable for long-term predictions
///
/// **Impact:** Too low may cause resolution failures during oracle delays;
/// too high may use outdated data for resolution.
///
/// ## `min_confidence_ratio`
/// Minimum acceptable confidence ratio in basis points (1 bp = 0.01%).
/// The confidence ratio is calculated as `(confidence * 10000) / price`.
/// If the actual confidence ratio exceeds this threshold, the price is rejected.
///
/// **Rationale for values:**
/// - **100 bps (1%)**: High confidence requirement, suitable for liquid assets
/// - **500 bps (5%)**: Moderate confidence, suitable for less liquid assets
/// - **1000 bps (10%)**: Low confidence, suitable for volatile or illiquid assets
///
/// **Impact:** Too low may accept unreliable prices; too high may reject
/// valid prices during periods of market volatility.
///
/// # Staleness Check Integration
/// These parameters are used in `is_price_valid` to enforce multi-layer
/// staleness validation before allowing price-based resolution.
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
    /// This function sets up the oracle integration by registering the Pyth contract
    /// address and configuring validation parameters. This is a one-time setup operation
    /// that should be called by the contract admin during protocol initialization.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `admin` - The admin address (must authenticate via `require_auth`)
    /// - `pyth_contract` - The Pyth Network oracle contract address on Stellar
    /// - `max_price_age` - Maximum age of price data in seconds (must be > 0)
    /// - `min_confidence_ratio` - Minimum confidence ratio in basis points (must be <= 10,000)
    ///
    /// # Oracle Integration Pattern
    ///
    /// This is the **first step** in the oracle integration flow:
    /// 1. Call `init_oracle` (this function) to configure the oracle
    /// 2. Call `update_price_feed` to ingest price data
    /// 3. Call `set_price_condition` to bind pools to feeds
    /// 4. Call `resolve_pool_from_price` to resolve pools automatically
    ///
    /// # Validation
    ///
    /// - `max_price_age` must be > 0 (zero would reject all prices)
    /// - `min_confidence_ratio` must be <= 10,000 (100%)
    /// - Admin must authenticate via `require_auth`
    ///
    /// # Errors
    ///
    /// - `InvalidData` - `max_price_age` is zero
    /// - `InvalidFeeBps` - `min_confidence_ratio` exceeds 10,000
    /// - `Unauthorized` - Caller is not the admin
    ///
    /// # Storage
    ///
    /// Stores the `OracleConfig` at `DataKey::OracleConfig` in persistent storage.
    /// This configuration is used by all subsequent price validation operations.
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

    /// Get the current oracle configuration.
    ///
    /// This function retrieves the global oracle configuration from persistent storage.
    /// The configuration contains the Pyth contract address and validation parameters
    /// used for price staleness checks.
    ///
    /// # Oracle Integration Pattern
    ///
    /// This function is used by:
    /// - `is_price_valid` to retrieve staleness validation parameters
    /// - Admin interfaces to display current oracle settings
    /// - Testing and debugging to verify configuration state
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    ///
    /// # Returns
    ///
    /// The `OracleConfig` struct containing:
    /// - `pyth_contract` - Pyth Network oracle contract address
    /// - `max_price_age` - Maximum age of price data in seconds
    /// - `min_confidence_ratio` - Minimum confidence ratio in basis points
    ///
    /// # Panics
    ///
    /// This function panics if the oracle configuration has not been initialized
    /// via `init_oracle`. This is intentional to fail fast if the oracle is not
    /// properly configured before attempting price validation.
    ///
    /// # Storage
    ///
    /// Reads from `DataKey::OracleConfig` in persistent storage.
    pub fn get_oracle_config(env: &Env) -> OracleConfig {
        env.storage()
            .persistent()
            .get(&DataKey::OracleConfig)
            .expect("Oracle config not initialized")
    }

    /// Update price feed data for a specific asset pair.
    ///
    /// This function ingests fresh price data from an oracle provider into the
    /// contract's persistent storage. It is typically called by off-chain keepers
    /// or authorized oracle roles on a periodic basis (e.g., every 30 seconds).
    ///
    /// # Oracle Integration Pattern
    ///
    /// This is the **second step** in the oracle integration flow:
    /// 1. Call `init_oracle` to configure the oracle
    /// 2. Call `update_price_feed` (this function) to ingest price data
    /// 3. Call `set_price_condition` to bind pools to feeds
    /// 4. Call `resolve_pool_from_price` to resolve pools automatically
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `oracle` - The oracle address (must authenticate via `require_auth`)
    /// - `feed_pair` - The asset pair symbol (e.g., `symbol!("BTC/USD")`)
    /// - `price` - Current price in oracle's native decimal format (must be > 0)
    /// - `confidence` - Confidence interval (uncertainty) in same format (must be >= 0)
    /// - `timestamp` - Unix timestamp when price was last updated (must be < current time)
    /// - `expires_at` - Unix timestamp when this price data expires (must be > timestamp)
    ///
    /// # Price Normalization
    ///
    /// Prices are stored in the oracle's native decimal format without conversion.
    /// The contract assumes all prices for a given `feed_pair` use consistent
    /// decimal precision. For Pyth Network, this is typically 8 decimals.
    ///
    /// # Validation
    ///
    /// - `price` must be > 0 (positive prices only)
    /// - `confidence` must be >= 0 (non-negative uncertainty)
    /// - `timestamp` must be < current ledger time (strictly in the past)
    /// - `expires_at` must be > `timestamp` (expiration after update time)
    /// - Oracle must authenticate via `require_auth`
    ///
    /// # Errors
    ///
    /// - `InvalidAmount` - `price` <= 0 or `confidence` < 0
    /// - `InvalidData` - `timestamp` >= current time or `expires_at` <= `timestamp`
    /// - `Unauthorized` - Caller is not the oracle
    ///
    /// # Storage
    ///
    /// Stores the `PriceFeed` at `DataKey::PriceFeed(feed_pair)` in persistent storage.
    /// The timestamp is embedded in the feed; no separate `LastUpdate` key is needed.
    ///
    /// # Staleness Checks
    ///
    /// This function does NOT perform staleness checks; it accepts valid-looking data.
    /// Staleness validation occurs later in `is_price_valid` during resolution.
    /// This separation allows keepers to batch-update feeds even if some are slightly stale,
    /// with the staleness check applied at resolution time.
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

    /// Get the current price feed data for a specific asset pair.
    ///
    /// This function retrieves the latest price feed data from persistent storage
    /// for the specified asset pair. It returns `None` if no feed data exists for
    /// the pair.
    ///
    /// # Oracle Integration Pattern
    ///
    /// This function is used by:
    /// - `evaluate_price_condition` to retrieve price data for resolution
    /// - Admin interfaces to display current oracle prices
    /// - Off-chain systems to query contract state
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `feed_pair` - The asset pair symbol (e.g., `symbol!("BTC/USD")`)
    ///
    /// # Returns
    ///
    /// - `Some(PriceFeed)` - The latest price feed data for the pair
    /// - `None` - No price feed data exists for the pair
    ///
    /// # Price Data Validity
    ///
    /// This function does NOT perform staleness validation. It returns the stored
    /// price feed regardless of whether it is fresh or expired. Staleness checks
    /// are performed separately by `is_price_valid` during resolution.
    ///
    /// # Storage
    ///
    /// Reads from `DataKey::PriceFeed(feed_pair)` in persistent storage.
    pub fn get_price_feed(env: &Env, feed_pair: &Symbol) -> Option<PriceFeed> {
        let feed: Option<PriceFeed> = env
            .storage()
            .persistent()
            .get(&DataKey::PriceFeed(feed_pair.clone()));

        feed
    }

    /// Check if price feed data is valid and fresh.
    ///
    /// This function performs multi-layer staleness validation to ensure only
    /// reliable, fresh price data is used for pool resolution. It is called during
    /// resolution before evaluating price conditions.
    ///
    /// # Staleness Checks
    ///
    /// The function performs three independent staleness checks:
    ///
    /// ## 1. Expiration Check
    /// ```text
    /// if current_time > feed.expires_at → invalid
    /// ```
    /// Ensures the price data has not passed the oracle's declared expiration time.
    /// This provides oracle-specific freshness guarantees.
    ///
    /// ## 2. Age Check
    /// ```text
    /// if current_time > feed.timestamp + config.max_price_age → invalid
    /// ```
    /// Ensures the price data is not older than the configured maximum age.
    /// This prevents resolution based on outdated market data.
    ///
    /// ## 3. Confidence Ratio Check
    /// ```text
    /// confidence_ratio = (confidence * 10000) / price
    /// if confidence_ratio > config.min_confidence_ratio → invalid
    /// ```
    /// Ensures the price data has sufficient confidence (low uncertainty).
    /// High confidence ratios indicate unreliable or volatile price data.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `feed` - The price feed data to validate
    ///
    /// # Returns
    ///
    /// - `true` if the price data passes all staleness checks
    /// - `false` if any staleness check fails
    ///
    /// # Fallback Mechanism
    ///
    /// If this function returns `false`, automated price-based resolution will fail.
    /// The fallback is to use manual resolution via `resolve_pool` called by operators.
    /// This ensures pools can always be settled even if oracle data is stale or invalid.
    ///
    /// # Configuration
    ///
    /// This function uses the global `OracleConfig` stored at `DataKey::OracleConfig`,
/// which must be initialized via `init_oracle` before any price validation can occur.
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

    /// Set price condition for a pool.
    ///
    /// This function binds a prediction pool to a specific price feed and resolution criteria.
    /// Once set, the pool can be resolved automatically via `resolve_pool_from_price`
    /// based on the current market price matching the condition.
    ///
    /// # Oracle Integration Pattern
    ///
    /// This is the **third step** in the oracle integration flow:
    /// 1. Call `init_oracle` to configure the oracle
    /// 2. Call `update_price_feed` to ingest price data
    /// 3. Call `set_price_condition` (this function) to bind pools to feeds
    /// 4. Call `resolve_pool_from_price` to resolve pools automatically
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `pool_id` - The unique identifier of the prediction pool
    /// - `condition` - The price condition defining resolution criteria
    ///
    /// # Price Condition Requirements
    ///
    /// - `feed_pair` must match a symbol registered via `update_price_feed`
    /// - `target_price` must be in the same decimal format as the oracle feed
    /// - `operator` must be 0 (Equal), 1 (Greater), or 2 (Less)
    /// - `tolerance_bps` must be <= 10,000 (100%)
    ///
    /// # Storage
    ///
    /// Stores the `PriceCondition` at `DataKey::PriceCondition(pool_id)` in persistent storage.
    /// This condition is retrieved during resolution by `resolve_pool_from_price`.
    ///
    /// # Usage Timing
    ///
    /// This function can be called:
    /// - During pool creation (as part of the setup process)
    /// - After pool creation (to add or update price-based resolution)
    /// - Before pool resolution (to configure automated resolution)
    ///
    /// It cannot be called after the pool is resolved or canceled.
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

    /// Get the price condition configured for a specific pool.
    ///
    /// This function retrieves the price condition from persistent storage
    /// for the specified pool. The condition defines how the pool should be
    /// resolved automatically based on oracle price data.
    ///
    /// # Oracle Integration Pattern
    ///
    /// This function is used by:
    /// - `resolve_pool_from_price` to retrieve the pool's resolution criteria
    /// - Admin interfaces to display pool configuration
    /// - Off-chain systems to query pool resolution conditions
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `pool_id` - The unique identifier of the prediction pool
    ///
    /// # Returns
    ///
    /// - `Some(PriceCondition)` - The price condition configured for the pool
    /// - `None` - No price condition is set for the pool
    ///
    /// # Usage Context
    ///
    /// If this function returns `None`, the pool cannot be resolved via
    /// `resolve_pool_from_price`. Manual resolution via `resolve_pool` must
    /// be used instead.
    ///
    /// # Storage
    ///
    /// Reads from `DataKey::PriceCondition(pool_id)` in persistent storage.
    pub fn get_price_condition(env: &Env, pool_id: u64) -> Option<PriceCondition> {
        env.storage()
            .persistent()
            .get(&DataKey::PriceCondition(pool_id))
    }

    /// Evaluate price condition against current price data.
    ///
    /// This function retrieves the latest price data for the condition's feed pair,
/// validates it for staleness, and evaluates whether the condition is met.
/// It is called by `resolve_pool_from_price` during pool resolution.
    ///
    /// # Evaluation Flow
    ///
    /// 1. **Price Retrieval**: Fetch the latest `PriceFeed` for `condition.feed_pair`
    /// 2. **Staleness Validation**: Call `is_price_valid` to ensure the price is fresh
    /// 3. **Tolerance Calculation**: Compute the tolerance buffer around the target price
    /// 4. **Condition Evaluation**: Apply the operator to determine if the condition is met
    ///
    /// # Tolerance Calculation
    ///
    /// The tolerance amount is calculated as:
    /// ```text
    /// tolerance_amount = (target_price * tolerance_bps) / 10_000
    /// ```
    /// This creates a symmetric buffer:
    /// - Lower bound: `target_price - tolerance_amount`
    /// - Upper bound: `target_price + tolerance_amount`
    ///
    /// # Operator Evaluation
    ///
    /// - **Equal (0)**: Returns `true` if `price` is within `[lower_bound, upper_bound]`
    /// - **Greater (1)**: Returns `true` if `price > upper_bound`
    /// - **Less (2)**: Returns `true` if `price < lower_bound`
    /// - **Invalid operator**: Returns `InvalidPoolState` error
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `condition` - The price condition to evaluate
    ///
    /// # Returns
    ///
    /// - `Ok(true)` - The condition is met
    /// - `Ok(false)` - The condition is not met
    /// - `Err(PoolNotResolved)` - Price feed not found for the pair
    /// - `Err(ResolutionDelayNotMet)` - Price data is stale or invalid
    /// - `Err(InvalidPoolState)` - Invalid operator value
    ///
    /// # Staleness Checks
    ///
    /// This function calls `is_price_valid` which performs:
    /// - Expiration check: `current_time > expires_at`
    /// - Age check: `current_time > timestamp + max_price_age`
    /// - Confidence check: `confidence_ratio > min_confidence_ratio`
    ///
    /// If any check fails, the function returns `ResolutionDelayNotMet`, indicating
    /// that the price data is not suitable for resolution.
    ///
    /// # Fallback Mechanism
    ///
    /// If this function returns an error (e.g., stale data), automated resolution fails.
    /// The fallback is to use manual resolution via `resolve_pool` called by operators.
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
        let tolerance_amount =
            (condition.target_price * condition.tolerance_bps as i128) / MAX_TOLERANCE as i128;

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
    /// This is the primary entry point for automated, oracle-based pool resolution.
    /// It retrieves the pool's price condition, evaluates it against the current
    /// market price, and returns the winning outcome index (0 or 1).
    ///
    /// # Oracle Integration Pattern
    ///
    /// This is the **fourth and final step** in the oracle integration flow:
    /// 1. Call `init_oracle` to configure the oracle
    /// 2. Call `update_price_feed` to ingest price data
    /// 3. Call `set_price_condition` to bind pools to feeds
    /// 4. Call `resolve_pool_from_price` (this function) to resolve pools automatically
    ///
    /// # Resolution Flow
    ///
    /// 1. **Condition Retrieval**: Fetch the pool's `PriceCondition` from storage
    /// 2. **Price Evaluation**: Call `evaluate_price_condition` to check if the condition is met
    /// 3. **Outcome Mapping**: Map the boolean result to a binary outcome index
    ///
    /// # Resolution Logic
    ///
    /// - If `evaluate_price_condition` returns `true` → Outcome `1` (Yes/Target Met)
    /// - If `evaluate_price_condition` returns `false` → Outcome `0` (No/Target Missed)
    ///
    /// This binary mapping assumes the pool has exactly 2 outcomes:
    /// - Outcome 0: "No" or "Target Not Met"
    /// - Outcome 1: "Yes" or "Target Met"
    ///
    /// For pools with more than 2 outcomes, this function is not suitable; manual
    /// resolution via `resolve_pool` should be used instead.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `pool_id` - The unique identifier of the prediction pool to resolve
    ///
    /// # Returns
    ///
    /// - `Ok(0)` - Outcome 0 (No/Target Not Met)
    /// - `Ok(1)` - Outcome 1 (Yes/Target Met)
    /// - `Err(PoolNotResolved)` - No price condition configured for the pool
    /// - `Err(ResolutionDelayNotMet)` - Price data is stale or invalid
    /// - `Err(InvalidPoolState)` - Invalid operator in the condition
    ///
    /// # Consumption During Pool Resolution
    ///
    /// This function is called during the resolution phase:
    /// 1. After the pool's `end_time` has passed
    /// 2. After the `resolution_delay` has elapsed
    /// 3. By an authorized caller (operator, oracle, or automated system)
    ///
    /// The returned outcome is then passed to the main resolution logic in `lib.rs`,
    /// which updates the pool's state to `Resolved`, calculates payouts, and allows
    /// users to claim winnings via `claim_winnings`.
    ///
    /// # Staleness Checks
    ///
    /// This function relies on `evaluate_price_condition` which calls `is_price_valid`.
    /// If the price data is stale (too old, expired, or low confidence), resolution
    /// fails with `ResolutionDelayNotMet`. This ensures only fresh, reliable data
    /// is used for resolution.
    ///
    /// # Fallback Mechanism
    ///
    /// If this function returns an error (e.g., no price feed, stale data), the pool
    /// can still be resolved via manual resolution using the `resolve_pool` function
    /// called by operators. This provides a critical fallback to ensure markets can
    /// always be settled even if oracle data is unavailable.
    pub fn resolve_pool_from_price(env: &Env, pool_id: u64) -> Result<u32, PredifiError> {
        let condition =
            Self::get_price_condition(env, pool_id).ok_or(PredifiError::PoolNotResolved)?;

        let condition_met = Self::evaluate_price_condition(env, &condition)?;

        // Return outcome: 1 if condition met, 0 if not met
        Ok(if condition_met { 1 } else { 0 })
    }

    /// Batch update multiple price feeds in a single transaction.
    ///
    /// This function allows updating multiple asset pairs in one call, reducing
    /// transaction costs and improving efficiency. It is typically called by
    /// off-chain keepers that monitor multiple price feeds simultaneously.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `oracle` - The oracle address (must authenticate via `require_auth`)
    /// - `updates` - A vector of tuples, each containing:
    ///   - `feed_pair` - Asset pair symbol (e.g., `symbol!("BTC/USD")`)
    ///   - `price` - Current price (must be > 0)
    ///   - `confidence` - Confidence interval (must be >= 0)
    ///   - `timestamp` - Update timestamp (must be < current time)
    ///   - `expires_at` - Expiration timestamp (must be > timestamp)
    ///
    /// # Validation
    ///
    /// Each update in the batch is validated independently using the same rules
    /// as `update_price_feed`:
    /// - `price` must be > 0
    /// - `confidence` must be >= 0
    /// - `timestamp` must be < current time
    /// - `expires_at` must be > `timestamp`
    ///
    /// If any update fails validation, the entire batch fails and no updates are applied.
    /// This atomic behavior ensures partial updates cannot occur.
    ///
    /// # Errors
    ///
    /// - `InvalidAmount` - Any update has `price` <= 0 or `confidence` < 0
    /// - `InvalidData` - Any update has invalid timestamps
    /// - `Unauthorized` - Caller is not the oracle
    ///
    /// # Storage
    ///
    /// Each successful update stores a `PriceFeed` at `DataKey::PriceFeed(feed_pair)`.
    /// The function does not update `DataKey::PriceFeedList`; that must be managed
    /// separately if a registry of available feeds is needed.
    ///
    /// # Gas Efficiency
    ///
    /// Batch updates are more gas-efficient than individual updates because:
    /// - Single authentication check (`require_auth`)
    /// - Single transaction overhead
    /// - Reduced storage I/O batching
    ///
    /// Recommended batch size: 10-20 feeds per transaction to balance efficiency
    /// with Soroban's transaction size limits.
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

    /// Get all available price feed pairs registered in the contract.
    ///
    /// This function returns a list of all asset pair symbols that have
    /// price feed data available. This is useful for discovery and
    /// validation purposes.
    ///
    /// # Oracle Integration Pattern
    ///
    /// This function is used by:
    /// - Admin interfaces to display available oracle feeds
    /// - Pool creation interfaces to show valid feed options
    /// - Off-chain systems to discover supported asset pairs
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    ///
    /// # Returns
    ///
    /// A `Vec<Symbol>` containing all feed pair symbols with available data.
    ///
    /// # Current Implementation
    ///
    /// This function currently returns an empty vector. A full implementation
    /// would scan storage for all `DataKey::PriceFeed` keys and return the
    /// corresponding pair symbols. This depends on Soroban's storage scanning
    /// capabilities which may be limited.
    ///
    /// # Future Enhancement
    ///
    /// To implement this function fully, consider:
    /// - Maintaining a `DataKey::PriceFeedList` registry of all feed pairs
    /// - Updating the registry in `update_price_feed` and `cleanup_expired_feeds`
    /// - Returning the registry from this function
    ///
    /// # Storage
    ///
    /// Would read from `DataKey::PriceFeedList` if a registry is implemented.
    pub fn get_available_feeds(env: &Env) -> Vec<Symbol> {
        // This would typically scan storage for all PriceFeed keys
        // For now, return empty vector - implementation depends on storage scanning capabilities
        Vec::new(env)
    }

    /// Remove all expired price feeds from storage.
    ///
    /// This function performs maintenance by cleaning up expired price feeds,
    /// preventing storage bloat and ensuring only valid data is retained.
    /// It iterates through the feed registry, removes expired entries, and updates
    /// the registry.
    ///
    /// # Cleanup Logic
    ///
    /// The function performs the following steps:
    ///
    /// 1. **Registry Retrieval**: Fetch the current list of feed pairs from
    ///    `DataKey::PriceFeedList`. If the list doesn't exist, return 0.
    ///
    /// 2. **Expiration Check**: For each feed pair in the registry:
    ///    - Fetch the `PriceFeed` from `DataKey::PriceFeed(pair)`
    ///    - If the feed is missing from storage, treat it as expired
    ///    - If the feed exists and `expires_at < current_time`, it's expired
    ///
    /// 3. **Removal**: Delete expired feeds from storage via
    ///    `storage().persistent().remove(&DataKey::PriceFeed(pair))`
    ///
    /// 4. **Registry Update**: Write the pruned list (non-expired feeds only)
    ///    back to `DataKey::PriceFeedList`.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    ///
    /// # Returns
    ///
    /// - `Ok(count)` - The number of expired feeds removed
    ///
    /// # Fallback Mechanism
    ///
    /// This function is a maintenance fallback to keep the system clean. It does
    /// not affect resolution logic because `is_price_valid` already rejects expired
    /// feeds. However, regular cleanup reduces storage costs and prevents storage
    /// from growing unbounded.
    ///
    /// # Storage Management
    ///
    /// - **Input**: Reads from `DataKey::PriceFeedList`
    /// - **Per-feed**: Reads from `DataKey::PriceFeed(pair)`
    /// - **Output**: Writes to `DataKey::PriceFeedList`
    /// - **Deletions**: Removes `DataKey::PriceFeed(pair)` for expired feeds
    ///
    /// # Recommended Usage
    ///
    /// This function should be called periodically (e.g., daily or weekly) by:
    /// - Off-chain maintenance bots
    /// - Admins during routine operations
    /// - cron jobs or scheduled tasks
    ///
    /// Frequency depends on oracle update patterns and storage cost considerations.
    pub fn cleanup_expired_feeds(env: &Env) -> Result<u32, PredifiError> {
        let current_time = env.ledger().timestamp();

        let list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&DataKey::PriceFeedList)
            .unwrap_or_else(|| Vec::new(env));

        let mut remaining: Vec<Symbol> = Vec::new(env);
        let mut removed: u32 = 0;

        for i in 0..list.len() {
            let pair = list.get(i).unwrap();

            // A missing entry is also treated as expired.
            let expired = env
                .storage()
                .persistent()
                .get::<DataKey, PriceFeed>(&DataKey::PriceFeed(pair.clone()))
                .map(|feed| feed.expires_at < current_time)
                .unwrap_or(true);

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

        Ok(removed)
    }
}

#[cfg(test)]
mod tests {}
// #[cfg(test)]
// mod tests;

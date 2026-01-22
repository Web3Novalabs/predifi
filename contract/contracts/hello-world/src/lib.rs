#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Symbol};

// Storage keys for the contract
#[derive(Clone)]
#[contracttype]
pub enum StorageKey {
    Admin,
    ProtocolFee,        // Fee percentage in basis points (1% = 100)
    Treasury,           // Treasury address for fee collection
    TotalFeesCollected, // Total fees collected
}

// Events for tracking fee operations
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeCollectedEvent {
    pub pool_id: u64,
    pub amount: i128,
    pub fee_type: Symbol,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeDistributedEvent {
    pub treasury: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfigUpdatedEvent {
    pub old_fee: u32,
    pub new_fee: u32,
    pub timestamp: u64,
}

// Pool data structure
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pool {
    pub pool_id: u64,
    pub total_amount: i128,
    pub is_resolved: bool,
    pub creator: Address,
}

#[contract]
pub struct PredictionMarket;

// Constants
const MAX_FEE_BPS: u32 = 1000; // Maximum 10% fee
const DEFAULT_FEE_BPS: u32 = 200; // Default 2% fee
const BPS_DENOMINATOR: i128 = 10000; // Basis points denominator

#[contractimpl]
impl PredictionMarket {
    /// Initialize the contract with admin, treasury, and default fee
    pub fn initialize(env: Env, admin: Address, treasury: Address, protocol_fee_bps: u32) {
        // Ensure contract is not already initialized
        if env.storage().instance().has(&StorageKey::Admin) {
            panic!("Contract already initialized");
        }

        // Validate fee percentage
        if protocol_fee_bps > MAX_FEE_BPS {
            panic!("Fee exceeds maximum allowed");
        }

        // Require admin authentication
        admin.require_auth();

        // Store admin and configuration
        env.storage().instance().set(&StorageKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&StorageKey::Treasury, &treasury);
        env.storage()
            .instance()
            .set(&StorageKey::ProtocolFee, &protocol_fee_bps);
        env.storage()
            .instance()
            .set(&StorageKey::TotalFeesCollected, &0i128);
    }

    /// Update protocol fee percentage (admin only)
    pub fn update_protocol_fee(env: Env, new_fee_bps: u32) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&StorageKey::Admin)
            .expect("Not initialized");
        admin.require_auth();

        if new_fee_bps > MAX_FEE_BPS {
            panic!("Fee exceeds maximum allowed");
        }

        let old_fee: u32 = env
            .storage()
            .instance()
            .get(&StorageKey::ProtocolFee)
            .unwrap_or(DEFAULT_FEE_BPS);

        env.storage()
            .instance()
            .set(&StorageKey::ProtocolFee, &new_fee_bps);

        // Emit event
        env.events().publish(
            (Symbol::new(&env, "fee_updated"),),
            FeeConfigUpdatedEvent {
                old_fee,
                new_fee: new_fee_bps,
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    /// Update treasury address (admin only)
    pub fn update_treasury(env: Env, new_treasury: Address) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&StorageKey::Admin)
            .expect("Not initialized");
        admin.require_auth();

        env.storage()
            .instance()
            .set(&StorageKey::Treasury, &new_treasury);
    }

    /// Calculate fee amount from a given total
    pub fn calculate_fee(env: Env, amount: i128) -> i128 {
        let fee_bps: u32 = env
            .storage()
            .instance()
            .get(&StorageKey::ProtocolFee)
            .unwrap_or(DEFAULT_FEE_BPS);

        // Fee calculation: (amount * fee_bps) / BPS_DENOMINATOR
        (amount * fee_bps as i128) / BPS_DENOMINATOR
    }

    /// Calculate amount after deducting fee
    pub fn calculate_amount_after_fee(env: Env, amount: i128) -> (i128, i128) {
        let fee = Self::calculate_fee(env.clone(), amount);
        let net_amount = amount - fee;
        (net_amount, fee)
    }

    /// Create a pool with fee deduction
    pub fn create_pool(
        env: Env,
        pool_id: u64,
        creator: Address,
        token: Address,
        amount: i128,
    ) -> i128 {
        creator.require_auth();

        // Calculate pool creation fee
        let fee = Self::calculate_fee(env.clone(), amount);
        let net_amount = amount - fee;

        // Transfer the fee to treasury
        let treasury: Address = env
            .storage()
            .instance()
            .get(&StorageKey::Treasury)
            .expect("Treasury not set");

        if fee > 0 {
            let token_client = token::Client::new(&env, &token);
            token_client.transfer(&creator, &treasury, &fee);

            // Update total fees collected
            let total_fees: i128 = env
                .storage()
                .instance()
                .get(&StorageKey::TotalFeesCollected)
                .unwrap_or(0);
            env.storage()
                .instance()
                .set(&StorageKey::TotalFeesCollected, &(total_fees + fee));

            // Emit fee collected event
            env.events().publish(
                (Symbol::new(&env, "fee_collected"),),
                FeeCollectedEvent {
                    pool_id,
                    amount: fee,
                    fee_type: Symbol::new(&env, "pool_creation"),
                    timestamp: env.ledger().timestamp(),
                },
            );

            // Emit fee distributed event
            env.events().publish(
                (Symbol::new(&env, "fee_distributed"),),
                FeeDistributedEvent {
                    treasury: treasury.clone(),
                    amount: fee,
                    timestamp: env.ledger().timestamp(),
                },
            );
        }

        // Return net amount that goes into the pool
        net_amount
    }

    /// Distribute winnings with fee deduction
    pub fn distribute_winnings(
        env: Env,
        pool_id: u64,
        token: Address,
        winner: Address,
        total_winnings: i128,
    ) -> i128 {
        // Calculate fee on winnings
        let (net_winnings, fee) = Self::calculate_amount_after_fee(env.clone(), total_winnings);

        // Transfer fee to treasury
        let treasury: Address = env
            .storage()
            .instance()
            .get(&StorageKey::Treasury)
            .expect("Treasury not set");

        if fee > 0 {
            let token_client = token::Client::new(&env, &token);

            // Transfer fee to treasury
            token_client.transfer(&env.current_contract_address(), &treasury, &fee);

            // Update total fees collected
            let total_fees: i128 = env
                .storage()
                .instance()
                .get(&StorageKey::TotalFeesCollected)
                .unwrap_or(0);
            env.storage()
                .instance()
                .set(&StorageKey::TotalFeesCollected, &(total_fees + fee));

            // Emit fee collected event
            env.events().publish(
                (Symbol::new(&env, "fee_collected"),),
                FeeCollectedEvent {
                    pool_id,
                    amount: fee,
                    fee_type: Symbol::new(&env, "winnings_distribution"),
                    timestamp: env.ledger().timestamp(),
                },
            );

            // Emit fee distributed event
            env.events().publish(
                (Symbol::new(&env, "fee_distributed"),),
                FeeDistributedEvent {
                    treasury: treasury.clone(),
                    amount: fee,
                    timestamp: env.ledger().timestamp(),
                },
            );
        }

        // Transfer net winnings to winner
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &winner, &net_winnings);

        net_winnings
    }

    /// Resolve pool with fee calculation
    pub fn resolve_pool(
        env: Env,
        pool_id: u64,
        token: Address,
        total_pool_amount: i128,
    ) -> (i128, i128) {
        // Calculate resolution fee
        let (net_amount, fee) = Self::calculate_amount_after_fee(env.clone(), total_pool_amount);

        // Transfer fee to treasury
        let treasury: Address = env
            .storage()
            .instance()
            .get(&StorageKey::Treasury)
            .expect("Treasury not set");

        if fee > 0 {
            let token_client = token::Client::new(&env, &token);
            token_client.transfer(&env.current_contract_address(), &treasury, &fee);

            // Update total fees collected
            let total_fees: i128 = env
                .storage()
                .instance()
                .get(&StorageKey::TotalFeesCollected)
                .unwrap_or(0);
            env.storage()
                .instance()
                .set(&StorageKey::TotalFeesCollected, &(total_fees + fee));

            // Emit fee collected event
            env.events().publish(
                (Symbol::new(&env, "fee_collected"),),
                FeeCollectedEvent {
                    pool_id,
                    amount: fee,
                    fee_type: Symbol::new(&env, "pool_resolution"),
                    timestamp: env.ledger().timestamp(),
                },
            );

            // Emit fee distributed event
            env.events().publish(
                (Symbol::new(&env, "fee_distributed"),),
                FeeDistributedEvent {
                    treasury: treasury.clone(),
                    amount: fee,
                    timestamp: env.ledger().timestamp(),
                },
            );
        }

        // Return (net_amount for distribution, fee collected)
        (net_amount, fee)
    }

    /// Get current protocol fee in basis points
    pub fn get_protocol_fee(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&StorageKey::ProtocolFee)
            .unwrap_or(DEFAULT_FEE_BPS)
    }

    /// Get treasury address
    pub fn get_treasury(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&StorageKey::Treasury)
            .expect("Treasury not set")
    }

    /// Get total fees collected
    pub fn get_total_fees_collected(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&StorageKey::TotalFeesCollected)
            .unwrap_or(0)
    }

    /// Get maximum allowed fee
    pub fn get_max_fee(_env: Env) -> u32 {
        MAX_FEE_BPS
    }
}

mod test;

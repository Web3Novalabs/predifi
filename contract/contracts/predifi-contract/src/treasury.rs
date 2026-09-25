//! # Treasury Domain (#1653)
//!
//! Manages the protocol treasury: the configured recipient address for protocol
//! fees, and the admin-gated functions that move funds out of the contract.
//!
//! ## What the Treasury Holds
//!
//! The treasury does not hold funds directly on-chain — it is simply an [`Address`]
//! stored in [`crate::Config::treasury`] (set at `initialize` and updatable via
//! [`PredifiContract::set_treasury`]). The actual token balances that back the
//! protocol's fee revenue accumulate in the contract's own balance: when a pool
//! is created or a user claims winnings, the protocol fee (`Config.fee_bps`, or
//! a matching fee tier) is deducted from the total stake/payout pool and simply
//! left in the contract rather than transferred out immediately.
//!
//! ## How Fees Flow Into the Treasury
//!
//! Fee accrual and fee withdrawal are two separate steps:
//! 1. **Accrual (automatic)** — `create_pool` and `claim_winnings` compute the
//!    protocol fee via [`crate::safe_math::SafeMath::percentage`] and simply
//!    don't pay that portion out, so it remains part of the contract's token
//!    balance.
//! 2. **Withdrawal (manual, admin-only)** — [`PredifiContract::withdraw_treasury`]
//!    is the only function that actually moves those accrued fees out of the
//!    contract, transferring them to a `recipient` address (typically the
//!    configured treasury address, though the function accepts any recipient).
//!    Nothing is pushed to the treasury automatically; an admin must call
//!    `withdraw_treasury` to sweep it.
//!
//! ## Who Is Authorised
//!
//! Every function in this module requires the caller to hold the **Admin role
//! (role 0)** in the access control contract, verified via
//! [`PredifiContract::require_admin_role`], plus `admin.require_auth()`:
//! - [`PredifiContract::set_treasury`] — repoints the treasury address.
//! - [`PredifiContract::withdraw_treasury`] — withdraws accrued protocol fees
//!   (or any unused liquidity) to a recipient; enforces
//!   [`crate::MIN_WITHDRAWAL_AMOUNT`] and a sufficient contract balance.
//! - [`PredifiContract::emergency_withdraw`] — an escape hatch for rescuing any
//!   token balance held by the contract (e.g. after an oracle or protocol
//!   failure), bypassing the pause check so funds can still be recovered while
//!   the contract is paused.
//!
//! All three emit an audit event ([`crate::TreasuryUpdateEvent`],
//! [`crate::TreasuryWithdrawnEvent`], [`crate::EmergencyWithdrawEvent`]) and are
//! reentrancy-guarded around the token transfer.

use soroban_sdk::{contractimpl, token, Address, Env};

use crate::{
    DataKey, EmergencyWithdrawEvent, PredifiContract, PredifiContractArgs, PredifiContractClient,
    PredifiError, TreasuryUpdateEvent, TreasuryWithdrawnEvent, MIN_WITHDRAWAL_AMOUNT,
};

#[contractimpl]
impl PredifiContract {
    /// Set treasury address. Caller must have Admin role (0).
    pub fn set_treasury(env: Env, admin: Address, treasury: Address) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "set_treasury")?;
        let mut config = Self::get_config(&env);
        config.treasury = treasury.clone();
        env.storage().instance().set(&DataKey::Config, &config);
        Self::extend_instance(&env);

        TreasuryUpdateEvent { admin, treasury }.publish(&env);
        Ok(())
    }

    /// Withdraw accumulated protocol fees or unused liquidity from the contract.
    /// Only callable by Admin (role 0).
    ///
    /// # Arguments
    /// * `admin` - Address with Admin role (must provide auth)
    /// * `token` - The token contract address to withdraw
    /// * `amount` - Amount to withdraw (must be > 0)
    /// * `recipient` - Address to receive the withdrawn funds (typically treasury)
    ///
    /// # Returns
    /// Result indicating success or error
    ///
    /// # Security
    /// - Requires Admin role (0)
    /// - Emits TreasuryWithdrawnEvent for audit trail
    /// - Validates amount >= MIN_WITHDRAWAL_AMOUNT
    /// - Checks contract has sufficient balance
    pub fn withdraw_treasury(
        env: Env,
        admin: Address,
        token: Address,
        amount: i128,
        recipient: Address,
    ) -> Result<(), PredifiError> {
        Self::require_not_paused(&env)?;
        admin.require_auth();

        // Verify admin role
        Self::require_admin_role(&env, &admin, "withdraw_treasury")?;

        // Reject zero or negative withdrawals before touching token state.
        if amount <= 0 || amount < MIN_WITHDRAWAL_AMOUNT {
            return Err(PredifiError::InvalidAmount);
        }

        // Get token client and check the contract's available balance first.
        let token_client = token::Client::new(&env, &token);
        let available_balance = token_client.balance(&env.current_contract_address());

        // Verify sufficient balance
        if available_balance < amount {
            return Err(PredifiError::InsufficientBalance);
        }

        Self::enter_reentrancy_guard(&env);

        // Validate token transfer before withdrawal
        Self::validate_token_transfer(
            &env,
            &token,
            &env.current_contract_address(),
            &recipient,
            amount,
        )?;

        // Transfer tokens to recipient
        token_client.transfer(&env.current_contract_address(), &recipient, &amount);

        // Compute remaining balance after transfer for the audit event
        let remaining_balance = token_client.balance(&env.current_contract_address());

        Self::exit_reentrancy_guard(&env);

        // Emit audit event
        TreasuryWithdrawnEvent {
            admin: admin.clone(),
            token: token.clone(),
            amount,
            recipient: recipient.clone(),
            remaining_balance,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(())
    }

    /// Emergency escape hatch: transfers any token balance held by this contract
    /// to a destination address. Restricted to the admin role.
    ///
    /// Intended for use when the protocol or oracle has failed and funds must be
    /// rescued. Emits an `EmergencyWithdraw` event for on-chain auditability.
    pub fn emergency_withdraw(
        env: Env,
        admin: Address,
        token: Address,
        destination: Address,
        amount: i128,
    ) -> Result<(), PredifiError> {
        admin.require_auth();
        Self::require_admin_role(&env, &admin, "emergency_withdraw")?;

        // Validate token transfer before execution
        Self::validate_token_transfer(
            &env,
            &token,
            &env.current_contract_address(),
            &destination,
            amount,
        )?;

        let token_client = token::Client::new(&env, &token);

        Self::enter_reentrancy_guard(&env);
        token_client.transfer(&env.current_contract_address(), &destination, &amount);
        Self::exit_reentrancy_guard(&env);

        EmergencyWithdrawEvent {
            admin,
            token,
            destination,
            amount,
        }
        .publish(&env);

        Ok(())
    }
}

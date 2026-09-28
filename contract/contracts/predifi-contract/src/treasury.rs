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
//! ## Withdrawal Authority
//!
//! Every withdrawal function in this module is gated by the **Admin role (role 0)**
//! in the companion access-control contract. The caller must both supply a valid
//! Soroban authentication signature (`require_auth()`) and hold role 0
//! (`require_admin_role`). The Admin is the sole authority permitted to:
//! - [`PredifiContract::set_treasury`] — repoint the treasury address.
//! - [`PredifiContract::withdraw_treasury`] — withdraw accrued protocol fees
//!   (or any unused liquidity) to a recipient.
//! - [`PredifiContract::emergency_withdraw`] — rescue any token balance held by
//!   the contract (e.g. after an oracle or protocol failure), bypassing the pause
//!   check so funds can still be recovered while the contract is paused.
//!
//! All three functions emit an audit event ([`crate::TreasuryUpdateEvent`],
//! [`crate::TreasuryWithdrawnEvent`], [`crate::EmergencyWithdrawEvent`]) and are
//! reentrancy-guarded around the token transfer.
//!
//! ## Withdrawal Limits
//!
//! | Limit type        | Value / behaviour                                                        |
//! |-------------------|--------------------------------------------------------------------------|
//! | **Per-call minimum** | [`crate::MIN_WITHDRAWAL_AMOUNT`] (= 1 in base token units / stroops).  |
//! |                     | Withdrawals of zero or negative amount are rejected with `InvalidAmount`. |
//! | **Per-call maximum** | The contract's current token balance — any request exceeding the       |
//! |                     | available balance is rejected with `InsufficientBalance`.                |
//! | **Per-period limit** | **None.** An admin may invoke `withdraw_treasury` any number of times   |
//! |                     | within a ledger close; there is no cooldown, rate-limit, or cap on the  |
//! |                     | number of withdrawals per epoch or day.                                  |
//!
//! ## Unavailable Authority
//!
//! The contract does **not** implement a multi-signature scheme, a timelock,
//! or a recovery mechanism that bypasses the Admin role. If the Admin private
//! key is lost or the Admin is otherwise unavailable:
//!
//! - Funds that have already been withdrawn to the treasury address remain
//!   accessible to whoever controls that address.
//! - Funds still sitting in the contract's token balance **cannot be recovered**
//!   by anyone — `withdraw_treasury` and `emergency_withdraw` both require the
//!   Admin role, and there is no fallback path.
//! - The contract's paused state (set via `pause`) does not help here:
//!   `emergency_withdraw` bypasses the pause check but still requires the Admin
//!   role.
//!
//! Operators should secure the Admin key (e.g. multi-sig off-chain, hardware
//! wallet, or a well-rehearsed key-recovery process) because the contract
//! provides no on-chain mechanism to replace or recover the Admin.

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

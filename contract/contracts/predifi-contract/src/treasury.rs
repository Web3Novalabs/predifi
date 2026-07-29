//! Treasury domain: treasury configuration, fee withdrawal and emergency
//! fund recovery.

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

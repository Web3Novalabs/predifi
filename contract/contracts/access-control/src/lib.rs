#![no_std]

// ═══════════════════════════════════════════════════════════════════════════
// ACCESS CONTROL — ROLES & PERMISSIONS
// ═══════════════════════════════════════════════════════════════════════════
//
// This contract provides role-based access control (RBAC) for the PrediFi
// prediction market protocol. Roles are hierarchical with Admin having the
// highest privileges.
//
// ┌──────────┬───────┬──────────────────────────────────────────────────────┐
// │ Role     │ Value │ Description                                          │
// ├──────────┼───────┼──────────────────────────────────────────────────────┤
// │ Admin    │   0   │ Full control over protocol configuration and role    │
// │          │       │ management. Can assign/revoke roles, transfer      │
// │          │       │ admin rights, and manage all contract settings.    │
// ├──────────┼───────┼──────────────────────────────────────────────────────┤
// │ Operator │   1   │ Can manage prediction pools, perform operational    │
// │          │       │ tasks such as resolving pools and updating config.  │
// ├──────────┼───────┼──────────────────────────────────────────────────────┤
// │ Moderator│   2   │ Can handle disputes and moderate content.            │
// ├──────────┼───────┼──────────────────────────────────────────────────────┤
// │ Oracle   │   3   │ Can resolve pools based on external data and        │
// │          │       │ price feeds. Provides multi-vote resolution system. │
// ├──────────┼───────┼──────────────────────────────────────────────────────┤
// │ User     │   4   │ Basic role for regular protocol participants.       │
// └──────────┴───────┴──────────────────────────────────────────────────────┘
//
// NOTE: The predifi-contract uses role checks to enforce permissions:
// - Role 0 (Admin): pause/unpause, set_fee_bps, set_treasury, set_resolution_delay,
//   set_referral_cut_bps, add_token_to_whitelist, remove_token_from_whitelist,
//   withdraw_treasury, upgrade_contract, migrate_state
// - Role 1 (Operator): resolve_pool, cancel_pool, set_stake_limits
// - Role 3 (Oracle): oracle_resolve (OracleCallback trait)
//
// ROLES ARE ASSIGNED
// ───────────────────
// 1. Deploy this access-control contract and call `init(admin)` to set the
//    initial administrator. The admin role (0) is automatically assigned.
// 2. The admin calls `assign_role(admin_caller, user, Role::Operator)` (or any
//    other role) to grant a role to any address.
// 3. Roles can be managed via:
//    - `revoke_role`: Remove a specific role from a user
//    - `transfer_role`: Move a role from one user to another
//    - `revoke_all_roles`: Remove all roles from a user
//    - `propose_new_admin` + `accept_admin_role`: Two-step admin transfer
//    - `transfer_admin`: Legacy one-step admin transfer
// 4. Any contract can check if a user has a role by calling `has_role(user, role)`.
// 5. The `has_any_role` function allows checking if a user has any of a set of roles.
//
// SECURITY CONSIDERATIONS
// ───────────────────────
// - Only the admin can assign or revoke roles
// - All admin operations require authentication (`require_auth`)
// - Admin transfer supports a two-step propose/accept flow for safety
// - Role checks are performed via storage lookups in persistent storage
//
// ═══════════════════════════════════════════════════════════════════════════

use predifi_errors::PrediFiError;
use soroban_sdk::{contract, contractevent, contractimpl, contracttype, Address, Env};

/// Role-based access control enumeration.
///
/// This enum defines all available roles in the protocol.
/// Roles are hierarchical with Admin having the highest privileges.
///
/// # Role Hierarchy
/// - Admin (0): Full control over protocol configuration and role management
/// - Operator (1): Can manage pools and perform operational tasks
/// - Moderator (2): Can moderate content and handle disputes
/// - Oracle (3): Can resolve pools and provide external data
/// - User (4): Basic role for regular users (often implicit)
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Role {
    /// Administrator with full control over protocol settings and role assignments.
    Admin = 0,
    /// Operator can manage pools, update configurations, and perform operational tasks.
    Operator = 1,
    /// Moderator can handle disputes and moderate content.
    Moderator = 2,
    /// Oracle can resolve pools based on external data and price feeds.
    Oracle = 3,
    /// Basic user role for regular protocol participants.
    User = 4,
}

#[contractevent(topics = ["admin_init"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminInitEvent {
    pub admin: Address,
}

#[contractevent(topics = ["role_assigned"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleAssignedEvent {
    pub admin: Address,
    pub user: Address,
    pub role: Role,
}

#[contractevent(topics = ["role_revoked"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleRevokedEvent {
    pub admin: Address,
    pub user: Address,
    pub role: Role,
}

#[contractevent(topics = ["role_transferred"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleTransferredEvent {
    pub admin: Address,
    pub from: Address,
    pub to: Address,
    pub role: Role,
}

#[contractevent(topics = ["admin_transferred"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminTransferredEvent {
    pub admin: Address,
    pub new_admin: Address,
}

#[contractevent(topics = ["admin_transfer_proposed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminTransferProposedEvent {
    pub admin: Address,
    pub proposed_admin: Address,
}

#[contractevent(topics = ["all_roles_revoked"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AllRolesRevokedEvent {
    pub admin: Address,
    pub user: Address,
}

/// Status of a prediction pool in the access control system.
///
/// This enum tracks the lifecycle state of a pool for permission management.
/// Different roles may have different permissions based on pool status.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoolStatus {
    /// Pool is active and accepting operations.
    Active,
    /// Pool has been resolved and is in payout phase.
    Resolved,
    /// Pool is closed and no longer accepting operations.
    Closed,
    /// Pool is under dispute and requires moderator intervention.
    Disputed,
}

/// Category classification for prediction pools.
///
/// This enum provides a standardized set of categories for organizing
/// prediction markets. Categories help users discover relevant pools.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoolCategory {
    /// Sports-related predictions (e.g., game outcomes, tournaments, player performance).
    Sports,
    /// Political event predictions (e.g., elections, policy decisions, approvals).
    Politics,
    /// Financial market predictions (e.g., stock prices, indices, economic indicators).
    Finance,
    /// Entertainment industry predictions (e.g., awards, box office, TV shows).
    Entertainment,
    /// Miscellaneous predictions that don't fit other categories.
    Other,
}

/// Storage keys for access control data.
///
/// This enum defines all storage keys used by the access control contract.
#[contracttype]
pub enum DataKey {
    /// Admin address: Admin -> Address
    Admin,
    /// Proposed admin address awaiting acceptance: ProposedAdmin -> Address
    ProposedAdmin,
    /// Role assignment: Role(user_address, role) -> ()
    Role(Address, Role),
    /// Pool data: Pool(pool_id) -> Pool
    Pool(u64),
    /// Pool counter: PoolCount -> u64
    PoolCount,
    /// Count of addresses currently holding the Operator role: OperatorCount -> u32
    OperatorCount,
}

#[contract]
pub struct AccessControl;

#[contractimpl]
impl AccessControl {
    pub fn init(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            soroban_sdk::panic_with_error!(&env, PrediFiError::AlreadyInitializedOrConfigNotSet);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&DataKey::Role(admin.clone(), Role::Admin), &());

        AdminInitEvent { admin }.publish(&env);
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("NotInitialized")
    }

    pub fn get_proposed_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::ProposedAdmin)
    }

    pub fn assign_role(
        env: Env,
        admin_caller: Address,
        user: Address,
        role: Role,
    ) -> Result<(), PrediFiError> {
        admin_caller.require_auth();

        let current_admin = Self::get_admin(env.clone());
        if admin_caller != current_admin {
            return Err(PrediFiError::Unauthorized);
        }

        // Only increment operator count if the role is newly assigned
        let role_key = DataKey::Role(user.clone(), role.clone());
        let is_new = !env.storage().persistent().has(&role_key);

        env.storage().persistent().set(&role_key, &());

        if matches!(role, Role::Operator) && is_new {
            let count: u32 = env
                .storage()
                .instance()
                .get(&DataKey::OperatorCount)
                .unwrap_or(0);
            env.storage()
                .instance()
                .set(&DataKey::OperatorCount, &(count + 1));
        }

        RoleAssignedEvent {
            admin: admin_caller,
            user,
            role,
        }
        .publish(&env);
        Ok(())
    }

    pub fn revoke_role(
        env: Env,
        admin_caller: Address,
        user: Address,
        role: Role,
    ) -> Result<(), PrediFiError> {
        admin_caller.require_auth();

        let current_admin = Self::get_admin(env.clone());
        if admin_caller != current_admin {
            return Err(PrediFiError::Unauthorized);
        }

        if !env
            .storage()
            .persistent()
            .has(&DataKey::Role(user.clone(), role.clone()))
        {
            return Err(PrediFiError::InsufficientPermissions);
        }

        env.storage()
            .persistent()
            .remove(&DataKey::Role(user.clone(), role.clone()));

        if matches!(role, Role::Operator) {
            let count: u32 = env
                .storage()
                .instance()
                .get(&DataKey::OperatorCount)
                .unwrap_or(0);
            if count > 0 {
                env.storage()
                    .instance()
                    .set(&DataKey::OperatorCount, &(count - 1));
            }
        }

        RoleRevokedEvent {
            admin: admin_caller,
            user,
            role,
        }
        .publish(&env);
        Ok(())
    }

    pub fn has_role(env: Env, user: Address, role: Role) -> bool {
        env.storage().persistent().has(&DataKey::Role(user, role))
    }

    pub fn transfer_role(
        env: Env,
        admin_caller: Address,
        from: Address,
        to: Address,
        role: Role,
    ) -> Result<(), PrediFiError> {
        admin_caller.require_auth();

        let current_admin = Self::get_admin(env.clone());
        if admin_caller != current_admin {
            return Err(PrediFiError::Unauthorized);
        }

        if !env
            .storage()
            .persistent()
            .has(&DataKey::Role(from.clone(), role.clone()))
        {
            return Err(PrediFiError::InsufficientPermissions);
        }

        env.storage()
            .persistent()
            .remove(&DataKey::Role(from.clone(), role.clone()));

        // Only increment if `to` doesn't already hold the role
        let to_key = DataKey::Role(to.clone(), role.clone());
        let to_is_new = !env.storage().persistent().has(&to_key);
        env.storage().persistent().set(&to_key, &());

        // Operator count: removed from `from` (always), added to `to` only if new
        if matches!(role, Role::Operator) {
            let count: u32 = env
                .storage()
                .instance()
                .get(&DataKey::OperatorCount)
                .unwrap_or(0);
            // `from` lost the role; `to` gains it only if they didn't already have it
            // Net change: -1 (from) + (1 if to_is_new else 0)
            let new_count = if to_is_new {
                count
            } else {
                count.saturating_sub(1)
            };
            env.storage()
                .instance()
                .set(&DataKey::OperatorCount, &new_count);
        }

        RoleTransferredEvent {
            admin: admin_caller,
            from,
            to,
            role,
        }
        .publish(&env);
        Ok(())
    }

    pub fn transfer_admin(
        env: Env,
        admin_caller: Address,
        new_admin: Address,
    ) -> Result<(), PrediFiError> {
        admin_caller.require_auth();

        let current_admin = Self::get_admin(env.clone());
        if admin_caller != current_admin {
            return Err(PrediFiError::Unauthorized);
        }

        Self::apply_admin_transfer(&env, current_admin.clone(), new_admin.clone());

        AdminTransferredEvent {
            admin: current_admin,
            new_admin,
        }
        .publish(&env);

        Ok(())
    }

    pub fn propose_new_admin(
        env: Env,
        current_admin: Address,
        new_admin: Address,
    ) -> Result<(), PrediFiError> {
        current_admin.require_auth();

        let stored_admin = Self::get_admin(env.clone());
        if current_admin != stored_admin {
            return Err(PrediFiError::Unauthorized);
        }

        env.storage()
            .instance()
            .set(&DataKey::ProposedAdmin, &new_admin);

        AdminTransferProposedEvent {
            admin: current_admin,
            proposed_admin: new_admin,
        }
        .publish(&env);

        Ok(())
    }

    pub fn accept_admin_role(env: Env, new_admin: Address) -> Result<(), PrediFiError> {
        new_admin.require_auth();

        let proposed_admin: Option<Address> = env.storage().instance().get(&DataKey::ProposedAdmin);
        if proposed_admin != Some(new_admin.clone()) {
            return Err(PrediFiError::Unauthorized);
        }

        let current_admin = Self::get_admin(env.clone());
        Self::apply_admin_transfer(&env, current_admin.clone(), new_admin.clone());

        AdminTransferredEvent {
            admin: current_admin,
            new_admin,
        }
        .publish(&env);

        Ok(())
    }

    pub fn is_admin(env: Env, user: Address) -> bool {
        let stored: Option<Address> = env.storage().instance().get(&DataKey::Admin);
        match stored {
            Some(admin) => admin == user,
            None => false,
        }
    }

    pub fn revoke_all_roles(
        env: Env,
        admin_caller: Address,
        user: Address,
    ) -> Result<(), PrediFiError> {
        admin_caller.require_auth();

        let current_admin = Self::get_admin(env.clone());
        if admin_caller != current_admin {
            return Err(PrediFiError::Unauthorized);
        }

        for role in [
            Role::Admin,
            Role::Operator,
            Role::Moderator,
            Role::Oracle,
            Role::User,
        ]
        .iter()
        {
            let key = DataKey::Role(user.clone(), role.clone());
            if env.storage().persistent().has(&key) {
                env.storage().persistent().remove(&key);
                // Decrement operator count if the Operator role is being removed
                if matches!(role, Role::Operator) {
                    let count: u32 = env
                        .storage()
                        .instance()
                        .get(&DataKey::OperatorCount)
                        .unwrap_or(0);
                    if count > 0 {
                        env.storage()
                            .instance()
                            .set(&DataKey::OperatorCount, &(count - 1));
                    }
                }
            }
        }

        AllRolesRevokedEvent {
            admin: admin_caller,
            user,
        }
        .publish(&env);

        Ok(())
    }

    pub fn has_any_role(env: Env, user: Address, roles: soroban_sdk::Vec<Role>) -> bool {
        for role in roles.iter() {
            if env
                .storage()
                .persistent()
                .has(&DataKey::Role(user.clone(), role))
            {
                return true;
            }
        }
        false
    }

    /// Returns the number of addresses currently holding the Operator role.
    ///
    /// This is used by the predifi-contract to validate that `required_resolutions`
    /// does not exceed the number of active operators (which would make the pool
    /// permanently unresolvable).
    pub fn get_operator_count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::OperatorCount)
            .unwrap_or(0)
    }

    fn apply_admin_transfer(env: &Env, current_admin: Address, new_admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &new_admin);
        env.storage().instance().remove(&DataKey::ProposedAdmin);

        env.storage()
            .persistent()
            .remove(&DataKey::Role(current_admin, Role::Admin));
        env.storage()
            .persistent()
            .set(&DataKey::Role(new_admin.clone(), Role::Admin), &());
    }
}

mod test;

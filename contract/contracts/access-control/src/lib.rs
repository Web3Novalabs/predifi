#![no_std]
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
    /// Role assignment: Role(user_address, role) -> ()
    Role(Address, Role),
    /// Pool data: Pool(pool_id) -> Pool
    Pool(u64),
    /// Pool counter: PoolCount -> u64
    PoolCount,
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

        env.storage()
            .persistent()
            .set(&DataKey::Role(user.clone(), role.clone()), &());

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
        env.storage()
            .persistent()
            .set(&DataKey::Role(to.clone(), role.clone()), &());

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

        env.storage().instance().set(&DataKey::Admin, &new_admin);

        env.storage()
            .persistent()
            .remove(&DataKey::Role(current_admin, Role::Admin));
        env.storage()
            .persistent()
            .set(&DataKey::Role(new_admin.clone(), Role::Admin), &());

        AdminTransferredEvent {
            admin: admin_caller,
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
}

mod test;

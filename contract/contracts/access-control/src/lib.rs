#![no_std]
use predifi_errors::PrediFiError;
use soroban_sdk::{contract, contractevent, contractimpl, contracttype, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Role {
    Admin = 0,
    Operator = 1,
    Moderator = 2,
    Oracle = 3,
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

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoolStatus {
    Active,
    Resolved,
    Closed,
    Disputed,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoolCategory {
    Sports,
    Politics,
    Finance,
    Entertainment,
    Other,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Role(Address, Role),
    Pool(u64),
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

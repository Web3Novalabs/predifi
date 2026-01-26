#![no_std]
use predifi_errors::PrediFiError;
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};
type Error = PrediFiError;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Role {
    Admin = 0,
    Operator = 1,
    Moderator = 2,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoolStatus {
    /// The pool is open for predictions.
    Active,
    /// The event has occurred and the outcome is determined.
    Resolved,
    /// The pool is closed for new predictions but not yet resolved.
    Closed,
    /// The outcome is being disputed.
    Disputed,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoolCategory {
    /// Sports-related predictions.
    Sports,
    /// Political predictions.
    Politics,
    /// Financial predictions.
    Finance,
    /// Entertainment predictions.
    Entertainment,
    /// Other categories.
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
    /// Initialize the contract with an initial admin address.
    ///
    /// # Arguments
    /// * `admin` - The address to be appointed as the initial super admin.
    ///
    /// # Errors
    /// * `AlreadyInitialized` - If the contract has already been initialized.
    pub fn init(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("AlreadyInitialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        // Also grant the Admin role to the admin address
        env.storage()
            .persistent()
            .set(&DataKey::Role(admin, Role::Admin), &());
        Ok(())
    }

    /// Returns the current super admin address.
    ///
    /// # Returns
    /// The address of the current super admin.
    ///
    /// # Errors
    /// * `NotInitialized` - If the contract hasn't been initialized yet.
    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("NotInitialized")
    }

    /// Assigns a specific role to a user.
    ///
    /// Only the current super admin can call this function.
    ///
    /// # Arguments
    /// * `admin_caller` - The address of the admin calling the function.
    /// * `user` - The address to receive the role.
    /// * `role` - The role to be assigned.
    ///
    /// # Errors
    /// * `Unauthorized` - If the caller is not the super admin.
    pub fn assign_role(
        env: Env,
        admin_caller: Address,
        user: Address,
        role: Role,
    ) -> Result<(), Error> {
        admin_caller.require_auth();

        let current_admin = Self::get_admin(env.clone())?;
        if admin_caller != current_admin {
            panic!("Unauthorized");
        }

        env.storage()
            .persistent()
            .set(&DataKey::Role(user, role), &());
        Ok(())
    }

    /// Revokes a specific role from a user.
    ///
    /// Only the current super admin can call this function.
    ///
    /// # Arguments
    /// * `admin_caller` - The address of the admin calling the function.
    /// * `user` - The address from which the role will be revoked.
    /// * `role` - The role to be revoked.
    ///
    /// # Errors
    /// * `Unauthorized` - If the caller is not the super admin.
    /// * `RoleNotFound` - If the user doesn't have the specified role.
    pub fn revoke_role(
        env: Env,
        admin_caller: Address,
        user: Address,
        role: Role,
    ) -> Result<(), Error> {
        admin_caller.require_auth();

        let current_admin = Self::get_admin(env.clone())?;
        if admin_caller != current_admin {
            panic!("Unauthorized");
        }

        if !env
            .storage()
            .persistent()
            .has(&DataKey::Role(user.clone(), role.clone()))
        {
            return Err(Error::RoleNotFound);
        }

        env.storage()
            .persistent()
            .remove(&DataKey::Role(user, role));
        Ok(())
    }

    /// Checks if a user has a specific role.
    ///
    /// # Arguments
    /// * `user` - The address to check.
    /// * `role` - The role to check for.
    ///
    /// # Returns
    /// `true` if the user has the role, `false` otherwise.
    pub fn has_role(env: Env, user: Address, role: Role) -> bool {
        env.storage().persistent().has(&DataKey::Role(user, role))
    }

    /// Transfers a role from one address to another.
    ///
    /// Only the current super admin can call this function.
    ///
    /// # Arguments
    /// * `admin_caller` - The address of the admin calling the function.
    /// * `from` - The address currently holding the role.
    /// * `to` - The address to receive the role.
    /// * `role` - The role to be transferred.
    ///
    /// # Errors
    /// * `Unauthorized` - If the caller is not the super admin.
    /// * `RoleNotFound` - If the `from` address doesn't have the specified role.
    pub fn transfer_role(
        env: Env,
        admin_caller: Address,
        from: Address,
        to: Address,
        role: Role,
    ) {
        admin_caller.require_auth();

        let current_admin = Self::get_admin(env.clone())?;
        if admin_caller != current_admin {
            panic!("Unauthorized");
        }

        if !env
            .storage()
            .persistent()
            .has(&DataKey::Role(from.clone(), role.clone()))
        {
            return Err(Error::RoleNotFound);
        }

        env.storage()
            .persistent()
            .remove(&DataKey::Role(from, role.clone()));
        env.storage()
            .persistent()
            .set(&DataKey::Role(to, role), &());
        Ok(())
    }

    /// Transfers the super admin status to a new address.
    ///
    /// Only the current super admin can call this function.
    ///
    /// # Arguments
    /// * `admin_caller` - The address of the current admin.
    /// * `new_admin` - The address to become the new super admin.
    ///
    /// # Errors
    /// * `Unauthorized` - If the caller is not the current super admin.
    pub fn transfer_admin(
        env: Env,
        admin_caller: Address,
        new_admin: Address,
    ) -> Result<(), Error> {
        admin_caller.require_auth();

        let current_admin = Self::get_admin(env.clone())?;
        if admin_caller != current_admin {
            panic!("Unauthorized");
        }

        // Update the admin address
        env.storage().instance().set(&DataKey::Admin, &new_admin);

        // Transfer the Admin role record
        env.storage()
            .persistent()
            .remove(&DataKey::Role(current_admin, Role::Admin));
        env.storage()
            .persistent()
            .set(&DataKey::Role(new_admin, Role::Admin), &());

        Ok(())
    }
}

mod test;

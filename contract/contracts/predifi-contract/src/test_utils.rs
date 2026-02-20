#![cfg(test)]

use soroban_sdk::{token, Address, Env};

pub struct TokenTestContext {
    pub token_address: Address,
    pub token: token::Client<'static>,
    pub admin: token::StellarAssetClient<'static>,
}

impl TokenTestContext {
    pub fn deploy(env: &Env, admin: &Address) -> Self {
        let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
        let token = token::Client::new(env, &token_contract);
        let token_admin = token::StellarAssetClient::new(env, &token_contract);

        Self {
            token_address: token_contract,
            token,
            admin: token_admin,
        }
    }

    pub fn mint(&self, to: &Address, amount: i128) {
        self.admin.mint(to, &amount);
    }
}

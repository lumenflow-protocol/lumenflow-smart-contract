#![no_std]

//! BRK — Birkinlabs governance and discount token.

use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Env, String,
};

#[contracttype]
pub enum DataKey {
    Balance(Address),
    Allowance(Address, Address),
    Admin,
    TotalSupply,
    Name,
    Symbol,
    Decimals,
}

#[contract]
pub struct BrkToken;

#[contractimpl]
impl BrkToken {
    /// Deploy and mint initial supply to admin.
    pub fn initialize(env: Env, admin: Address, initial_supply: i128) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Name, &String::from_str(&env, "Birkinlabs Token"));
        env.storage().instance().set(&DataKey::Symbol, &String::from_str(&env, "BRK"));
        env.storage().instance().set(&DataKey::Decimals, &7u32);
        env.storage().instance().set(&DataKey::TotalSupply, &initial_supply);
        env.storage().persistent().set(&DataKey::Balance(admin), &initial_supply);
    }

    pub fn name(env: Env) -> String {
        env.storage().instance().get(&DataKey::Name).unwrap()
    }

    pub fn symbol(env: Env) -> String {
        env.storage().instance().get(&DataKey::Symbol).unwrap()
    }

    pub fn decimals(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::Decimals).unwrap()
    }

    pub fn total_supply(env: Env) -> i128 {
        env.storage().instance().get(&DataKey::TotalSupply).unwrap_or(0)
    }

    pub fn balance(env: Env, account: Address) -> i128 {
        env.storage().persistent().get(&DataKey::Balance(account)).unwrap_or(0)
    }

    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        assert!(amount > 0, "Amount must be positive");

        let from_balance = Self::balance(env.clone(), from.clone());
        assert!(from_balance >= amount, "Insufficient balance");

        env.storage().persistent().set(&DataKey::Balance(from), &(from_balance - amount));

        let to_balance = Self::balance(env.clone(), to.clone());
        env.storage().persistent().set(&DataKey::Balance(to), &(to_balance + amount));
    }

    /// Admin mints new tokens (e.g. for buyer rewards).
    pub fn mint(env: Env, to: Address, amount: i128) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        admin.require_auth();
        assert!(amount > 0, "Amount must be positive");

        let to_balance = Self::balance(env.clone(), to.clone());
        env.storage().persistent().set(&DataKey::Balance(to), &(to_balance + amount));

        let supply: i128 = Self::total_supply(env.clone());
        env.storage().instance().set(&DataKey::TotalSupply, &(supply + amount));
    }

    /// Admin burns tokens.
    pub fn burn(env: Env, from: Address, amount: i128) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        admin.require_auth();

        let balance = Self::balance(env.clone(), from.clone());
        assert!(balance >= amount, "Insufficient balance to burn");

        env.storage().persistent().set(&DataKey::Balance(from), &(balance - amount));

        let supply: i128 = Self::total_supply(env.clone());
        env.storage().instance().set(&DataKey::TotalSupply, &(supply - amount));
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::Env;

    #[test]
    fn test_token_lifecycle() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let buyer = Address::generate(&env);

        let contract_id = env.register_contract(None, BrkToken);
        let client = BrkTokenClient::new(&env, &contract_id);

        client.initialize(&admin, &1_000_000_0000000i128);
        assert_eq!(client.balance(&admin), 1_000_000_0000000i128);

        client.mint(&buyer, &100_0000000);
        assert_eq!(client.balance(&buyer), 100_0000000);

        client.transfer(&buyer, &admin, &50_0000000);
        assert_eq!(client.balance(&buyer), 50_0000000);
    }
}

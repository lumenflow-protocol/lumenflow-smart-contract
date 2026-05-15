#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, String};

#[contracttype]
pub enum DataKey {
    Vault(String),
    Admin,
}

#[contracttype]
#[derive(Clone, PartialEq)]
pub enum VaultStatus {
    Locked,
    Released,
    Refunded,
    Disputed,
}

#[contracttype]
#[derive(Clone)]
pub struct Vault {
    pub id: String,
    pub buyer: Address,
    pub seller: Address,
    pub token: Address,
    pub amount: i128,
    pub status: VaultStatus,
}

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Lock funds in escrow vault.
    pub fn lock(
        env: Env,
        vault_id: String,
        buyer: Address,
        seller: Address,
        token: Address,
        amount: i128,
    ) {
        buyer.require_auth();
        assert!(amount > 0, "Amount must be positive");

        let existing: Option<Vault> = env.storage().persistent().get(&DataKey::Vault(vault_id.clone()));
        assert!(existing.is_none(), "Vault already exists");

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&buyer, &env.current_contract_address(), &amount);

        env.storage().persistent().set(
            &DataKey::Vault(vault_id.clone()),
            &Vault { id: vault_id, buyer, seller, token, amount, status: VaultStatus::Locked },
        );
    }

    /// Buyer confirms delivery and releases funds to seller.
    pub fn release(env: Env, vault_id: String, buyer: Address) {
        buyer.require_auth();

        let mut vault: Vault = env
            .storage().persistent().get(&DataKey::Vault(vault_id.clone()))
            .expect("Vault not found");

        assert!(vault.buyer == buyer, "Only buyer can release");
        assert!(vault.status == VaultStatus::Locked, "Vault not locked");

        token::Client::new(&env, &vault.token)
            .transfer(&env.current_contract_address(), &vault.seller, &vault.amount);

        vault.status = VaultStatus::Released;
        env.storage().persistent().set(&DataKey::Vault(vault_id), &vault);
    }

    /// Admin refunds buyer.
    pub fn refund(env: Env, vault_id: String) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        admin.require_auth();

        let mut vault: Vault = env
            .storage().persistent().get(&DataKey::Vault(vault_id.clone()))
            .expect("Vault not found");

        assert!(
            vault.status == VaultStatus::Locked || vault.status == VaultStatus::Disputed,
            "Cannot refund"
        );

        token::Client::new(&env, &vault.token)
            .transfer(&env.current_contract_address(), &vault.buyer, &vault.amount);

        vault.status = VaultStatus::Refunded;
        env.storage().persistent().set(&DataKey::Vault(vault_id), &vault);
    }

    /// Buyer opens a dispute.
    pub fn dispute(env: Env, vault_id: String, buyer: Address) {
        buyer.require_auth();

        let mut vault: Vault = env
            .storage().persistent().get(&DataKey::Vault(vault_id.clone()))
            .expect("Vault not found");

        assert!(vault.buyer == buyer, "Only buyer can dispute");
        assert!(vault.status == VaultStatus::Locked, "Vault not disputable");

        vault.status = VaultStatus::Disputed;
        env.storage().persistent().set(&DataKey::Vault(vault_id), &vault);
    }

    pub fn get_vault(env: Env, vault_id: String) -> Vault {
        env.storage().persistent().get(&DataKey::Vault(vault_id)).expect("Vault not found")
    }
}

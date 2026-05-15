#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, Env, String,
};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Order(String),
    Admin,
}

#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum OrderStatus {
    Pending,
    Paid,
    Fulfilled,
    Refunded,
    Disputed,
}

#[contracttype]
#[derive(Clone)]
pub struct Order {
    pub id: String,
    pub buyer: Address,
    pub seller: Address,
    pub token: Address,
    pub amount: i128,
    pub status: OrderStatus,
}

#[contract]
pub struct PaymentContract;

#[contractimpl]
impl PaymentContract {
    /// Initialize contract with an admin address.
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Buyer pays for an order — funds transferred to contract (held in escrow).
    pub fn pay(
        env: Env,
        order_id: String,
        buyer: Address,
        seller: Address,
        token: Address,
        amount: i128,
    ) {
        buyer.require_auth();
        assert!(amount > 0, "Amount must be positive");

        let existing: Option<Order> = env.storage().persistent().get(&DataKey::Order(order_id.clone()));
        assert!(existing.is_none(), "Order already exists");

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&buyer, &env.current_contract_address(), &amount);

        let order = Order {
            id: order_id.clone(),
            buyer,
            seller,
            token,
            amount,
            status: OrderStatus::Paid,
        };

        env.storage().persistent().set(&DataKey::Order(order_id), &order);
    }

    /// Buyer confirms delivery — releases funds to seller.
    pub fn release(env: Env, order_id: String, buyer: Address) {
        buyer.require_auth();

        let mut order: Order = env
            .storage()
            .persistent()
            .get(&DataKey::Order(order_id.clone()))
            .expect("Order not found");

        assert!(order.buyer == buyer, "Only buyer can release");
        assert!(order.status == OrderStatus::Paid, "Order not in Paid state");

        let token_client = token::Client::new(&env, &order.token);
        token_client.transfer(&env.current_contract_address(), &order.seller, &order.amount);

        order.status = OrderStatus::Fulfilled;
        env.storage().persistent().set(&DataKey::Order(order_id), &order);
    }

    /// Admin refunds buyer (e.g. dispute resolved in buyer's favour).
    pub fn refund(env: Env, order_id: String) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        admin.require_auth();

        let mut order: Order = env
            .storage()
            .persistent()
            .get(&DataKey::Order(order_id.clone()))
            .expect("Order not found");

        assert!(order.status == OrderStatus::Paid || order.status == OrderStatus::Disputed, "Cannot refund");

        let token_client = token::Client::new(&env, &order.token);
        token_client.transfer(&env.current_contract_address(), &order.buyer, &order.amount);

        order.status = OrderStatus::Refunded;
        env.storage().persistent().set(&DataKey::Order(order_id), &order);
    }

    /// Buyer raises a dispute.
    pub fn dispute(env: Env, order_id: String, buyer: Address) {
        buyer.require_auth();

        let mut order: Order = env
            .storage()
            .persistent()
            .get(&DataKey::Order(order_id.clone()))
            .expect("Order not found");

        assert!(order.buyer == buyer, "Only buyer can dispute");
        assert!(order.status == OrderStatus::Paid, "Order not disputable");

        order.status = OrderStatus::Disputed;
        env.storage().persistent().set(&DataKey::Order(order_id), &order);
    }

    /// Read order state.
    pub fn get_order(env: Env, order_id: String) -> Order {
        env.storage()
            .persistent()
            .get(&DataKey::Order(order_id))
            .expect("Order not found")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{token::StellarAssetClient, Env, String};

    #[test]
    fn test_full_happy_path() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let buyer = Address::generate(&env);
        let seller = Address::generate(&env);

        // Deploy a test token
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_addr = token_id.address();
        let sac = StellarAssetClient::new(&env, &token_addr);
        sac.mint(&buyer, &1000);

        // Deploy payment contract
        let contract_id = env.register_contract(None, PaymentContract);
        let client = PaymentContractClient::new(&env, &contract_id);

        client.initialize(&admin);

        let order_id = String::from_str(&env, "order-001");
        client.pay(&order_id, &buyer, &seller, &token_addr, &500);

        let order = client.get_order(&order_id);
        assert_eq!(order.status, OrderStatus::Paid);

        client.release(&order_id, &buyer);

        let order = client.get_order(&order_id);
        assert_eq!(order.status, OrderStatus::Fulfilled);
    }
}

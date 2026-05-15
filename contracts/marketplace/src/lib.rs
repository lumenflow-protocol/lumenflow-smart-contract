#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, vec, Address, Env, String, Vec,
};

#[contracttype]
pub enum DataKey {
    Listing(String),
    SellerListings(Address),
    Admin,
}

#[contracttype]
#[derive(Clone)]
pub struct Listing {
    pub id: String,
    pub seller: Address,
    pub name: String,
    pub price: i128,
    pub token: Address,
    pub stock: u32,
    pub active: bool,
}

#[contract]
pub struct MarketplaceContract;

#[contractimpl]
impl MarketplaceContract {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Seller lists a product on-chain.
    pub fn list_product(
        env: Env,
        listing_id: String,
        seller: Address,
        name: String,
        price: i128,
        token: Address,
        stock: u32,
    ) {
        seller.require_auth();
        assert!(price > 0, "Price must be positive");
        assert!(stock > 0, "Stock must be positive");

        let existing: Option<Listing> = env.storage().persistent().get(&DataKey::Listing(listing_id.clone()));
        assert!(existing.is_none(), "Listing already exists");

        let listing = Listing {
            id: listing_id.clone(),
            seller: seller.clone(),
            name,
            price,
            token,
            stock,
            active: true,
        };

        env.storage().persistent().set(&DataKey::Listing(listing_id.clone()), &listing);

        // Track seller's listings
        let mut seller_listings: Vec<String> = env
            .storage()
            .persistent()
            .get(&DataKey::SellerListings(seller.clone()))
            .unwrap_or(vec![&env]);
        seller_listings.push_back(listing_id);
        env.storage().persistent().set(&DataKey::SellerListings(seller), &seller_listings);
    }

    /// Seller updates stock after a sale.
    pub fn update_stock(env: Env, listing_id: String, seller: Address, new_stock: u32) {
        seller.require_auth();

        let mut listing: Listing = env
            .storage()
            .persistent()
            .get(&DataKey::Listing(listing_id.clone()))
            .expect("Listing not found");

        assert!(listing.seller == seller, "Only seller can update");

        listing.stock = new_stock;
        listing.active = new_stock > 0;
        env.storage().persistent().set(&DataKey::Listing(listing_id), &listing);
    }

    /// Seller deactivates their listing.
    pub fn delist(env: Env, listing_id: String, seller: Address) {
        seller.require_auth();

        let mut listing: Listing = env
            .storage()
            .persistent()
            .get(&DataKey::Listing(listing_id.clone()))
            .expect("Listing not found");

        assert!(listing.seller == seller, "Only seller can delist");

        listing.active = false;
        env.storage().persistent().set(&DataKey::Listing(listing_id), &listing);
    }

    pub fn get_listing(env: Env, listing_id: String) -> Listing {
        env.storage()
            .persistent()
            .get(&DataKey::Listing(listing_id))
            .expect("Listing not found")
    }

    pub fn get_seller_listings(env: Env, seller: Address) -> Vec<String> {
        env.storage()
            .persistent()
            .get(&DataKey::SellerListings(seller))
            .unwrap_or(vec![&env])
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Env, String};

    #[test]
    fn test_list_and_delist() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let seller = Address::generate(&env);
        let token = Address::generate(&env);

        let contract_id = env.register_contract(None, MarketplaceContract);
        let client = MarketplaceContractClient::new(&env, &contract_id);

        client.initialize(&admin);

        let listing_id = String::from_str(&env, "listing-001");
        let name = String::from_str(&env, "Stella Tote Bag");

        client.list_product(&listing_id, &seller, &name, &500, &token, &10);

        let listing = client.get_listing(&listing_id);
        assert!(listing.active);
        assert_eq!(listing.stock, 10);

        client.delist(&listing_id, &seller);
        let listing = client.get_listing(&listing_id);
        assert!(!listing.active);
    }
}

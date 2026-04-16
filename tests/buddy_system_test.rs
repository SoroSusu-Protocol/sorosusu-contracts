#![cfg(test)]
use soroban_sdk::testutils::Address as _;

use soroban_sdk::{
    contract, contractimpl, token,
    testutils::Address as _,
    Address, Env,
};
use sorosusu_contracts::{DataKey, SoroSusu, SoroSusuClient};

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
}

#[allow(deprecated)]
fn register_token(env:, &Env, admin:, &Address) -> Address {
    env.register_stellar_asset_contract(admin.clone())
}

#[test]
fn test_buddy_pairing() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    
    // Register mock token
    let token_admin = Address::generate(&env);
    let token = register_token(&env, &token_admin);
    
    let _nft_contract = env.register_contract(None, MockNft);

    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    // Initialize contract
    client.init(&admin, &100);

    // Create a circle
    let circle_id = client.create_circle(&creator, &1000, &5, &token, &604800, &0);

    // Both users join the circle
    client.join_circle(&user1, &circle_id);
    client.join_circle(&user2, &circle_id);

    // User1 pairs with User2 as buddy
    client.pair_with_member(&user1, &user2);

    // User2 sets safety deposit
    // Need to mint tokens to user2 first
    let token_admin_client = token::StellarAssetClient::new(&env, &token);
    token_admin_client.mint(&user2, &5000);

    client.set_safety_deposit(&user2, &circle_id, &2000);
}

#[test]
fn test_buddy_payment_fallback() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    
    // Register mock token
    let token_admin = Address::generate(&env);
    let token = register_token(&env, &token_admin);

    let _nft_contract = env.register_contract(None, MockNft);

    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    // Initialize contract
    client.init(&admin, &100);

    // Create a circle
    let circle_id = client.create_circle(&creator, &1000, &5, &token, &604800, &0);

    // Both users join the circle
    client.join_circle(&user1, &circle_id);
    client.join_circle(&user2, &circle_id);

    // User1 pairs with User2 as buddy
    client.pair_with_member(&user1, &user2);

    // User2 sets safety deposit(enough to cover user1's payment)
    let token_admin_client = token::StellarAssetClient::new(&env, &token, &1);
    let token_client = token::Client::new(&env, &token);
    token_admin_client.mint(&user2, &5000);

    client.set_safety_deposit(&user2, &circle_id, &2000);

    client.deposit(&user1, &circle_id, &1);

    env.as_contract(&contract_id, || {
        let remaining_deposit: i128 = env
            .storage()
            .instance()
            .get(&DataKey::SafetyDeposit(user2.clone(), circle_id))
            .unwrap();
        assert_eq!(remaining_deposit, 1000);
    });

    assert_eq!(token_client.balance(&user2), 3000);
}



























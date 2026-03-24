use soroban_sdk::{testutils::Address as _, Address, Env, contract, contractimpl};
use sorosusu_contracts::{SoroSusu, SoroSusuClient, SoroSusuTrait};

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
}

#[test]
fn test_global_blacklist() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    
    client.init(&admin);
    
    // Check initial blacklist status
    assert!(!client.is_blacklisted(&user));
    
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);
    let circle_id = client.create_circle(&creator, &1000, &5, &token, &86400, &100, &nft_contract, &0);
    
    // Join circle
    client.join_circle(&user, &circle_id, &1, &None);
    
    // Mark as defaulted
    client.mark_member_defaulted(&creator, &circle_id, &user);
    
    // Check blacklist status again
    assert!(client.is_blacklisted(&user));
    
    // Admin can remove from blacklist
    client.update_blacklist_status(&admin, &user, &false);
    assert!(!client.is_blacklisted(&user));
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_unauthorized_blacklist_update() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);
    let user = Address::generate(&env);
    
    client.init(&admin);
    
    client.update_blacklist_status(&non_admin, &user, &true);
}

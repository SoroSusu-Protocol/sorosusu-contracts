#![cfg_attr(not(test), no_std)]
use soroban_sdk::{testutils::Address as _, Address, Env, token, Vec, String};
use sorosusu_contracts::{SoroSusu, SoroSusuClient, AnchorInfo, AnchorDeposit, AuditAction};

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
}

fn register_token(env: &Env, admin: &Address) -> Address {
    env.register_stellar_asset_contract(admin.clone())
}

#[test]
fn test_anchor_deposit_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let anchor_address = Address::generate(&env);

    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    client.init(&admin, &0);

    let token_admin = Address::generate(&env);
    let token_id = register_token(&env, &token_admin);
    let token_client = token::StellarAssetClient::new(&env, &token_id);

    let anchor_info = AnchorInfo {
        anchor_address: anchor_address.clone(),
        anchor_name: String::from_str(&env, "Test Anchor"),
        sep_version: String::from_str(&env, "SEP-24"),
        authorization_level: 2,
        compliance_level: 2,
        is_active: true,
        registration_timestamp: env.ledger().timestamp(),
        last_activity: env.ledger().timestamp(),
        supported_countries: Vec::new(&env),
        max_deposit_amount: 10_000_000_000,
        daily_deposit_limit: 50_000_000_000,
    };

    client.register_anchor(&admin, &anchor_info);
    token_client.mint(&anchor_address, &10_000_000_000);

    let circle_id = client.create_circle(&creator, &1_000_000_000, &2, &token_id, &86400, &0);
    client.join_circle(&user, &circle_id);

    let memo = String::from_str(&env, "DEP_123");
    let fiat = String::from_str(&env, "BANK_TX");
    let sep = String::from_str(&env, "SEP-24");

    client.deposit_for_user(&anchor_address, &user, &circle_id, &1_000_000_000, &memo, &fiat, &sep);

    let deposit_id = env.ledger().sequence() as u64; // Based on implementation
    let record = client.get_deposit_record(&deposit_id);
    assert!(record.processed);
    assert_eq!(record.beneficiary_user, user);

    let member = client.get_member(&user);
    assert!(member.has_contributed_current_round);
}

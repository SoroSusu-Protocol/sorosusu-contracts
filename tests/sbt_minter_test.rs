#![cfg_attr(not(test), no_std)]
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
use sorosusu_contracts::{
    SoroSusuSbtMinter, SoroSusuSbtMinterClient, SbtStatus, ReputationTier, 
    SoroSusuCredential, ReputationMilestone, UserReputationMetrics, DataKey
};

#[test]
fn test_sbt_minter_flow() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    
    let minter_id = env.register_contract(None, SoroSusuSbtMinter);
    let client = SoroSusuSbtMinterClient::new(&env, &minter_id);
    
    client.init_sbt_minter(&admin);
    
    let desc = String::from_str(&env, "Complete 5 cycles");
    let mid = client.create_reputation_milestone(&user, &5, &desc, &ReputationTier::Silver);
    
    let m = client.get_reputation_milestone(&mid);
    assert_eq!(m.cycles_required, 5);

    let tid = client.issue_credential(&user, &mid, &String::from_str(&env, "uri"));
    let cred = client.get_credential(&tid);
    assert_eq!(cred.holder, user);
    assert_eq!(cred.reputation_tier, ReputationTier::Silver);
}

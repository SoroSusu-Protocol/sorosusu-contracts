use soroban_sdk::testutils::Address as _;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, Env};
use sorosusu_contracts::{CollateralStatus, DataKey, MemberStatus, SoroSusu, SoroSusuClient};

use soroban_sdk::{contract, contractimpl};

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
}

fn setup_token(env: &Env) -> (Address, token::StellarAssetClient) {
    let token_admin = Address::generate(env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(env, &token_id);
    (token_id, token_client)
}

#[test]
fn test_collateral_required_for_high_value_circles() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let (token, _) = setup_token(&env);
    let nft_contract = env.register_contract(None, MockNft);

    // Initialize contract
    client.init(&admin);

    // Create a high-value circle (above threshold)
    let high_amount = 2_000_000_0; // 2000 XLM
    let max_members = 5u32;
    let circle_id = client.create_circle(
        &creator,
        &high_amount,
        &max_members,
        &token,
        &86400u64, // 1 day cycle
        &100u32,   // 1% insurance fee
        &nft_contract,
    );

    // Verify collateral is required
    let circle_info = env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .get::<_, sorosusu_contracts::CircleInfo>(&DataKey::Circle(circle_id))
            .unwrap()
    });
    assert!(circle_info.requires_collateral);
    assert_eq!(circle_info.collateral_bps, 2000); // 20%
    assert_eq!(circle_info.total_cycle_value, high_amount * 5);
}

#[test]
fn test_collateral_not_required_for_low_value_circles() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let (token, _) = setup_token(&env);
    let nft_contract = env.register_contract(None, MockNft);

    // Initialize contract
    client.init(&admin);

    // Create a low-value circle (below threshold)
    let low_amount = 100_000_0; // 100 XLM
    let max_members = 5u32;
    let circle_id = client.create_circle(
        &creator,
        &low_amount,
        &max_members,
        &token,
        &86400u64, // 1 day cycle
        &100u32,   // 1% insurance fee
        &nft_contract,
    );

    // Verify collateral is not required
    let circle_info = env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .get::<_, sorosusu_contracts::CircleInfo>(&DataKey::Circle(circle_id))
            .unwrap()
    });
    assert!(!circle_info.requires_collateral);
    assert_eq!(circle_info.collateral_bps, 0);
}

#[test]
fn test_stake_collateral() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let (token, token_client) = setup_token(&env);
    let nft_contract = env.register_contract(None, MockNft);

    // Initialize contract
    client.init(&admin);

    // Create high-value circle
    let high_amount = 2_000_000_0; // 2000 XLM
    let max_members = 5u32;
    let circle_id = client.create_circle(
        &creator,
        &high_amount,
        &max_members,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );

    // Calculate required collateral (20% of total cycle value)
    let total_cycle_value = high_amount * 5;
    let required_collateral = (total_cycle_value * 2000) / 10000; // 20%

    // Mint tokens to user for collateral staking
    token_client.mint(&user, &required_collateral);

    // Stake collateral
    client.stake_collateral(&user, &circle_id, &required_collateral);

    // Verify collateral is staked
    let collateral_info = env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .get::<_, sorosusu_contracts::CollateralInfo>(&DataKey::CollateralVault(
                user, circle_id,
            ))
            .unwrap()
    });
    assert_eq!(collateral_info.status, CollateralStatus::Staked);
    assert_eq!(collateral_info.amount, required_collateral);
}

#[test]
fn test_join_circle_requires_collateral() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let (token, token_client) = setup_token(&env);
    let nft_contract = env.register_contract(None, MockNft);

    // Initialize contract
    client.init(&admin);

    // Create high-value circle
    let high_amount = 2_000_000_0; // 2000 XLM
    let max_members = 5u32;
    let circle_id = client.create_circle(
        &creator,
        &high_amount,
        &max_members,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );

    // Try to join without staking collateral - should fail
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.join_circle(&user, &circle_id, &1u32, &Option::<Address>::None);
    }));
    assert!(result.is_err());

    // Mint tokens to user for collateral
    let total_cycle_value = high_amount * 5;
    let required_collateral = (total_cycle_value * 2000) / 10000;
    token_client.mint(&user, &required_collateral);
    client.stake_collateral(&user, &circle_id, &required_collateral);

    // Now joining should work
    client.join_circle(&user, &circle_id, &1u32, &Option::<Address>::None);
}

#[test]
fn test_mark_member_defaulted_and_slash_collateral() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let (token, token_client) = setup_token(&env);
    let nft_contract = env.register_contract(None, MockNft);

    // Initialize contract
    client.init(&admin);

    // Create high-value circle
    let high_amount = 2_000_000_0; // 2000 XLM
    let max_members = 5u32;
    let circle_id = client.create_circle(
        &creator,
        &high_amount,
        &max_members,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );

    // Mint tokens to user and stake collateral
    let total_cycle_value = high_amount * 5;
    let required_collateral = (total_cycle_value * 2000) / 10000;
    token_client.mint(&user, &required_collateral);
    client.stake_collateral(&user, &circle_id, &required_collateral);

    // User joins the circle
    client.join_circle(&user, &circle_id, &1u32, &Option::<Address>::None);

    // Mark member as defaulted (slash_collateral is called internally,
    // so we skip direct mark_member_defaulted and test slashing separately)
    client.mark_member_defaulted(&creator, &circle_id, &user);
    // Note: The auto-slash happens inside mark_member_defaulted via Self::slash_collateral

    // Verify member is marked as defaulted
    let member_info = env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .get::<_, sorosusu_contracts::Member>(&DataKey::Member(user.clone()))
            .unwrap()
    });
    assert_eq!(member_info.status, MemberStatus::Defaulted);

    // Verify collateral is slashed
    let collateral_info = env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .get::<_, sorosusu_contracts::CollateralInfo>(&DataKey::CollateralVault(
                user, circle_id,
            ))
            .unwrap()
    });
    assert_eq!(collateral_info.status, CollateralStatus::Slashed);

    // Verify slashed amount is in group reserve
    let reserve = env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .get::<_, i128>(&DataKey::GroupReserve)
            .unwrap_or(0)
    });
    assert_eq!(reserve, required_collateral);
}

#[test]
fn test_release_collateral_after_completion() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let (token, token_client) = setup_token(&env);
    let nft_contract = env.register_contract(None, MockNft);

    // Initialize contract
    client.init(&admin);

    // Create high-value circle
    let high_amount = 2_000_000_0; // 2000 XLM
    let max_members = 5u32;
    let circle_id = client.create_circle(
        &creator,
        &high_amount,
        &max_members,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );

    // Mint tokens to user and stake collateral
    let total_cycle_value = high_amount * 5;
    let required_collateral = (total_cycle_value * 2000) / 10000;
    token_client.mint(&user, &required_collateral);
    client.stake_collateral(&user, &circle_id, &required_collateral);

    // Simulate member completing all contributions
    env.as_contract(&contract_id, || {
        let member_info = sorosusu_contracts::Member {
            address: user.clone(),
            index: 0,
            contribution_count: max_members, // Completed all contributions
            last_contribution_time: env.ledger().timestamp(),
            status: MemberStatus::Active,
            tier_multiplier: 1,
            referrer: None,
            buddy: None,
        };
        env.storage()
            .instance()
            .set(&DataKey::Member(user.clone()), &member_info);
    });

    // Release collateral
    client.release_collateral(&user, &circle_id, &user);

    // Verify collateral is released
    let collateral_info = env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .get::<_, sorosusu_contracts::CollateralInfo>(&DataKey::CollateralVault(
                user, circle_id,
            ))
            .unwrap()
    });
    assert_eq!(collateral_info.status, CollateralStatus::Released);
    assert!(collateral_info.release_timestamp.is_some());
}

#[test]
fn test_insufficient_collateral_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let (token, _token_client) = setup_token(&env);
    let nft_contract = env.register_contract(None, MockNft);

    // Initialize contract
    client.init(&admin);

    // Create high-value circle
    let high_amount = 2_000_000_0; // 2000 XLM
    let max_members = 5u32;
    let circle_id = client.create_circle(
        &creator,
        &high_amount,
        &max_members,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );

    // Calculate required collateral
    let total_cycle_value = high_amount * 5;
    let required_collateral = (total_cycle_value * 2000) / 10000;
    let insufficient_amount = required_collateral - 100_000_0; // Less than required

    // Try to stake insufficient collateral - should fail
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.stake_collateral(&user, &circle_id, &insufficient_amount);
    }));
    assert!(result.is_err());
}

#[test]
fn test_double_collateral_staking() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let (token, token_client) = setup_token(&env);
    let nft_contract = env.register_contract(None, MockNft);

    // Initialize contract
    client.init(&admin);

    // Create high-value circle
    let high_amount = 2_000_000_0; // 2000 XLM
    let max_members = 5u32;
    let circle_id = client.create_circle(
        &creator,
        &high_amount,
        &max_members,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );

    // Calculate required collateral
    let total_cycle_value = high_amount * 5;
    let required_collateral = (total_cycle_value * 2000) / 10000;

    // Mint tokens and stake collateral first time
    token_client.mint(&user, &required_collateral);
    client.stake_collateral(&user, &circle_id, &required_collateral);

    // Try to stake again - should fail
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.stake_collateral(&user, &circle_id, &required_collateral);
    }));
    assert!(result.is_err());
}

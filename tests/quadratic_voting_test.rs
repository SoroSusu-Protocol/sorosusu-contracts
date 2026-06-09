use soroban_sdk::{contract, contractimpl};

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
}
use soroban_sdk::testutils::Address as _;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{Address, Env, String};
use sorosusu_contracts::{
    DataKey, ProposalStatus, ProposalType, QuadraticVoteChoice, SoroSusu, SoroSusuClient,
};

#[test]
fn test_quadratic_voting_enabled_for_large_groups() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);

    client.init(&admin);

    let circle_id = client.create_circle(
        &creator,
        &100_000,
        &15u32,
        &token,
        &86400u64,
        &0u32,
        &nft_contract,
    );

    let circle_info = env.as_contract(&contract_id, || {
        let circle_key = DataKey::Circle(circle_id);
        env.storage()
            .instance()
            .get::<_, sorosusu_contracts::CircleInfo>(&circle_key)
            .unwrap()
    });
    assert!(circle_info.quadratic_voting_enabled);

    let small_circle_id = client.create_circle(
        &Address::generate(&env),
        &100_000,
        &5u32,
        &token,
        &86400u64,
        &0u32,
        &nft_contract,
    );

    let small_circle_info = env.as_contract(&contract_id, || {
        let small_circle_key = DataKey::Circle(small_circle_id);
        env.storage()
            .instance()
            .get::<_, sorosusu_contracts::CircleInfo>(&small_circle_key)
            .unwrap()
    });
    assert!(!small_circle_info.quadratic_voting_enabled);
}

#[test]
fn test_create_proposal() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let proposer = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);

    client.init(&admin);

    let circle_id = client.create_circle(
        &creator,
        &100_000,
        &15u32,
        &token,
        &86400u64,
        &0u32,
        &nft_contract,
    );

    client.join_circle(&proposer, &circle_id, &1u32, &None);

    let title = String::from_str(&env, "Reduce late fee to 0.5%");
    let description = String::from_str(&env, "Proposal to reduce late fee from 1% to 0.5%");
    let execution_data = String::from_str(&env, "{\"late_fee_bps\": 50}");

    let proposal_id = client.create_proposal(
        &proposer,
        &circle_id,
        &ProposalType::ChangeLateFee,
        &title,
        &description,
        &execution_data,
    );

    let proposal = client.get_proposal(&proposal_id);
    assert_eq!(proposal.proposer, proposer);
    assert_eq!(proposal.circle_id, circle_id);
    assert_eq!(proposal.proposal_type, ProposalType::ChangeLateFee);
    assert_eq!(proposal.status, ProposalStatus::Active);
    assert_eq!(proposal.for_votes, 0);
    assert_eq!(proposal.against_votes, 0);
}

#[test]
fn test_create_proposal_fails_for_small_groups() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let proposer = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);

    client.init(&admin);

    let circle_id = client.create_circle(
        &creator,
        &100_000,
        &5u32,
        &token,
        &86400u64,
        &0u32,
        &nft_contract,
    );

    client.join_circle(&proposer, &circle_id, &1u32, &None);

    let title = String::from_str(&env, "Test proposal");
    let description = String::from_str(&env, "Test description");
    let execution_data = String::from_str(&env, "{}");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.create_proposal(
            &proposer,
            &circle_id,
            &ProposalType::ChangeLateFee,
            &title,
            &description,
            &execution_data,
        );
    }));
    assert!(result.is_err());
}

#[test]
fn test_voting_power_calculation() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let member = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);

    client.init(&admin);

    let circle_id = client.create_circle(
        &creator,
        &100_000,
        &15u32,
        &token,
        &86400u64,
        &0u32,
        &nft_contract,
    );

    client.join_circle(&member, &circle_id, &1u32, &None);

    client.update_voting_power(&member, &circle_id, &1_000_000_0);
    let voting_power = client.get_voting_power(&member, &circle_id);
    assert_eq!(voting_power.token_balance, 1_000_000_0);
    assert!(voting_power.quadratic_power > 0);

    client.update_voting_power(&member, &circle_id, &0);
    let zero_power = client.get_voting_power(&member, &circle_id);
    assert_eq!(zero_power.token_balance, 0);
    assert_eq!(zero_power.quadratic_power, 0);
}

#[test]
fn test_quadratic_vote_cost_calculation() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);

    client.init(&admin);

    let circle_id = client.create_circle(
        &creator,
        &100_000,
        &15u32,
        &token,
        &86400u64,
        &0u32,
        &nft_contract,
    );

    client.join_circle(&proposer, &circle_id, &1u32, &None);
    client.join_circle(&voter, &circle_id, &1u32, &None);

    let title = String::from_str(&env, "Test proposal");
    let description = String::from_str(&env, "Test description");
    let execution_data = String::from_str(&env, "{}");

    let proposal_id = client.create_proposal(
        &proposer,
        &circle_id,
        &ProposalType::ChangeLateFee,
        &title,
        &description,
        &execution_data,
    );

    client.update_voting_power(&voter, &circle_id, &10_000_000_0);

    client.quadratic_vote(&voter, &proposal_id, &10u32, &QuadraticVoteChoice::For);

    let proposal = client.get_proposal(&proposal_id);
    assert_eq!(proposal.for_votes, 100);
    assert_eq!(proposal.against_votes, 0);
    assert!(proposal.total_voting_power > 0);
}

#[test]
fn test_insufficient_voting_power() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);

    client.init(&admin);

    let circle_id = client.create_circle(
        &creator,
        &100_000,
        &15u32,
        &token,
        &86400u64,
        &0u32,
        &nft_contract,
    );

    client.join_circle(&proposer, &circle_id, &1u32, &None);
    client.join_circle(&voter, &circle_id, &1u32, &None);

    let title = String::from_str(&env, "Test proposal");
    let description = String::from_str(&env, "Test description");
    let execution_data = String::from_str(&env, "{}");

    let proposal_id = client.create_proposal(
        &proposer,
        &circle_id,
        &ProposalType::ChangeLateFee,
        &title,
        &description,
        &execution_data,
    );

    client.update_voting_power(&voter, &circle_id, &50_000); // quadratic_power = 50

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.quadratic_vote(&voter, &proposal_id, &10u32, &QuadraticVoteChoice::For);
        // cost = 100 > 50
    }));
    assert!(result.is_err());

    client.quadratic_vote(&voter, &proposal_id, &5u32, &QuadraticVoteChoice::For);

    let proposal = client.get_proposal(&proposal_id);
    assert_eq!(proposal.for_votes, 25);
}

#[test]
fn test_double_voting_prevention() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);

    client.init(&admin);

    let circle_id = client.create_circle(
        &creator,
        &100_000,
        &15u32,
        &token,
        &86400u64,
        &0u32,
        &nft_contract,
    );

    client.join_circle(&proposer, &circle_id, &1u32, &None);
    client.join_circle(&voter, &circle_id, &1u32, &None);

    let title = String::from_str(&env, "Test proposal");
    let description = String::from_str(&env, "Test description");
    let execution_data = String::from_str(&env, "{}");

    let proposal_id = client.create_proposal(
        &proposer,
        &circle_id,
        &ProposalType::ChangeLateFee,
        &title,
        &description,
        &execution_data,
    );

    client.update_voting_power(&voter, &circle_id, &10_000_000_0);

    client.quadratic_vote(&voter, &proposal_id, &5u32, &QuadraticVoteChoice::For);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.quadratic_vote(&voter, &proposal_id, &3u32, &QuadraticVoteChoice::Against);
    }));
    assert!(result.is_err());
}

#[test]
fn test_quorum_requirement() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);

    client.init(&admin);

    let circle_id = client.create_circle(
        &creator,
        &100_000,
        &15u32,
        &token,
        &86400u64,
        &0u32,
        &nft_contract,
    );

    client.join_circle(&proposer, &circle_id, &1u32, &None);
    client.join_circle(&voter1, &circle_id, &1u32, &None);
    client.join_circle(&voter2, &circle_id, &1u32, &None);
    for _ in 0..12 {
        let m = Address::generate(&env);
        client.join_circle(&m, &circle_id, &1u32, &None);
    }

    let title = String::from_str(&env, "Test proposal");
    let description = String::from_str(&env, "Test description");
    let execution_data = String::from_str(&env, "{}");

    let proposal_id = client.create_proposal(
        &proposer,
        &circle_id,
        &ProposalType::ChangeLateFee,
        &title,
        &description,
        &execution_data,
    );

    client.update_voting_power(&voter1, &circle_id, &1_000_000_0);
    client.update_voting_power(&voter2, &circle_id, &1_000_000_0);

    client.quadratic_vote(&voter1, &proposal_id, &2u32, &QuadraticVoteChoice::For);

    let proposal = client.get_proposal(&proposal_id);
    assert!(!proposal.quorum_met);

    client.quadratic_vote(&voter2, &proposal_id, &3u32, &QuadraticVoteChoice::For);

    let updated_proposal = client.get_proposal(&proposal_id);
    assert_eq!(updated_proposal.for_votes, 13);
    assert!(updated_proposal.quorum_met);
}

#[test]
fn test_proposal_execution() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    let voter3 = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);

    client.init(&admin);

    let circle_id = client.create_circle(
        &creator,
        &100_000,
        &15u32,
        &token,
        &86400u64,
        &0u32,
        &nft_contract,
    );

    client.join_circle(&proposer, &circle_id, &1u32, &None);
    client.join_circle(&voter1, &circle_id, &1u32, &None);
    client.join_circle(&voter2, &circle_id, &1u32, &None);
    client.join_circle(&voter3, &circle_id, &1u32, &None);

    let title = String::from_str(&env, "Test proposal");
    let description = String::from_str(&env, "Test description");
    let execution_data = String::from_str(&env, "{}");

    let proposal_id = client.create_proposal(
        &proposer,
        &circle_id,
        &ProposalType::ChangeLateFee,
        &title,
        &description,
        &execution_data,
    );

    client.update_voting_power(&voter1, &circle_id, &10_000_000_0);
    client.update_voting_power(&voter2, &circle_id, &10_000_000_0);
    client.update_voting_power(&voter3, &circle_id, &10_000_000_0);

    client.quadratic_vote(&voter1, &proposal_id, &10u32, &QuadraticVoteChoice::For);
    client.quadratic_vote(&voter2, &proposal_id, &8u32, &QuadraticVoteChoice::For);
    client.quadratic_vote(&voter3, &proposal_id, &2u32, &QuadraticVoteChoice::Against);

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 700000);

    client.execute_proposal(&admin, &proposal_id);

    let proposal = client.get_proposal(&proposal_id);
    assert_eq!(proposal.status, ProposalStatus::Executed);
    assert_eq!(proposal.for_votes, 164);
    assert_eq!(proposal.against_votes, 4);

    let stats = client.get_proposal_stats(&circle_id);
    assert_eq!(stats.executed_proposals, 1);
}

#[test]
fn test_proposal_rejection_insufficient_majority() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);

    client.init(&admin);

    let circle_id = client.create_circle(
        &creator,
        &100_000,
        &15u32,
        &token,
        &86400u64,
        &0u32,
        &nft_contract,
    );

    client.join_circle(&proposer, &circle_id, &1u32, &None);
    client.join_circle(&voter1, &circle_id, &1u32, &None);
    client.join_circle(&voter2, &circle_id, &1u32, &None);

    let title = String::from_str(&env, "Test proposal");
    let description = String::from_str(&env, "Test description");
    let execution_data = String::from_str(&env, "{}");

    let proposal_id = client.create_proposal(
        &proposer,
        &circle_id,
        &ProposalType::ChangeLateFee,
        &title,
        &description,
        &execution_data,
    );

    client.update_voting_power(&voter1, &circle_id, &10_000_000_0);
    client.update_voting_power(&voter2, &circle_id, &10_000_000_0);

    client.quadratic_vote(&voter1, &proposal_id, &10u32, &QuadraticVoteChoice::For);
    client.quadratic_vote(&voter2, &proposal_id, &10u32, &QuadraticVoteChoice::Against);

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 700000);

    client.execute_proposal(&admin, &proposal_id);

    let proposal = client.get_proposal(&proposal_id);
    assert_eq!(proposal.status, ProposalStatus::Rejected);
    assert_eq!(proposal.for_votes, 100);
    assert_eq!(proposal.against_votes, 100);

    let stats = client.get_proposal_stats(&circle_id);
    assert_eq!(stats.rejected_proposals, 1);
}

#[test]
fn test_max_vote_weight_enforcement() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);

    client.init(&admin);

    let circle_id = client.create_circle(
        &creator,
        &100_000,
        &15u32,
        &token,
        &86400u64,
        &0u32,
        &nft_contract,
    );

    client.join_circle(&proposer, &circle_id, &1u32, &None);
    client.join_circle(&voter, &circle_id, &1u32, &None);

    let title = String::from_str(&env, "Test proposal");
    let description = String::from_str(&env, "Test description");
    let execution_data = String::from_str(&env, "{}");

    let proposal_id = client.create_proposal(
        &proposer,
        &circle_id,
        &ProposalType::ChangeLateFee,
        &title,
        &description,
        &execution_data,
    );

    client.update_voting_power(&voter, &circle_id, &100_000_000_0);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.quadratic_vote(&voter, &proposal_id, &150u32, &QuadraticVoteChoice::For);
    }));
    assert!(result.is_err());

    client.quadratic_vote(&voter, &proposal_id, &0u32, &QuadraticVoteChoice::For);

    let proposal = client.get_proposal(&proposal_id);
    assert_eq!(proposal.for_votes, 0);
}

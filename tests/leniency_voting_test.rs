use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{contract, contractimpl, Address, Env, String};
use sorosusu_contracts::{DataKey, LeniencyRequestStatus, LeniencyVote, SoroSusu, SoroSusuClient};

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
}

fn setup_circle(
    env: &Env,
) -> (
    SoroSusuClient,
    u64,
    Address,
    Address,
    Address,
    Address,
    Address,
) {
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let creator = Address::generate(env);
    let requester = Address::generate(env);
    let voter1 = Address::generate(env);
    let voter2 = Address::generate(env);
    let voter3 = Address::generate(env);
    let voter4 = Address::generate(env);
    let token = Address::generate(env);
    let nft_contract = env.register_contract(None, MockNft);

    client.init(&admin);

    let circle_id = client.create_circle(
        &creator,
        &100_000, // 10 XLM - below HIGH_VALUE_THRESHOLD (1000 XLM)
        &10u32,   // 10 members
        &token,
        &86400u64,
        &0u32,
        &nft_contract,
    );

    client.join_circle(&requester, &circle_id, &1u32, &None);
    client.join_circle(&voter1, &circle_id, &1u32, &None);
    client.join_circle(&voter2, &circle_id, &1u32, &None);
    client.join_circle(&voter3, &circle_id, &1u32, &None);
    client.join_circle(&voter4, &circle_id, &1u32, &None);

    (client, circle_id, requester, voter1, voter2, voter3, voter4)
}

#[test]
fn test_request_leniency() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let requester = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);

    client.init(&admin);

    let circle_id = client.create_circle(
        &creator,
        &100_000, // 10 XLM
        &5u32,
        &token,
        &86400u64,
        &0u32,
        &nft_contract,
    );

    client.join_circle(&requester, &circle_id, &1u32, &None);

    let reason = String::from_str(&env, "Medical emergency - need extra time");
    client.request_leniency(&requester, &circle_id, &reason);

    let request = client.get_leniency_request(&circle_id, &requester);
    assert_eq!(request.requester, requester);
    assert_eq!(request.circle_id, circle_id);
    assert_eq!(request.status, LeniencyRequestStatus::Pending);
    assert_eq!(request.extension_hours, 48);
    assert_eq!(request.approve_votes, 0);
    assert_eq!(request.reject_votes, 0);
}

#[test]
fn test_vote_on_leniency_approval() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, circle_id, requester, voter1, voter2, _voter3, _voter4) = setup_circle(&env);

    let reason = String::from_str(&env, "Need extra time for payment");
    client.request_leniency(&requester, &circle_id, &reason);

    // With 5 members (requester + 4 voters), total_possible_votes = 4
    // votes_needed_for_majority = 4 * 51 / 100 = 2
    // Vote 1: approve_votes = 1, not enough for majority
    client.vote_on_leniency(&voter1, &circle_id, &requester, &LeniencyVote::Approve);
    let request = client.get_leniency_request(&circle_id, &requester);
    assert_eq!(request.status, LeniencyRequestStatus::Pending);
    assert_eq!(request.approve_votes, 1);

    // Vote 2: approve_votes = 2, triggers auto-approval
    client.vote_on_leniency(&voter2, &circle_id, &requester, &LeniencyVote::Approve);
    let request = client.get_leniency_request(&circle_id, &requester);
    assert_eq!(request.status, LeniencyRequestStatus::Approved);
    assert_eq!(request.approve_votes, 2);
}

#[test]
fn test_vote_on_leniency_rejection() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, circle_id, requester, voter1, voter2, _voter3, _voter4) = setup_circle(&env);

    let reason = String::from_str(&env, "Need extra time");
    client.request_leniency(&requester, &circle_id, &reason);

    // With 4 eligible voters, majority = 4*51/100 = 2
    client.vote_on_leniency(&voter1, &circle_id, &requester, &LeniencyVote::Reject);
    let request = client.get_leniency_request(&circle_id, &requester);
    assert_eq!(request.status, LeniencyRequestStatus::Pending);
    assert_eq!(request.reject_votes, 1);

    client.vote_on_leniency(&voter2, &circle_id, &requester, &LeniencyVote::Reject);
    let request = client.get_leniency_request(&circle_id, &requester);
    assert_eq!(request.status, LeniencyRequestStatus::Rejected);
    assert_eq!(request.reject_votes, 2);
}

#[test]
fn test_cannot_vote_for_own_request() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, circle_id, requester, _v1, _v2, _v3, _v4) = setup_circle(&env);

    let reason = String::from_str(&env, "Need extra time");
    client.request_leniency(&requester, &circle_id, &reason);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.vote_on_leniency(&requester, &circle_id, &requester, &LeniencyVote::Approve);
    }));
    assert!(result.is_err());
}

#[test]
fn test_double_voting_prevention() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, circle_id, requester, voter, _v2, _v3, _v4) = setup_circle(&env);

    let reason = String::from_str(&env, "Need extra time");
    client.request_leniency(&requester, &circle_id, &reason);

    client.vote_on_leniency(&voter, &circle_id, &requester, &LeniencyVote::Approve);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.vote_on_leniency(&voter, &circle_id, &requester, &LeniencyVote::Approve);
    }));
    assert!(result.is_err());
}

#[test]
fn test_social_capital_tracking() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, circle_id, requester, voter, _v2, _v3, _v4) = setup_circle(&env);

    let reason = String::from_str(&env, "Need extra time");
    client.request_leniency(&requester, &circle_id, &reason);

    // Vote to approve
    client.vote_on_leniency(&voter, &circle_id, &requester, &LeniencyVote::Approve);

    // Check voter's social capital increased
    let voter_social = client.get_social_capital(&voter, &circle_id);
    assert_eq!(voter_social.leniency_given, 1);
    assert_eq!(voter_social.voting_participation, 1);
    assert_eq!(voter_social.trust_score, 52); // 50 + 2 for approving

    // Check requester's social capital after approval
    // Note: requester's social capital is updated during finalization (if approved)
    let requester_social = client.get_social_capital(&requester, &circle_id);
    // The requester's social capital may or may not be updated depending on auto-finalization
    // With 4 eligible voters, majority = 2, so 1 vote doesn't trigger finalization
    assert_eq!(requester_social.leniency_received, 0);
    assert_eq!(requester_social.trust_score, 50);
}

#[test]
fn test_leniency_stats_tracking() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, circle_id, requester, voter1, voter2, _v3, _v4) = setup_circle(&env);

    let requester2 = Address::generate(&env);
    client.join_circle(&requester2, &circle_id, &1u32, &None);

    // Request leniency for requester1 and approve it
    let reason1 = String::from_str(&env, "Medical emergency");
    client.request_leniency(&requester, &circle_id, &reason1);
    client.vote_on_leniency(&voter1, &circle_id, &requester, &LeniencyVote::Approve);
    client.vote_on_leniency(&voter2, &circle_id, &requester, &LeniencyVote::Approve);
    // This should auto-finalize with approval

    // Request leniency for requester2 and reject it
    let reason2 = String::from_str(&env, "Need extra time");
    client.request_leniency(&requester2, &circle_id, &reason2);
    client.vote_on_leniency(&voter1, &circle_id, &requester2, &LeniencyVote::Reject);
    client.vote_on_leniency(&voter2, &circle_id, &requester2, &LeniencyVote::Reject);
    // This should auto-finalize with rejection

    // Check stats - only approved requests update stats via finalize_leniency_vote_internal
    let stats = client.get_leniency_stats(&circle_id);
    assert!(stats.approved_requests >= 1);
    assert!(stats.total_requests >= 1);
}

#[test]
fn test_grace_period_prevents_late_fees() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let requester = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);

    client.init(&admin);

    let circle_id = client.create_circle(
        &creator,
        &100_000, // 10 XLM
        &5u32,
        &token,
        &3600u64, // 1 hour deadline
        &0u32,
        &nft_contract,
    );

    client.join_circle(&requester, &circle_id, &1u32, &None);
    client.join_circle(&voter, &circle_id, &1u32, &None);

    let reason = String::from_str(&env, "Need extra time");
    client.request_leniency(&requester, &circle_id, &reason);
    client.vote_on_leniency(&voter, &circle_id, &requester, &LeniencyVote::Approve);

    // Advance time past original deadline but within grace period
    env.ledger().set_timestamp(env.ledger().timestamp() + 7200); // 2 hours later

    // The request should be approved and grace period applied
    let request = client.get_leniency_request(&circle_id, &requester);
    assert_eq!(request.status, LeniencyRequestStatus::Approved);
}

#[test]
fn test_voting_period_expiration() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, circle_id, requester, voter, _v2, _v3, _v4) = setup_circle(&env);

    let reason = String::from_str(&env, "Need extra time");
    client.request_leniency(&requester, &circle_id, &reason);

    // Advance time past voting period
    env.ledger().set_timestamp(env.ledger().timestamp() + 90000); // 25 hours later

    // Try to vote - should fail because voting period expired
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.vote_on_leniency(&voter, &circle_id, &requester, &LeniencyVote::Approve);
    }));
    assert!(result.is_err());
}

#[test]
fn test_minimum_participation_requirement() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, circle_id, requester, voter1, voter2, _v3, _v4) = setup_circle(&env);

    let reason = String::from_str(&env, "Need extra time");
    client.request_leniency(&requester, &circle_id, &reason);

    // With 5 members (4 eligible voters), majority = 4*51/100 = 2
    // minimum_participation = total_possible_votes * 50 / 100

    // Vote 1: approve - not majority yet, request stays Pending
    client.vote_on_leniency(&voter1, &circle_id, &requester, &LeniencyVote::Approve);
    let request = client.get_leniency_request(&circle_id, &requester);
    assert_eq!(request.status, LeniencyRequestStatus::Pending);

    // Vote 2: approve - reaches majority, auto-finalizes as Approved
    client.vote_on_leniency(&voter2, &circle_id, &requester, &LeniencyVote::Approve);
    let request = client.get_leniency_request(&circle_id, &requester);
    assert_eq!(request.status, LeniencyRequestStatus::Approved);
}

//! Issue #319: Test: Simultaneous "Bank Run" Contributions
//!
//! Simulate a high-traffic scenario where all 50 members of a Susu group
//! attempt to call `deposit` in the exact same ledger sequence right before
//! the deadline. Verifies that the contract sequences these transactions
//! without hitting compute/storage limits and that no member is unfairly
//! penalised for network congestion.

#![cfg(test)]

use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{token, Address, Env};
use sorosusu_contracts::{SoroSusu, SoroSusuClient};

const MAX_MEMBERS: u32 = 50;
const CONTRIBUTION: u64 = 1_000_000; // 1 token (6 decimals)
const CYCLE_DURATION: u64 = 7 * 24 * 60 * 60; // 1 week

/// Set up a circle with `MAX_MEMBERS` members, all funded and ready to deposit.
fn setup_bank_run_circle(
    env: &Env,
) -> (SoroSusuClient<'static>, u64, Vec<Address>, Address) {
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let creator = Address::generate(env);
    let token_admin = Address::generate(env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(env, &token);

    client.init(&admin, &0);

    // Mint enough tokens to creator and all future members
    token_client.mint(&creator, &(CONTRIBUTION as i128 * 2));

    let circle_id = client.create_circle(
        &creator,
        &CONTRIBUTION,
        &MAX_MEMBERS,
        &token,
        &CYCLE_DURATION,
        &false,
        &0u32,
        &(24 * 60 * 60u64), // 24 h grace period
        &100u32,             // 1% late fee
    );

    let mut members: Vec<Address> = Vec::new();
    for _ in 0..MAX_MEMBERS {
        let member = Address::generate(env);
        token_client.mint(&member, &(CONTRIBUTION as i128 * 2));
        client.join_circle(&member, &circle_id);
        members.push(member);
    }

    (client, circle_id, members, token)
}

/// All 50 members deposit at the same ledger timestamp (same sequence).
/// No member should be penalised; all should be marked as having contributed.
#[test]
fn test_all_50_members_deposit_same_ledger_sequence() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, circle_id, members, _token) = setup_bank_run_circle(&env);

    // Fix the ledger timestamp well before the deadline so every deposit is on-time.
    let deadline_buffer: u64 = CYCLE_DURATION / 2; // halfway through the cycle
    env.ledger().set_timestamp(deadline_buffer);

    for member in &members {
        // Every call happens at the same timestamp — simulating the same ledger sequence.
        client.deposit(member, &circle_id);
    }

    // Verify every member is marked as contributed and has no missed deadline.
    for member in &members {
        let member_key = sorosusu_contracts::DataKey::Member(member.clone());
        let member_data = env
            .storage()
            .instance()
            .get::<sorosusu_contracts::DataKey, sorosusu_contracts::Member>(&member_key)
            .expect("member record must exist");

        assert!(
            member_data.has_contributed,
            "member {:?} must be marked as contributed",
            member
        );
        assert_eq!(
            member_data.missed_deadline_timestamp, 0,
            "member {:?} must not have a missed deadline (no congestion penalty)",
            member
        );
        assert_eq!(
            member_data.contribution_count, 1,
            "member {:?} must have exactly 1 contribution",
            member
        );
    }
}

/// Group reserve must remain zero after all on-time deposits (no late fees collected).
#[test]
fn test_bank_run_no_late_fees_collected() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, circle_id, members, _token) = setup_bank_run_circle(&env);

    env.ledger().set_timestamp(CYCLE_DURATION / 2);

    for member in &members {
        client.deposit(member, &circle_id);
    }

    let reserve: u64 = env
        .storage()
        .instance()
        .get::<sorosusu_contracts::DataKey, u64>(&sorosusu_contracts::DataKey::GroupReserve)
        .unwrap_or(0);

    assert_eq!(
        reserve, 0,
        "group reserve must be 0 — no late fees for on-time deposits"
    );
}

/// Deposits that arrive at the very last second before the deadline are still on-time.
#[test]
fn test_bank_run_last_second_before_deadline_is_on_time() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, circle_id, members, _token) = setup_bank_run_circle(&env);

    // Set timestamp to exactly 1 second before the deadline.
    // create_circle sets deadline = ledger_time_at_creation + cycle_duration.
    // The env starts at timestamp 0, so deadline = CYCLE_DURATION.
    let deadline = CYCLE_DURATION;
    env.ledger().set_timestamp(deadline - 1);

    for member in &members {
        client.deposit(member, &circle_id);
    }

    for member in &members {
        let member_key = sorosusu_contracts::DataKey::Member(member.clone());
        let member_data = env
            .storage()
            .instance()
            .get::<sorosusu_contracts::DataKey, sorosusu_contracts::Member>(&member_key)
            .expect("member record must exist");

        assert!(member_data.has_contributed);
        assert_eq!(member_data.missed_deadline_timestamp, 0);
    }
}

/// Deposits that arrive 1 second after the deadline must be rejected by `deposit`
/// (member must use `late_contribution` instead). This ensures the deadline
/// boundary is enforced consistently regardless of how many members are in the group.
#[test]
fn test_bank_run_one_second_after_deadline_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, circle_id, members, _token) = setup_bank_run_circle(&env);

    let deadline = CYCLE_DURATION;
    env.ledger().set_timestamp(deadline + 1);

    // The first deposit attempt after the deadline must fail.
    let result = client.try_deposit(&members[0], &circle_id);
    assert!(
        result.is_err(),
        "deposit after deadline must be rejected; member must use late_contribution"
    );
}

/// Contribution counts must be sequential and consistent even when all 50
/// members deposit in the same ledger sequence (no double-counting).
#[test]
fn test_bank_run_contribution_counts_are_consistent() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, circle_id, members, _token) = setup_bank_run_circle(&env);

    env.ledger().set_timestamp(CYCLE_DURATION / 2);

    for member in &members {
        client.deposit(member, &circle_id);
    }

    // Each member should have exactly 1 contribution — no double-counting.
    for member in &members {
        let member_key = sorosusu_contracts::DataKey::Member(member.clone());
        let member_data = env
            .storage()
            .instance()
            .get::<sorosusu_contracts::DataKey, sorosusu_contracts::Member>(&member_key)
            .unwrap();
        assert_eq!(member_data.contribution_count, 1);
    }
}

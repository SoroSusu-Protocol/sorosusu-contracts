//! Issue #324: Implement Appeals Timelock for Slashed Collateral
//!
//! Before slashed collateral is redistributed to victims it must sit in a
//! PendingSlash vault for 72 hours. This provides a critical window for the
//! penalised user to submit an emergency appeal to the global DAO if they
//! believe the group colluded against them.

#![cfg(test)]

use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{token, Address, Env};
use sorosusu_contracts::{
    DataKey, PendingSlashRecord, SoroSusu, SoroSusuClient, APPEALS_TIMELOCK_SECS,
};

const CONTRIBUTION: u64 = 1_000_000;
const CYCLE: u64 = 7 * 24 * 60 * 60;

fn setup(env: &Env) -> (SoroSusuClient<'static>, Address, Address, Address, u64) {
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let creator = Address::generate(env);
    let defaulter = Address::generate(env);
    let token_admin = Address::generate(env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(env, &token);

    client.init(&admin, &0);

    token_client.mint(&creator, &(CONTRIBUTION as i128 * 2));
    token_client.mint(&defaulter, &(CONTRIBUTION as i128 * 2));

    let circle_id = client.create_circle(
        &creator,
        &CONTRIBUTION,
        &5u32,
        &token,
        &CYCLE,
        &false,
        &0u32,
        &(24 * 60 * 60u64),
        &100u32,
    );

    client.join_circle(&defaulter, &circle_id);

    // Simulate a late payment so the member gets a missed_deadline_timestamp,
    // then advance past the grace period and execute default.
    env.ledger().set_timestamp(CYCLE + 1); // past deadline
    // Mark missed deadline by attempting deposit (will fail, but sets the flag).
    let _ = client.try_deposit(&defaulter, &circle_id);

    // Advance past grace period (24 h) and execute default.
    env.ledger().set_timestamp(CYCLE + 1 + 24 * 60 * 60 + 1);
    client.execute_default(&circle_id, &defaulter).unwrap();

    // Seed the group reserve so slash_collateral has funds to move.
    env.storage()
        .instance()
        .set(&DataKey::GroupReserve, &(CONTRIBUTION * 10));

    (client, admin, defaulter, token, circle_id)
}

// ---------------------------------------------------------------------------
// slash_collateral
// ---------------------------------------------------------------------------

/// After slash_collateral the funds must be in the PendingSlash vault, not yet
/// in the group reserve.
#[test]
fn test_slash_moves_funds_to_pending_vault() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, defaulter, _token, circle_id) = setup(&env);

    let reserve_before: u64 = env
        .storage()
        .instance()
        .get::<DataKey, u64>(&DataKey::GroupReserve)
        .unwrap_or(0);

    client.slash_collateral(&circle_id, &defaulter).unwrap();

    // Reserve must have decreased by the contribution amount.
    let reserve_after: u64 = env
        .storage()
        .instance()
        .get::<DataKey, u64>(&DataKey::GroupReserve)
        .unwrap_or(0);
    assert_eq!(reserve_after, reserve_before - CONTRIBUTION);

    // PendingSlash vault must now hold the slashed amount.
    let record: PendingSlashRecord = env
        .storage()
        .instance()
        .get::<DataKey, PendingSlashRecord>(&DataKey::PendingSlash(circle_id, defaulter.clone()))
        .expect("PendingSlash record must exist after slash");

    assert_eq!(record.amount, CONTRIBUTION);
}

/// The PendingSlash record must store the correct slash timestamp so the
/// 72-hour window can be enforced.
#[test]
fn test_slash_records_correct_timestamp() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, defaulter, _token, circle_id) = setup(&env);

    let slash_time = env.ledger().timestamp();
    client.slash_collateral(&circle_id, &defaulter).unwrap();

    let record: PendingSlashRecord = env
        .storage()
        .instance()
        .get::<DataKey, PendingSlashRecord>(&DataKey::PendingSlash(circle_id, defaulter))
        .unwrap();

    assert_eq!(record.slashed_at, slash_time);
}

/// Slashing a member who has not defaulted must return an error.
#[test]
fn test_slash_non_defaulted_member_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, _defaulter, _token, circle_id) = setup(&env);

    let innocent = Address::generate(&env);
    let result = client.try_slash_collateral(&circle_id, &innocent);
    assert!(result.is_err(), "slashing a non-defaulted member must fail");
}

// ---------------------------------------------------------------------------
// release_pending_slash — timelock enforcement
// ---------------------------------------------------------------------------

/// Attempting to release within the 72-hour window must be rejected.
#[test]
fn test_release_before_timelock_expires_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, defaulter, _token, circle_id) = setup(&env);

    client.slash_collateral(&circle_id, &defaulter).unwrap();

    // Advance time by less than 72 hours.
    let slash_time = env.ledger().timestamp();
    env.ledger()
        .set_timestamp(slash_time + APPEALS_TIMELOCK_SECS - 1);

    let result = client.try_release_pending_slash(&circle_id, &defaulter);
    assert!(
        result.is_err(),
        "release must be blocked while the appeal window is open"
    );
}

/// Releasing exactly at the 72-hour boundary must succeed.
#[test]
fn test_release_at_exact_timelock_boundary_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, defaulter, _token, circle_id) = setup(&env);

    client.slash_collateral(&circle_id, &defaulter).unwrap();

    let slash_time = env.ledger().timestamp();
    env.ledger()
        .set_timestamp(slash_time + APPEALS_TIMELOCK_SECS);

    client
        .release_pending_slash(&circle_id, &defaulter)
        .unwrap();
}

/// After a successful release the slashed amount must be added to the group reserve
/// and the PendingSlash record must be removed.
#[test]
fn test_release_after_timelock_redistributes_to_reserve() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, defaulter, _token, circle_id) = setup(&env);

    client.slash_collateral(&circle_id, &defaulter).unwrap();

    let reserve_after_slash: u64 = env
        .storage()
        .instance()
        .get::<DataKey, u64>(&DataKey::GroupReserve)
        .unwrap_or(0);

    let slash_time = env.ledger().timestamp();
    env.ledger()
        .set_timestamp(slash_time + APPEALS_TIMELOCK_SECS + 1);

    client
        .release_pending_slash(&circle_id, &defaulter)
        .unwrap();

    // Reserve must have increased by the slashed amount.
    let reserve_final: u64 = env
        .storage()
        .instance()
        .get::<DataKey, u64>(&DataKey::GroupReserve)
        .unwrap_or(0);
    assert_eq!(reserve_final, reserve_after_slash + CONTRIBUTION);

    // PendingSlash record must be gone.
    let record = env
        .storage()
        .instance()
        .get::<DataKey, PendingSlashRecord>(&DataKey::PendingSlash(circle_id, defaulter));
    assert!(
        record.is_none(),
        "PendingSlash record must be removed after release"
    );
}

/// Releasing a pending slash that does not exist must return an error.
#[test]
fn test_release_nonexistent_pending_slash_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, _defaulter, _token, circle_id) = setup(&env);

    let random = Address::generate(&env);
    let result = client.try_release_pending_slash(&circle_id, &random);
    assert!(result.is_err(), "releasing a non-existent vault must fail");
}

/// Double-release must fail — the vault is cleared on first release.
#[test]
fn test_double_release_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, defaulter, _token, circle_id) = setup(&env);

    client.slash_collateral(&circle_id, &defaulter).unwrap();

    let slash_time = env.ledger().timestamp();
    env.ledger()
        .set_timestamp(slash_time + APPEALS_TIMELOCK_SECS + 1);

    client
        .release_pending_slash(&circle_id, &defaulter)
        .unwrap();

    let result = client.try_release_pending_slash(&circle_id, &defaulter);
    assert!(result.is_err(), "double-release must fail");
}

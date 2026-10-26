#![cfg(test)]
//! Automated Rent-Drip: authorize_rent_drip / revoke_rent_drip + claim_pot
//! routing a winner's payout to a LeaseFlow LeaseInstance contract (#105).

use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Ledger as _},
    token, Address, Env,
};
use sorosusu_contracts::{SoroSusu, SoroSusuClient};

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
}

/// Records the last `pay_rent` call it received, so tests can assert the
/// LeaseInstance was actually invoked (and with what amount) rather than
/// just checking the token balance moved.
#[contract]
pub struct MockLeaseInstance;

#[contractimpl]
impl MockLeaseInstance {
    pub fn pay_rent(env: Env, tenant: Address, token: Address, amount: i128) {
        env.storage().instance().set(&("last_tenant",), &tenant);
        env.storage().instance().set(&("last_token",), &token);
        env.storage().instance().set(&("last_amount",), &amount);
    }

    pub fn last_amount(env: Env) -> i128 {
        env.storage().instance().get(&("last_amount",)).unwrap_or(0)
    }

    pub fn last_tenant(env: Env) -> Option<Address> {
        env.storage().instance().get(&("last_tenant",))
    }
}

struct Setup {
    env: Env,
    client: SoroSusuClient<'static>,
    token_client: token::Client<'static>,
    creator: Address,
    circle_id: u64,
}

/// Builds a 2-member circle, has both members (including the creator) join
/// and deposit, and finalizes the round so the pot is ready to claim.
/// `claim_pot` always sets `current_pot_recipient` to the circle creator
/// (noted in `finalize_round` as simplified pending per-member rotation), so
/// the creator is the winner in every test here.
fn setup_finalized_circle() -> Setup {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user2 = Address::generate(&env);

    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    client.init(&admin);

    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);
    let token_client = token::Client::new(&env, &token_id);

    token_admin_client.mint(&creator, &10_000);
    token_admin_client.mint(&user2, &10_000);

    let nft_id = env.register_contract(None, MockNft);

    let contribution_amount: i128 = 1000;
    let circle_id = client.create_circle(
        &creator,
        &contribution_amount,
        &2,
        &token_id,
        &86_400,
        &0, // no insurance fee, to keep payout math simple
        &nft_id,
    );

    client.join_circle(&creator, &circle_id, &1, &None);
    client.join_circle(&user2, &circle_id, &1, &None);

    client.deposit(&creator, &circle_id);
    client.deposit(&user2, &circle_id);

    client.finalize_round(&creator, &circle_id);

    // Payout window opens 1 hour after finalize_round.
    env.ledger().with_mut(|li| li.timestamp += 3601);

    Setup {
        env,
        client,
        token_client,
        creator,
        circle_id,
    }
}

#[test]
fn claim_pot_pays_the_winner_directly_without_authorization() {
    let setup = setup_finalized_circle();
    let balance_before = setup.token_client.balance(&setup.creator);

    setup.client.claim_pot(&setup.creator, &setup.circle_id);

    // 2 members * 1000 contribution = 2000 pot.
    assert_eq!(
        setup.token_client.balance(&setup.creator),
        balance_before + 2000
    );
}

#[test]
fn authorize_rent_drip_routes_the_payout_to_the_lease_instance() {
    let setup = setup_finalized_circle();
    let lease_instance_id = setup.env.register_contract(None, MockLeaseInstance);
    let lease_client = MockLeaseInstanceClient::new(&setup.env, &lease_instance_id);

    let winner_balance_before = setup.token_client.balance(&setup.creator);

    setup
        .client
        .authorize_rent_drip(&setup.creator, &setup.circle_id, &lease_instance_id);
    setup.client.claim_pot(&setup.creator, &setup.circle_id);

    // The winner's own wallet balance is unaffected; the pot went to the
    // LeaseInstance contract instead.
    assert_eq!(
        setup.token_client.balance(&setup.creator),
        winner_balance_before
    );
    assert_eq!(setup.token_client.balance(&lease_instance_id), 2000);

    // And the LeaseInstance actually recorded the rent payment.
    assert_eq!(lease_client.last_amount(), 2000);
    assert_eq!(lease_client.last_tenant(), Some(setup.creator.clone()));
}

#[test]
fn revoke_rent_drip_restores_direct_payout_to_the_winner() {
    let setup = setup_finalized_circle();
    let lease_instance_id = setup.env.register_contract(None, MockLeaseInstance);

    setup
        .client
        .authorize_rent_drip(&setup.creator, &setup.circle_id, &lease_instance_id);
    setup
        .client
        .revoke_rent_drip(&setup.creator, &setup.circle_id);

    let winner_balance_before = setup.token_client.balance(&setup.creator);
    setup.client.claim_pot(&setup.creator, &setup.circle_id);

    assert_eq!(
        setup.token_client.balance(&setup.creator),
        winner_balance_before + 2000
    );
    assert_eq!(setup.token_client.balance(&lease_instance_id), 0);
}

#[test]
#[should_panic(expected = "Not a member of this circle")]
fn authorize_rent_drip_rejects_a_non_member() {
    let setup = setup_finalized_circle();
    let non_member = Address::generate(&setup.env);
    let lease_instance_id = setup.env.register_contract(None, MockLeaseInstance);

    setup
        .client
        .authorize_rent_drip(&non_member, &setup.circle_id, &lease_instance_id);
}

#[test]
#[should_panic(expected = "Circle not found")]
fn authorize_rent_drip_rejects_an_unknown_circle() {
    let setup = setup_finalized_circle();
    let lease_instance_id = setup.env.register_contract(None, MockLeaseInstance);

    setup
        .client
        .authorize_rent_drip(&setup.creator, &(setup.circle_id + 999), &lease_instance_id);
}

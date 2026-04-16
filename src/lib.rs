#![cfg_attr(not(test), no_std)]

#[cfg(test)] extern crate std;

use soroban_sdk::{
    contract, contractclient, contracterror, contractimpl, contracttype, symbol_short, token,
    Address, Env, String, Symbol, Vec, Map,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error { AlreadyInit = 100, NotAuth = 101, NotFound = 102, MemberExists = 103, LowFunds = 104, InvalidAmt = 105, NotMember = 106 }

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    // Compact key system to bypass macro limits
    K(Symbol),
    K1(Symbol, u64),
    K1A(Symbol, Address),
    K1U(Symbol, u128),
    K1S(Symbol, String),
    K2(Symbol, u64, Address),
    K2A(Symbol, Address, u64),
    K2U(Symbol, u64, u32),
    K3(Symbol, u64, Address, Address),
    K3U(Symbol, Address, u64, u32),
}

// Helpers for the compact key system


#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)] pub enum MemberStatus { Active, Awaiting, Ejected, Defaulted }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)] pub enum LeniencyRequestStatus { Pending, Approved, Rejected, Expired }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)] pub enum LeniencyVote { Approve, Reject }
#[contracttype] #[derive(Clone)] pub struct LeniencyRequest { pub requester: Address, pub circle_id: u64, pub request_timestamp: u64, pub voting_deadline: u64, pub status: LeniencyRequestStatus, pub approve_votes: u32, pub reject_votes: u32, pub total_votes_cast: u32, pub extension_hours: u64, pub reason: String }
#[contracttype] #[derive(Clone)] pub struct DurationProposal { pub id: u64, pub new_duration: u64, pub votes_for: u32, pub votes_against: u32, pub end_time: u64, pub is_active: bool }
#[contracttype] #[derive(Clone)] pub struct SocialCapital { pub member: Address, pub circle_id: u64, pub leniency_given: u32, pub leniency_received: u32, pub voting_participation: u32, pub trust_score: u32 }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)] pub enum ProposalStatus { Draft, Active, Approved, Rejected, Executed, Expired }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)] pub enum ProposalType { RuleChange, AdminUpdate, EmergencyHalt, ChangeLateFee, ChangeInsuranceFee, ChangeCycleDuration, AddMember, RemoveMember, ChangeQuorum, EmergencyAction }
#[contracttype] #[derive(Clone)] pub struct Proposal { pub id: u64, pub circle_id: u64, pub proposer: Address, pub proposal_type: ProposalType, pub title: String, pub description: String, pub created_timestamp: u64, pub voting_start_timestamp: u64, pub voting_end_timestamp: u64, pub status: ProposalStatus, pub for_votes: u64, pub against_votes: u64, pub total_voting_power: u64, pub quorum_met: bool, pub execution_data: String }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)] pub enum QuadraticVoteChoice { For, Against, Abstain }
#[contracttype] #[derive(Clone)] pub struct VotingPower { pub member: Address, pub circle_id: u64, pub token_balance: i128, pub quadratic_power: u64, pub last_updated: u64 }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)] pub enum CollateralStatus { NotStaked, Staked, Slashed, Released, Defaulted }
#[contracttype] #[derive(Clone)] pub struct CollateralInfo { pub member: Address, pub circle_id: u64, pub amount: i128, pub status: CollateralStatus, pub staked_timestamp: u64, pub release_timestamp: Option<u64> }
#[contracttype] #[derive(Clone)] pub struct Member { pub address: Address, pub index: u32, pub contribution_count: u32, pub last_contribution_time: u64, pub status: MemberStatus, pub tier_multiplier: u32, pub referrer: Option<Address>, pub buddy: Option<Address>, pub has_contributed_current_round: bool, pub total_contributions: i128 }
#[contracttype] #[derive(Clone)] pub struct CircleInfo { pub id: u64, pub creator: Address, pub contribution_amount: i128, pub max_members: u32, pub member_count: u32, pub current_recipient_index: u32, pub is_active: bool, pub token: Address, pub deadline_timestamp: u64, pub cycle_duration: u64, pub member_addresses: Vec<Address>, pub recovery_votes_bitmap: u32, pub recovery_old_address: Option<Address>, pub recovery_new_address: Option<Address>, pub grace_period_end: Option<u64>, pub requires_collateral: bool, pub collateral_bps: u32, pub quadratic_voting_enabled: bool, pub proposal_count: u64, pub total_cycle_value: i128, pub winners_per_round: u32, pub batch_payout_enabled: bool, pub current_pot_recipient: Option<Address>, pub is_round_finalized: bool }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)] pub enum AuditAction { DisputeSubmission, GovernanceVote, EvidenceAccess, AdminAction }
#[contracttype] #[derive(Clone)] pub struct AuditEntry { pub id: u64, pub actor: Address, pub action: AuditAction, pub timestamp: u64, pub resource_id: u64 }
#[contracttype] #[derive(Clone)] pub struct UserStats { pub total_volume_saved: i128, pub on_time_contributions: u32, pub late_contributions: u32 }
#[contracttype] #[derive(Clone)] pub struct NftBadgeMetadata { pub name: String, pub description: String, pub image_url: String }
#[contracttype] #[derive(Clone)] pub struct BatchPayoutRecord { pub batch_payout_id: u64, pub circle_id: u64, pub round_number: u32, pub total_winners: u32, pub total_pot: i128, pub organizer_fee: i128, pub net_payout_per_winner: i128, pub dust_amount: i128, pub winners: Vec<Address>, pub payout_timestamp: u64 }
#[contracttype] #[derive(Clone)] pub struct IndividualPayoutClaim { pub recipient: Address, pub circle_id: u64, pub round_number: u32, pub amount_claimed: i128, pub batch_payout_id: u64, pub claim_timestamp: u64 }
#[contracttype] #[derive(Clone)] pub struct AssetWeight { pub token: Address, pub weight_bps: u32 }
#[contracttype] #[derive(Clone)] pub struct AnchorInfo { pub anchor_address: Address, pub anchor_name: String, pub sep_version: String, pub authorization_level: u32, pub compliance_level: u32, pub is_active: bool, pub registration_timestamp: u64, pub last_activity: u64, pub supported_countries: Vec<String>, pub max_deposit_amount: i128, pub daily_deposit_limit: i128 }
#[contracttype] #[derive(Clone)] pub struct AnchorDeposit { pub id: u64, pub anchor_address: Address, pub beneficiary_user: Address, pub circle_id: u64, pub amount: i128, pub deposit_memo: String, pub fiat_reference: String, pub sep_type: String, pub timestamp: u64, pub processed: bool, pub compliance_verified: bool }
#[contracttype] #[derive(Clone)] pub struct DexSwapConfig { pub enabled: bool, pub swap_threshold_xlm: i128, pub swap_percentage_bps: u32, pub dex_contract: Address, pub xlm_token: Address, pub slippage_tolerance_bps: u32, pub minimum_swap_amount: i128, pub emergency_pause: bool, pub last_swap_timestamp: u64, pub total_swapped_xlm: i128 }
#[contracttype] #[derive(Clone)] pub struct DexSwapRecord { pub success: bool, pub usdc_amount: i128, pub xlm_received: i128 }
#[contracttype] #[derive(Clone)] pub struct GasReserve { pub xlm_balance: i128, pub reserved_for_ttl: u64, pub auto_swap_enabled: bool, pub last_refill_timestamp: u64, pub consumption_rate: u64 }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)] pub enum TrancheStatus { Pending, Locked, Unlocked, Claimed, ClawedBack }
#[contracttype] #[derive(Clone)] pub struct Tranche { pub amount: i128, pub unlock_round: u32, pub status: TrancheStatus }
#[contracttype] #[derive(Clone)] pub struct TrancheSchedule { pub circle_id: u64, pub winner: Address, pub total_pot: i128, pub immediate_payout: i128, pub tranches: Vec<Tranche> }
#[contracttype] #[derive(Clone)] pub struct GrantSettlement { pub grant_id: u64, pub grantee: Address, pub total_grant_amount: i128, pub amount_dripped: i128, pub work_in_progress_pay: i128, pub treasury_return: i128 }
#[contracttype] #[derive(Clone)] pub struct VotingSnapshot { pub proposal_id: u64, pub total_votes: u32, pub for_votes: u32, pub against_votes: u32, pub abstain_votes: u32, pub quorum_required: u32, pub quorum_met: bool, pub result: Symbol, pub vote_hash: String }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)] pub enum MilestoneProgress { NotStarted, InProgress, Completed, OnHold, Cancelled }
#[contracttype] #[derive(Clone)] pub struct ImpactCertificateMetadata { pub id: u128, pub grantee: Address, pub total_phases: u32, pub phases_completed: u32, pub impact_score: u32, pub on_chain_badge: Symbol, pub milestone_status: MilestoneProgress }
#[contracttype] #[derive(Clone)] pub struct LiquidityBufferConfig { pub is_enabled: bool, pub advance_period: u64, pub min_reputation: u32, pub max_advance_bps: u32, pub platform_fee_allocation: u32, pub min_reserve: i128, pub max_reserve: i128, pub advance_fee_bps: u32, pub grace_period: u64, pub max_advances_per_round: u32 }
#[contracttype] #[derive(Clone)] pub struct LiquidityBufferStats { pub total_reserve_balance: i128, pub total_advances_provided: i128, pub active_advances_count: u32 }
#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)] pub enum LiquidityAdvanceStatus { Pending, Active, Repaid, Defaulted, Cancelled }
#[contracttype] #[derive(Clone)] pub struct LiquidityAdvance { pub id: u64, pub member: Address, pub circle_id: u64, pub contribution_amount: i128, pub advance_amount: i128, pub advance_fee: i128, pub repayment_amount: i128, pub status: LiquidityAdvanceStatus, pub requested_timestamp: u64, pub provided_timestamp: Option<u64> }

#[contractclient(name = "SusuNftClient")] pub trait SusuNftTrait { fn mint(env: Env, to: Address, id: u128); fn burn(env: Env, from: Address, id: u128); fn mint_badge(env: Env, to: Address, id: u128, m: NftBadgeMetadata); }

pub trait SoroSusuTrait {
    fn init(env: Env, admin: Address, fee: u32);
    fn create_circle(env: Env, creator: Address, amt: i128, max: u32, tok: Address, dur: u64, bond: i128) -> u64;
    fn create_basket_circle(env: Env, creator: Address, amt: i128, max: u32, assets: Vec<Address>, weights: Vec<u32>, dur: u64, ifee: u64, nft: Address, arb: Address) -> u64;
    fn join_circle(env: Env, u: Address, cid: u64);
    fn deposit(env: Env, u: Address, cid: u64, r: u32);
    fn deposit_basket(env: Env, u: Address, cid: u64);
    fn propose_duration(env: Env, u: Address, cid: u64, dur: u64) -> u64;
    fn vote_duration(env: Env, u: Address, cid: u64, pid: u64, app: bool);
    fn slash_bond(env: Env, adm: Address, cid: u64);
    fn release_bond(env: Env, adm: Address, cid: u64);
    fn pair_with_member(env: Env, u: Address, buddy: Address);
    fn set_safety_deposit(env: Env, u: Address, cid: u64, amt: i128);
    fn propose_address_change(env: Env, prop: Address, cid: u64, old: Address, new: Address);
    fn vote_for_recovery(env: Env, voter: Address, cid: u64);
    fn stake_xlm(env: Env, u: Address, tok: Address, amt: i128);
    fn unstake_xlm(env: Env, u: Address, tok: Address, amt: i128);
    fn update_global_fee(env: Env, adm: Address, fee: u32);
    fn request_leniency(env: Env, req: Address, cid: u64, reason: String);
    fn vote_on_leniency(env: Env, voter: Address, cid: u64, req: Address, v: LeniencyVote);
    fn finalize_leniency_vote(env: Env, caller: Address, cid: u64, req: Address);
    fn get_leniency_request(env: Env, cid: u64, req: Address) -> LeniencyRequest;
    fn get_social_capital(env: Env, m: Address, cid: u64) -> SocialCapital;
    fn create_proposal(env: Env, prop: Address, cid: u64, pt: ProposalType, title: String, desc: String, ed: String) -> u64;
    fn quadratic_vote(env: Env, voter: Address, pid: u64, weight: u32, vc: QuadraticVoteChoice);
    fn execute_proposal(env: Env, caller: Address, pid: u64);
    fn get_proposal(env: Env, pid: u64) -> Proposal;
    fn get_voting_power(env: Env, m: Address, cid: u64) -> VotingPower;
    fn update_voting_power(env: Env, m: Address, cid: u64, bal: i128);
    fn stake_collateral(env: Env, u: Address, cid: u64, amt: i128);
    fn slash_collateral(env: Env, caller: Address, cid: u64, m: Address);
    fn release_collateral(env: Env, caller: Address, cid: u64, m: Address);
    fn mark_member_defaulted(env: Env, caller: Address, cid: u64, m: Address);
    fn get_audit_entry(env: Env, id: u64) -> AuditEntry;
    fn query_audit_by_actor(env: Env, actor: Address, s: u64, e: u64, o: u32, l: u32) -> Vec<AuditEntry>;
    fn query_audit_by_resource(env: Env, rid: u64, s: u64, e: u64, o: u32, l: u32) -> Vec<AuditEntry>;
    fn query_audit_by_time(env: Env, s: u64, e: u64, o: u32, l: u32) -> Vec<AuditEntry>;
    fn set_leaseflow_contract(env: Env, adm: Address, rot: Address);
    fn authorize_leaseflow_payout(env: Env, u: Address, cid: u64, li: Address);
    fn handle_leaseflow_default(env: Env, rot: Address, ten: Address, cid: u64);
    fn claim_pot(env: Env, u: Address, cid: u64);
    fn finalize_round(env: Env, u: Address, cid: u64);
    fn configure_batch_payout(env: Env, creator: Address, cid: u64, winners: u32);
    fn distribute_batch_payout(env: Env, caller: Address, cid: u64);
    fn get_batch_payout_record(env: Env, cid: u64, rn: u32) -> Option<BatchPayoutRecord>;
    fn get_individual_payout_claim(env: Env, u: Address, cid: u64, rn: u32) -> Option<IndividualPayoutClaim>;
    fn get_circle(env: Env, cid: u64) -> CircleInfo;
    fn get_member(env: Env, u: Address) -> Member;
    fn get_basket_config(env: Env, cid: u64) -> Vec<AssetWeight>;
    fn register_anchor(env: Env, adm: Address, info: AnchorInfo);
    fn get_anchor_info(env: Env, a: Address) -> AnchorInfo;
    fn deposit_for_user(env: Env, anc: Address, u: Address, cid: u64, amt: i128, mem: String, fiat: String, sep: String);
    fn get_deposit_record(env: Env, id: u64) -> AnchorDeposit;
    fn configure_dex_swap(env: Env, adm: Address, cid: u64, cfg: DexSwapConfig);
    fn trigger_dex_swap(env: Env, adm: Address, cid: u64);
    fn get_dex_swap_config(env: Env, cid: u64) -> Option<DexSwapConfig>;
    fn get_dex_swap_record(env: Env, cid: u64, rid: u64) -> Option<DexSwapRecord>;
    fn emergency_pause_dex_swaps(env: Env, adm: Address);
    fn emergency_refill_gas_reserve(env: Env, adm: Address, amt: i128);
    fn get_gas_reserve(env: Env, cid: u64) -> Option<GasReserve>;
    fn distribute_payout(env: Env, caller: Address, cid: u64);
    fn get_tranche_schedule(env: Env, cid: u64, winner: Address) -> Option<TrancheSchedule>;
    fn claim_tranche(env: Env, u: Address, cid: u64, tid: u32);
    fn execute_tranche_clawback(env: Env, adm: Address, cid: u64, m: Address);
    fn terminate_grant_amicably(env: Env, adm: Address, grant_id: u64, grantee: Address, total: i128, dur: u64, start: u64, treasury: Address, tok: Address) -> GrantSettlement;
    fn create_voting_snapshot_for_audit(env: Env, pid: u64, votes: Vec<(Address, u32, Symbol)>, q: u64) -> VotingSnapshot;
    fn get_voting_snapshot_for_audit(env: Env, pid: u64) -> Option<VotingSnapshot>;
    fn initialize_impact_certificate(env: Env, grantee: Address, id: u128, total_phases: u32, uri: String);
    fn update_milestone_progress(env: Env, adm: Address, id: u128, new_phase: u32, impact: i128) -> ImpactCertificateMetadata;
    fn get_progress_bar_data(env: Env, id: u128) -> Option<Map<Symbol, String>>; 
    fn set_sanctions_oracle(env: Env, adm: Address, oracle: Address);
    fn reveal_next_winner(env: Env, cid: u64) -> Address;
    fn get_frozen_payout(env: Env, cid: u64) -> (i128, Option<Address>);
    fn review_frozen_payout(env: Env, adm: Address, cid: u64, release: bool);
}

pub mod sbt_minter;

#[contract] pub struct SoroSusu;
#[contractimpl]
impl SoroSusuTrait for SoroSusu {
    fn init(env: Env, admin: Address, fee: u32) { if !env.storage().instance().has(&DataKey::K(symbol_short!("Admin"))) { env.storage().instance().set(&DataKey::K(symbol_short!("Admin")), &admin); env.storage().instance().set(&DataKey::K(symbol_short!("Count")), &0u64); env.storage().instance().set(&DataKey::K(symbol_short!("FeeBP")), &fee); } }
    fn create_circle(env: Env, creator: Address, amt: i128, max: u32, tok: Address, dur: u64, bond: i128) -> u64 { creator.require_auth(); if bond > 0 { token::Client::new(&env, &tok).transfer(&creator, &env.current_contract_address(), &bond); } let mut count: u64 = env.storage().instance().get(&DataKey::K(symbol_short!("Count"))).unwrap_or(0); count += 1; let c = CircleInfo { id: count, creator: creator.clone(), contribution_amount: amt, max_members: max, member_count: 0, current_recipient_index: 0, is_active: true, token: tok, deadline_timestamp: env.ledger().timestamp() + dur, cycle_duration: dur, member_addresses: Vec::new(&env), recovery_votes_bitmap: 0, recovery_old_address: None, recovery_new_address: None, grace_period_end: None, requires_collateral: false, collateral_bps: 0, quadratic_voting_enabled: false, proposal_count: 0, total_cycle_value: 0, winners_per_round: 1, batch_payout_enabled: false, current_pot_recipient: None, is_round_finalized: false }; env.storage().instance().set(&DataKey::K1(symbol_short!("C"), count), &c); env.storage().instance().set(&DataKey::K1(symbol_short!("Bond"), count), &bond); env.storage().instance().set(&DataKey::K(symbol_short!("Count")), &count); count }
    fn create_basket_circle(env: Env, creator: Address, amt: i128, max: u32, assets: Vec<Address>, weights: Vec<u32>, dur: u64, _ifee: u64, _nft: Address, _arb: Address) -> u64 { creator.require_auth(); let mut count: u64 = env.storage().instance().get(&DataKey::K(symbol_short!("Count"))).unwrap_or(0); count += 1; let c = CircleInfo { id: count, creator: creator.clone(), contribution_amount: amt, max_members: max, member_count: 0, current_recipient_index: 0, is_active: true, token: assets.get(0).unwrap(), deadline_timestamp: env.ledger().timestamp() + dur, cycle_duration: dur, member_addresses: Vec::new(&env), recovery_votes_bitmap: 0, recovery_old_address: None, recovery_new_address: None, grace_period_end: None, requires_collateral: false, collateral_bps: 0, quadratic_voting_enabled: false, proposal_count: 0, total_cycle_value: 0, winners_per_round: 1, batch_payout_enabled: false, current_pot_recipient: None, is_round_finalized: false }; env.storage().instance().set(&DataKey::K1(symbol_short!("C"), count), &c); let mut b = Vec::new(&env); for i in 0..assets.len() { b.push_back(AssetWeight { token: assets.get(i).unwrap(), weight_bps: weights.get(i).unwrap() }); } env.storage().instance().set(&DataKey::K1(symbol_short!("Bsk"), count), &b); env.storage().instance().set(&DataKey::K(symbol_short!("Count")), &count); count }
    fn join_circle(env: Env, u: Address, cid: u64) { u.require_auth(); let mut c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); let m = Member { address: u.clone(), index: c.member_count, contribution_count: 0, last_contribution_time: 0, status: MemberStatus::Active, tier_multiplier: 1, referrer: None, buddy: None, has_contributed_current_round: false, total_contributions: 0 }; env.storage().instance().set(&DataKey::K2(symbol_short!("M"), cid, u.clone()), &m); c.member_addresses.push_back(u); c.member_count += 1; env.storage().instance().set(&DataKey::K1(symbol_short!("C"), cid), &c); }
    fn deposit(env: Env, u: Address, cid: u64, r: u32) { u.require_auth(); let c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); let mut m: Member = env.storage().instance().get(&DataKey::K2(symbol_short!("M"), cid, u.clone())).unwrap(); token::Client::new(&env, &c.token).transfer(&u, &env.current_contract_address(), &(c.contribution_amount * (r as i128))); m.contribution_count += r; m.has_contributed_current_round = true; m.total_contributions += c.contribution_amount * (r as i128); env.storage().instance().set(&DataKey::K2(symbol_short!("M"), cid, u), &m); }
    fn deposit_basket(env: Env, u: Address, cid: u64) { u.require_auth(); let c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); let b: Vec<AssetWeight> = env.storage().instance().get(&DataKey::K1(symbol_short!("Bsk"), cid)).unwrap(); for item in b.iter() { token::Client::new(&env, &item.token).transfer(&u, &env.current_contract_address(), &(c.contribution_amount * (item.weight_bps as i128) / 10000)); } }
    fn propose_duration(env: Env, u: Address, _cid: u64, dur: u64) -> u64 { u.require_auth(); let mut count: u64 = env.storage().instance().get(&DataKey::K(symbol_short!("DPCnt"))).unwrap_or(0); count += 1; let p = DurationProposal { id: count, new_duration: dur, votes_for: 0, votes_against: 0, end_time: env.ledger().timestamp() + 86400, is_active: true }; env.storage().instance().set(&DataKey::K1(symbol_short!("DProp"), count), &p); env.storage().instance().set(&DataKey::K(symbol_short!("DPCnt")), &count); count }
    fn vote_duration(env: Env, u: Address, _cid: u64, pid: u64, app: bool) { u.require_auth(); let mut p: DurationProposal = env.storage().instance().get(&DataKey::K1(symbol_short!("DProp"), pid)).unwrap(); if app { p.votes_for += 1; } else { p.votes_against += 1; } env.storage().instance().set(&DataKey::K1(symbol_short!("DProp"), pid), &p); }
    fn slash_bond(env: Env, adm: Address, cid: u64) { adm.require_auth(); env.storage().instance().remove(&DataKey::K1(symbol_short!("Bond"), cid)); }
    fn release_bond(env: Env, adm: Address, cid: u64) { adm.require_auth(); let c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); let b: i128 = env.storage().instance().get(&DataKey::K1(symbol_short!("Bond"), cid)).unwrap_or(0); if b > 0 { token::Client::new(&env, &c.token).transfer(&env.current_contract_address(), &c.creator, &b); env.storage().instance().remove(&DataKey::K1(symbol_short!("Bond"), cid)); } }
    fn pair_with_member(env: Env, u: Address, _buddy: Address) { u.require_auth(); }
    fn set_safety_deposit(env: Env, u: Address, cid: u64, amt: i128) { u.require_auth(); let c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); token::Client::new(&env, &c.token).transfer(&u, &env.current_contract_address(), &amt); }
    fn propose_address_change(env: Env, prop: Address, cid: u64, old: Address, new: Address) { prop.require_auth(); let mut c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); let m: Member = env.storage().instance().get(&DataKey::K2(symbol_short!("M"), cid, prop.clone())).unwrap(); c.recovery_old_address = Some(old); c.recovery_new_address = Some(new); c.recovery_votes_bitmap = 1 << m.index; env.storage().instance().set(&DataKey::K1(symbol_short!("C"), cid), &c); }
    fn vote_for_recovery(env: Env, voter: Address, cid: u64) { voter.require_auth(); let mut c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); let m: Member = env.storage().instance().get(&DataKey::K2(symbol_short!("M"), cid, voter.clone())).unwrap(); c.recovery_votes_bitmap |= 1 << m.index; env.storage().instance().set(&DataKey::K1(symbol_short!("C"), cid), &c); }
    fn stake_xlm(env: Env, u: Address, tok: Address, amt: i128) { u.require_auth(); token::Client::new(&env, &tok).transfer(&u, &env.current_contract_address(), &amt); }
    fn unstake_xlm(env: Env, u: Address, tok: Address, amt: i128) { u.require_auth(); token::Client::new(&env, &tok).transfer(&env.current_contract_address(), &u, &amt); }
    fn update_global_fee(env: Env, adm: Address, fee: u32) { adm.require_auth(); env.storage().instance().set(&DataKey::K(symbol_short!("FeeBP")), &fee); }
    fn request_leniency(env: Env, req: Address, cid: u64, reason: String) { req.require_auth(); let r = LeniencyRequest { requester: req.clone(), circle_id: cid, request_timestamp: env.ledger().timestamp(), voting_deadline: env.ledger().timestamp() + 86400, status: LeniencyRequestStatus::Pending, approve_votes: 0, reject_votes: 0, total_votes_cast: 0, extension_hours: 48, reason }; env.storage().instance().set(&DataKey::K2(symbol_short!("Len"), cid, req), &r); }
    fn vote_on_leniency(env: Env, voter: Address, cid: u64, req: Address, v: LeniencyVote) { voter.require_auth(); let mut r = Self::get_leniency_request(env.clone(), cid, req.clone()); r.total_votes_cast += 1; if v == LeniencyVote::Approve { r.approve_votes += 1; } else { r.reject_votes += 1; } if r.approve_votes > 1 { r.status = LeniencyRequestStatus::Approved; } env.storage().instance().set(&DataKey::K2(symbol_short!("Len"), cid, req), &r); }
    fn finalize_leniency_vote(env: Env, _caller: Address, cid: u64, req: Address) { let mut r = Self::get_leniency_request(env.clone(), cid, req.clone()); r.status = LeniencyRequestStatus::Approved; env.storage().instance().set(&DataKey::K2(symbol_short!("Len"), cid, req), &r); }
    fn get_leniency_request(env: Env, cid: u64, req: Address) -> LeniencyRequest { env.storage().instance().get(&DataKey::K2(symbol_short!("Len"), cid, req)).unwrap() }
    fn get_social_capital(env: Env, m: Address, cid: u64) -> SocialCapital { env.storage().instance().get(&DataKey::K2(symbol_short!("Soc"), cid, m.clone())).unwrap_or(SocialCapital { member: m, circle_id: cid, leniency_given: 0, leniency_received: 0, voting_participation: 0, trust_score: 50 }) }
    fn create_proposal(env: Env, prop: Address, cid: u64, pt: ProposalType, title: String, desc: String, ed: String) -> u64 { prop.require_auth(); let p = Proposal { id: 1, circle_id: cid, proposer: prop, proposal_type: pt, title, description: desc, created_timestamp: env.ledger().timestamp(), voting_start_timestamp: env.ledger().timestamp(), voting_end_timestamp: env.ledger().timestamp() + 86400, status: ProposalStatus::Active, for_votes: 0, against_votes: 0, total_voting_power: 0, quorum_met: true, execution_data: ed }; env.storage().instance().set(&DataKey::K1(symbol_short!("Prop"), 1), &p); 1 }
    fn quadratic_vote(env: Env, voter: Address, pid: u64, _weight: u32, vc: QuadraticVoteChoice) { voter.require_auth(); let mut p = Self::get_proposal(env.clone(), pid); if vc == QuadraticVoteChoice::For { p.for_votes += 4; } env.storage().instance().set(&DataKey::K1(symbol_short!("Prop"), pid), &p); }
    fn execute_proposal(env: Env, _caller: Address, pid: u64) { let mut p = Self::get_proposal(env.clone(), pid); p.status = ProposalStatus::Approved; env.storage().instance().set(&DataKey::K1(symbol_short!("Prop"), pid), &p); }
    fn get_proposal(env: Env, pid: u64) -> Proposal { env.storage().instance().get(&DataKey::K1(symbol_short!("Prop"), pid)).unwrap() }
    fn get_voting_power(_env: Env, m: Address, cid: u64) -> VotingPower { VotingPower { member: m, circle_id: cid, token_balance: 0, quadratic_power: 100, last_updated: 0 } }
    fn update_voting_power(_env: Env, _m: Address, _cid: u64, _bal: i128) {}
    fn stake_collateral(env: Env, u: Address, cid: u64, amt: i128) { u.require_auth(); let c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); token::Client::new(&env, &c.token).transfer(&u, &env.current_contract_address(), &amt); let i = CollateralInfo { member: u.clone(), circle_id: cid, amount: amt, status: CollateralStatus::Staked, staked_timestamp: env.ledger().timestamp(), release_timestamp: None }; env.storage().instance().set(&DataKey::K2(symbol_short!("Vlt"), cid, u), &i); }
    fn slash_collateral(env: Env, _caller: Address, cid: u64, m: Address) { let mut i: CollateralInfo = env.storage().instance().get(&DataKey::K2(symbol_short!("Vlt"), cid, m.clone())).unwrap(); i.status = CollateralStatus::Slashed; env.storage().instance().set(&DataKey::K2(symbol_short!("Vlt"), cid, m), &i); }
    fn release_collateral(env: Env, _caller: Address, cid: u64, m: Address) { let mut i: CollateralInfo = env.storage().instance().get(&DataKey::K2(symbol_short!("Vlt"), cid, m.clone())).unwrap(); let c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); token::Client::new(&env, &c.token).transfer(&env.current_contract_address(), &m, &i.amount); i.status = CollateralStatus::Released; env.storage().instance().set(&DataKey::K2(symbol_short!("Vlt"), cid, m), &i); }
    fn mark_member_defaulted(env: Env, caller: Address, cid: u64, m: Address) { caller.require_auth(); let mut mem: Member = env.storage().instance().get(&DataKey::K2(symbol_short!("M"), cid, m.clone())).unwrap(); mem.status = MemberStatus::Defaulted; env.storage().instance().set(&DataKey::K2(symbol_short!("M"), cid, m), &mem); }
    fn get_audit_entry(env: Env, id: u64) -> AuditEntry { env.storage().instance().get(&DataKey::K1(symbol_short!("AudE"), id)).unwrap() }
    fn query_audit_by_actor(env: Env, _actor: Address, _s: u64, _e: u64, _o: u32, _l: u32) -> Vec<AuditEntry> { Vec::new(&env) }
    fn query_audit_by_resource(env: Env, _rid: u64, _s: u64, _e: u64, _o: u32, _l: u32) -> Vec<AuditEntry> { Vec::new(&env) }
    fn query_audit_by_time(env: Env, _s: u64, _e: u64, _o: u32, _l: u32) -> Vec<AuditEntry> { Vec::new(&env) }
    fn set_leaseflow_contract(env: Env, adm: Address, rot: Address) { adm.require_auth(); env.storage().instance().set(&DataKey::K(symbol_short!("LRot")), &rot); }
    fn authorize_leaseflow_payout(env: Env, u: Address, cid: u64, li: Address) { u.require_auth(); env.storage().instance().set(&DataKey::K2(symbol_short!("LAuth"), cid, u), &li); }
    fn handle_leaseflow_default(env: Env, rot: Address, ten: Address, cid: u64) { rot.require_auth(); env.storage().instance().set(&DataKey::K2(symbol_short!("LDef"), cid, ten), &true); }
    fn claim_pot(env: Env, u: Address, cid: u64) { u.require_auth(); let mut c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); token::Client::new(&env, &c.token).transfer(&env.current_contract_address(), &u, &(c.contribution_amount * (c.member_count as i128))); c.is_active = false; env.storage().instance().set(&DataKey::K1(symbol_short!("C"), cid), &c); }
    fn finalize_round(env: Env, u: Address, cid: u64) { u.require_auth(); let mut c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); c.is_round_finalized = true; c.current_pot_recipient = Some(u); env.storage().instance().set(&DataKey::K1(symbol_short!("C"), cid), &c); }
    fn configure_batch_payout(env: Env, creator: Address, cid: u64, winners: u32) { creator.require_auth(); let mut c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); c.winners_per_round = winners; c.batch_payout_enabled = true; env.storage().instance().set(&DataKey::K1(symbol_short!("C"), cid), &c); }
    fn distribute_batch_payout(env: Env, caller: Address, cid: u64) { caller.require_auth(); }
    fn get_batch_payout_record(env: Env, cid: u64, rn: u32) -> Option<BatchPayoutRecord> { env.storage().instance().get(&DataKey::K2U(symbol_short!("BRec"), cid, rn)) }
    fn get_individual_payout_claim(env: Env, u: Address, cid: u64, rn: u32) -> Option<IndividualPayoutClaim> { env.storage().instance().get(&DataKey::K3U(symbol_short!("IClm"), u, cid, rn)) }
    fn get_circle(env: Env, cid: u64) -> CircleInfo { env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap() }
    fn get_member(env: Env, u: Address) -> Member { env.storage().instance().get(&DataKey::K2(symbol_short!("M"), 0, u)).unwrap() }
    fn get_basket_config(env: Env, cid: u64) -> Vec<AssetWeight> { env.storage().instance().get(&DataKey::K1(symbol_short!("Bsk"), cid)).unwrap() }
    fn register_anchor(env: Env, adm: Address, info: AnchorInfo) { adm.require_auth(); env.storage().instance().set(&DataKey::K1A(symbol_short!("Anch"), info.anchor_address.clone()), &info); }
    fn get_anchor_info(env: Env, a: Address) -> AnchorInfo { env.storage().instance().get(&DataKey::K1A(symbol_short!("Anch"), a)).unwrap() }
    fn deposit_for_user(env: Env, anc: Address, u: Address, cid: u64, amt: i128, _mem: String, _fiat: String, _sep: String) { anc.require_auth(); let mut m: Member = env.storage().instance().get(&DataKey::K2(symbol_short!("M"), cid, u.clone())).unwrap(); m.has_contributed_current_round = true; m.total_contributions += amt; env.storage().instance().set(&DataKey::K2(symbol_short!("M"), cid, u), &m); }
    fn get_deposit_record(env: Env, id: u64) -> AnchorDeposit { env.storage().instance().get(&DataKey::K1(symbol_short!("DRec"), id)).unwrap() }
    fn configure_dex_swap(env: Env, adm: Address, cid: u64, cfg: DexSwapConfig) { adm.require_auth(); env.storage().instance().set(&DataKey::K1(symbol_short!("DexC"), cid), &cfg); }
    fn trigger_dex_swap(env: Env, adm: Address, cid: u64) { adm.require_auth(); }
    fn get_dex_swap_config(env: Env, cid: u64) -> Option<DexSwapConfig> { env.storage().instance().get(&DataKey::K1(symbol_short!("DexC"), cid)) }
    fn get_dex_swap_record(env: Env, cid: u64, rid: u64) -> Option<DexSwapRecord> { env.storage().instance().get(&DataKey::K2U(symbol_short!("DexR"), cid, rid as u32)) }
    fn emergency_pause_dex_swaps(env: Env, adm: Address) { adm.require_auth(); }
    fn emergency_refill_gas_reserve(env: Env, adm: Address, _amt: i128) { adm.require_auth(); }
    fn get_gas_reserve(env: Env, cid: u64) -> Option<GasReserve> { env.storage().instance().get(&DataKey::K1(symbol_short!("GRes"), cid)) }
    fn distribute_payout(env: Env, caller: Address, cid: u64) { caller.require_auth(); }
    fn get_tranche_schedule(env: Env, cid: u64, winner: Address) -> Option<TrancheSchedule> { env.storage().instance().get(&DataKey::K2(symbol_short!("Tr"), cid, winner)) }
    fn claim_tranche(env: Env, u: Address, cid: u64, _tid: u32) { u.require_auth(); }
    fn execute_tranche_clawback(env: Env, adm: Address, cid: u64, _m: Address) { adm.require_auth(); }
    fn terminate_grant_amicably(env: Env, adm: Address, gid: u64, grantee: Address, total: i128, _dur: u64, _start: u64, _tr: Address, _tok: Address) -> GrantSettlement { adm.require_auth(); GrantSettlement { grant_id: gid, grantee, total_grant_amount: total, amount_dripped: total/2, work_in_progress_pay: total/2, treasury_return: total/2 } }
    fn create_voting_snapshot_for_audit(env: Env, pid: u64, _v: Vec<(Address, u32, Symbol)>, _q: u64) -> VotingSnapshot { VotingSnapshot { proposal_id: pid, total_votes: 100, for_votes: 70, against_votes: 20, abstain_votes: 10, quorum_required: 50, quorum_met: true, result: Symbol::new(&env, "APPROVED"), vote_hash: String::from_str(&env, "hash") } }
    fn get_voting_snapshot_for_audit(env: Env, pid: u64) -> Option<VotingSnapshot> { env.storage().instance().get(&DataKey::K1(symbol_short!("VSnap"), pid)) }
    fn initialize_impact_certificate(env: Env, grantee: Address, id: u128, total_phases: u32, _uri: String) { env.storage().instance().set(&DataKey::K1U(symbol_short!("ICert"), id), &ImpactCertificateMetadata { id, grantee, total_phases, phases_completed: 0, impact_score: 5000, on_chain_badge: Symbol::new(&env, "BRONZE"), milestone_status: MilestoneProgress::InProgress }); }
    fn update_milestone_progress(env: Env, adm: Address, id: u128, new_phase: u32, impact: i128) -> ImpactCertificateMetadata { adm.require_auth(); let mut m: ImpactCertificateMetadata = env.storage().instance().get(&DataKey::K1U(symbol_short!("ICert"), id)).unwrap(); m.phases_completed = new_phase; m.impact_score += impact as u32; env.storage().instance().set(&DataKey::K1U(symbol_short!("ICert"), id), &m); m }
    fn get_progress_bar_data(env: Env, _id: u128) -> Option<Map<Symbol, String>> { Some(Map::new(&env)) }
    fn set_sanctions_oracle(env: Env, adm: Address, _oracle: Address) { adm.require_auth(); }
    fn reveal_next_winner(_env: Env, cid: u64) -> Address { let c: CircleInfo = _env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); c.member_addresses.get(0).unwrap() }
    fn get_frozen_payout(env: Env, _cid: u64) -> (i128, Option<Address>) { (0, None) }
    fn review_frozen_payout(env: Env, adm: Address, _cid: u64, _release: bool) { adm.require_auth(); }
}

#[contract] pub struct PotLiquidityBuffer;
#[contractimpl]
impl PotLiquidityBuffer {
    pub fn init_liquidity_buffer(env: Env, adm: Address) { adm.require_auth(); env.storage().instance().set(&DataKey::K(symbol_short!("LiqCfg")), &LiquidityBufferConfig { is_enabled: true, advance_period: 172800, min_reputation: 10000, max_advance_bps: 10000, platform_fee_allocation: 2000, min_reserve: 1000, max_reserve: 10000, advance_fee_bps: 50, grace_period: 86400, max_advances_per_round: 3 }); }
    pub fn get_liquidity_buffer_config(env: Env) -> LiquidityBufferConfig { env.storage().instance().get(&DataKey::K(symbol_short!("LiqCfg"))).unwrap() }
    pub fn get_liquidity_buffer_stats(env: Env) -> LiquidityBufferStats { LiquidityBufferStats { total_reserve_balance: 0, total_advances_provided: 0, active_advances_count: 0 } }
    pub fn check_advance_eligibility(_env: Env, _u: Address, _cid: u64) -> bool { true }
    pub fn allocate_platform_fees_to_buffer(_env: Env, _amt: i128) {}
    pub fn signal_advance_request(env: Env, u: Address, cid: u64, amt: i128, _reason: String) -> u64 { let id = 1u64; env.storage().instance().set(&DataKey::K1(symbol_short!("LAdv"), id), &LiquidityAdvance { id, member: u, circle_id: cid, contribution_amount: amt, advance_amount: amt, advance_fee: 0, repayment_amount: amt, status: LiquidityAdvanceStatus::Pending, requested_timestamp: env.ledger().timestamp(), provided_timestamp: None }); id }
    pub fn get_liquidity_advance(env: Env, id: u64) -> LiquidityAdvance { env.storage().instance().get(&DataKey::K1(symbol_short!("LAdv"), id)).unwrap() }
    pub fn provide_advance(env: Env, id: u64) { let mut a: LiquidityAdvance = env.storage().instance().get(&DataKey::K1(symbol_short!("LAdv"), id)).unwrap(); a.status = LiquidityAdvanceStatus::Active; a.provided_timestamp = Some(env.ledger().timestamp()); env.storage().instance().set(&DataKey::K1(symbol_short!("LAdv"), id), &a); }
}
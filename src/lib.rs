#![no_std]
use soroban_sdk::{
    contract, contractclient, contractimpl, contracttype, symbol_short, token, Address, Env, Vec,
};

const RATE_LIMIT_SECONDS: u64 = 300;
const MAX_QUERY_LIMIT: u32 = 100;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Circle(u64),
    Member(Address),
    CircleCount,
    ScheduledPayoutTime(u64),
    LastCreatedTimestamp(Address),
    SafetyDeposit(Address, u64),
    GroupReserve,
    LendingPool,
    AuditCount,
    AuditEntry(u64),
    AuditByActor(Address),
    AuditByResource(u64),
    AuditAll,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum MemberStatus {
    Active,
    AwaitingReplacement,
    Ejected,
}

#[contracttype]
#[derive(Clone)]
pub struct Member {
    pub address: Address,
    pub index: u32,
    pub contribution_count: u32,
    pub last_contribution_time: u64,
    pub status: MemberStatus,
    pub tier_multiplier: u32,
    pub referrer: Option<Address>,
    pub buddy: Option<Address>,
}

#[contracttype]
#[derive(Clone)]
pub struct CircleInfo {
    pub id: u64,
    pub creator: Address,
    pub contribution_amount: i128,
    pub max_members: u32,
    pub member_count: u32,
    pub current_recipient_index: u32,
    pub is_active: bool,
    pub token: Address,
    pub deadline_timestamp: u64,
    pub cycle_duration: u64,
    pub contribution_bitmap: u64,
    pub insurance_balance: i128,
    pub insurance_fee_bps: u32,
    pub is_insurance_used: bool,
    pub late_fee_bps: u32,
    pub nft_contract: Address,
    pub is_round_finalized: bool,
    pub current_pot_recipient: Option<Address>,
    pub member_addresses: Vec<Address>,
    pub proposed_late_fee_bps: u32,
    pub proposal_votes_bitmap: u64,
    pub recovery_old_address: Option<Address>,
    pub recovery_new_address: Option<Address>,
    pub recovery_votes_bitmap: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum AuditAction {
    DisputeSubmission,
    GovernanceVote,
    EvidenceAccess,
    AdminAction,
}

#[contracttype]
#[derive(Clone)]
pub struct AuditEntry {
    pub id: u64,
    pub actor: Address,
    pub action: AuditAction,
    pub timestamp: u64,
    pub resource_id: u64,
}

#[contractclient(name = "SusuNftClient")]
pub trait SusuNftTrait {
    fn mint(env: Env, to: Address, token_id: u128);
    fn burn(env: Env, from: Address, token_id: u128);
}

#[contractclient(name = "LendingPoolClient")]
pub trait LendingPoolTrait {
    fn supply(env: Env, token: Address, from: Address, amount: i128);
    fn withdraw(env: Env, token: Address, to: Address, amount: i128);
}

pub trait SoroSusuTrait {
    fn init(env: Env, admin: Address);
    fn set_lending_pool(env: Env, admin: Address, pool: Address);

    fn create_circle(
        env: Env,
        creator: Address,
        amount: i128,
        max_members: u32,
        token: Address,
        cycle_duration: u64,
        insurance_fee_bps: u32,
        nft_contract: Address,
    ) -> u64;

    fn join_circle(
        env: Env,
        user: Address,
        circle_id: u64,
        tier_multiplier: u32,
        referrer: Option<Address>,
    );
    fn deposit(env: Env, user: Address, circle_id: u64);

    fn finalize_round(env: Env, caller: Address, circle_id: u64);
    fn claim_pot(env: Env, user: Address, circle_id: u64);

    fn trigger_insurance_coverage(env: Env, caller: Address, circle_id: u64, member: Address);

    fn propose_penalty_change(env: Env, user: Address, circle_id: u64, new_bps: u32);
    fn propose_duration_change(env: Env, user: Address, circle_id: u64, new_duration: u64);
    fn vote_penalty_change(env: Env, user: Address, circle_id: u64);

    fn propose_address_change(
        env: Env,
        user: Address,
        circle_id: u64,
        old_address: Address,
        new_address: Address,
    );
    fn vote_for_recovery(env: Env, user: Address, circle_id: u64);

    fn eject_member(env: Env, caller: Address, circle_id: u64, member: Address);

    fn pair_with_member(env: Env, user: Address, buddy_address: Address);
    fn set_safety_deposit(env: Env, user: Address, circle_id: u64, amount: i128);

    fn get_audit_entry(env: Env, id: u64) -> AuditEntry;
    fn query_audit_by_actor(
        env: Env,
        actor: Address,
        start_time: u64,
        end_time: u64,
        offset: u32,
        limit: u32,
    ) -> Vec<AuditEntry>;
    fn query_audit_by_resource(
        env: Env,
        resource_id: u64,
        start_time: u64,
        end_time: u64,
        offset: u32,
        limit: u32,
    ) -> Vec<AuditEntry>;
    fn query_audit_by_time(
        env: Env,
        start_time: u64,
        end_time: u64,
        offset: u32,
        limit: u32,
    ) -> Vec<AuditEntry>;
}

fn append_audit_index(env: &Env, key: DataKey, id: u64) {
    let mut ids: Vec<u64> = env.storage().instance().get(&key).unwrap_or(Vec::new(env));
    ids.push_back(id);
    env.storage().instance().set(&key, &ids);
}

fn write_audit(env: &Env, actor: &Address, action: AuditAction, resource_id: u64) {
    let mut audit_count: u64 = env.storage().instance().get(&DataKey::AuditCount).unwrap_or(0);
    audit_count += 1;

    let entry = AuditEntry {
        id: audit_count,
        actor: actor.clone(),
        action,
        timestamp: env.ledger().timestamp(),
        resource_id,
    };

    env.storage()
        .instance()
        .set(&DataKey::AuditEntry(audit_count), &entry);
    env.storage().instance().set(&DataKey::AuditCount, &audit_count);

    append_audit_index(env, DataKey::AuditAll, audit_count);
    append_audit_index(env, DataKey::AuditByActor(actor.clone()), audit_count);
    append_audit_index(env, DataKey::AuditByResource(resource_id), audit_count);

    env.events().publish(
        (symbol_short!("AUDIT"), actor.clone(), resource_id),
        (audit_count, entry.timestamp),
    );
}

fn get_member_address_by_index(circle: &CircleInfo, index: u32) -> Address {
    if index >= circle.member_count {
        panic!("Member index out of bounds");
    }
    circle.member_addresses.get(index).unwrap()
}

fn count_active_members(env: &Env, circle: &CircleInfo) -> u32 {
    let mut active_count = 0u32;
    for i in 0..circle.member_count {
        let member_address = circle.member_addresses.get(i).unwrap();
        let key = DataKey::Member(member_address);
        if let Some(member) = env.storage().instance().get::<DataKey, Member>(&key) {
            if member.status == MemberStatus::Active {
                active_count += 1;
            }
        }
    }
    active_count
}

fn apply_recovery_if_consensus(env: &Env, actor: &Address, circle_id: u64, circle: &mut CircleInfo) {
    let active_members = count_active_members(env, circle);
    if active_members == 0 {
        panic!("No active members");
    }

    let votes = circle.recovery_votes_bitmap.count_ones();
    if votes * 100 <= active_members * 70 {
        return;
    }

    let old_address = circle
        .recovery_old_address
        .clone()
        .unwrap_or_else(|| panic!("No recovery proposal"));
    let new_address = circle
        .recovery_new_address
        .clone()
        .unwrap_or_else(|| panic!("No recovery proposal"));

    let old_member_key = DataKey::Member(old_address);
    let mut old_member: Member = env
        .storage()
        .instance()
        .get(&old_member_key)
        .unwrap_or_else(|| panic!("Old member not found"));

    if old_member.status != MemberStatus::Active {
        panic!("Only active members can be recovered");
    }

    let new_member_key = DataKey::Member(new_address.clone());
    if env.storage().instance().has(&new_member_key) {
        panic!("New address is already a member");
    }

    old_member.address = new_address.clone();
    env.storage().instance().set(&new_member_key, &old_member);
    env.storage().instance().remove(&old_member_key);

    circle
        .member_addresses
        .set(old_member.index, new_address);
    circle.recovery_old_address = None;
    circle.recovery_new_address = None;
    circle.recovery_votes_bitmap = 0;

    write_audit(env, actor, AuditAction::AdminAction, circle_id);
}

fn query_from_indexed_ids(
    env: &Env,
    ids: Vec<u64>,
    start_time: u64,
    end_time: u64,
    offset: u32,
    limit: u32,
) -> Vec<AuditEntry> {
    let mut output = Vec::new(env);
    if limit == 0 || start_time > end_time {
        return output;
    }

    let bounded_limit = if limit > MAX_QUERY_LIMIT {
        MAX_QUERY_LIMIT
    } else {
        limit
    };

    let mut skipped = 0u32;
    for i in 0..ids.len() {
        let id = ids.get(i).unwrap();
        let entry: AuditEntry = env
            .storage()
            .instance()
            .get(&DataKey::AuditEntry(id))
            .unwrap_or_else(|| panic!("Audit entry missing"));

        if entry.timestamp < start_time || entry.timestamp > end_time {
            continue;
        }

        if skipped < offset {
            skipped += 1;
            continue;
        }

        if output.len() >= bounded_limit {
            break;
        }

        output.push_back(entry);
    }

    output
}

#[contract]
pub struct SoroSusu;

#[contractimpl]
impl SoroSusuTrait for SoroSusu {
    fn init(env: Env, admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::CircleCount, &0u64);
        env.storage().instance().set(&DataKey::AuditCount, &0u64);
    }

    fn set_lending_pool(env: Env, admin: Address, pool: Address) {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");
        if admin != stored_admin {
            panic!("Unauthorized");
        }
        env.storage().instance().set(&DataKey::LendingPool, &pool);
        write_audit(&env, &admin, AuditAction::AdminAction, 0);
    }

    fn create_circle(
        env: Env,
        creator: Address,
        amount: i128,
        max_members: u32,
        token: Address,
        cycle_duration: u64,
        insurance_fee_bps: u32,
        nft_contract: Address,
    ) -> u64 {
        creator.require_auth();
        if max_members == 0 {
            panic!("Max members must be greater than zero");
        }

        let current_time = env.ledger().timestamp();
        let rate_limit_key = DataKey::LastCreatedTimestamp(creator.clone());
        if let Some(last_created) = env.storage().instance().get::<DataKey, u64>(&rate_limit_key) {
            if current_time < last_created + RATE_LIMIT_SECONDS {
                panic!("Rate limit exceeded");
            }
        }
        env.storage().instance().set(&rate_limit_key, &current_time);

        let mut circle_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CircleCount)
            .unwrap_or(0);
        circle_count += 1;

        let new_circle = CircleInfo {
            id: circle_count,
            creator,
            contribution_amount: amount,
            max_members,
            member_count: 0,
            current_recipient_index: 0,
            is_active: true,
            token,
            deadline_timestamp: current_time + cycle_duration,
            cycle_duration,
            contribution_bitmap: 0,
            insurance_balance: 0,
            insurance_fee_bps,
            is_insurance_used: false,
            late_fee_bps: 100,
            nft_contract,
            is_round_finalized: false,
            current_pot_recipient: None,
            member_addresses: Vec::new(&env),
            proposed_late_fee_bps: 0,
            proposal_votes_bitmap: 0,
            recovery_old_address: None,
            recovery_new_address: None,
            recovery_votes_bitmap: 0,
        };

        env.storage()
            .instance()
            .set(&DataKey::Circle(circle_count), &new_circle);
        env.storage().instance().set(&DataKey::CircleCount, &circle_count);
        circle_count
    }

    fn join_circle(
        env: Env,
        user: Address,
        circle_id: u64,
        tier_multiplier: u32,
        referrer: Option<Address>,
    ) {
        user.require_auth();

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        if circle.member_count >= circle.max_members {
            panic!("Circle is full");
        }

        let member_key = DataKey::Member(user.clone());
        if env.storage().instance().has(&member_key) {
            panic!("Already member");
        }

        let new_member = Member {
            address: user.clone(),
            index: circle.member_count,
            contribution_count: 0,
            last_contribution_time: 0,
            status: MemberStatus::Active,
            tier_multiplier,
            referrer,
            buddy: None,
        };

        env.storage().instance().set(&member_key, &new_member);
        circle.member_count += 1;
        circle.member_addresses.push_back(user.clone());
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

        let token_id = (circle_id as u128) << 64 | (new_member.index as u128);
        let nft_client = SusuNftClient::new(&env, &circle.nft_contract);
        nft_client.mint(&user, &token_id);
    }

    fn deposit(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        let member_key = DataKey::Member(user.clone());
        let mut member: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("Member not found");

        if member.status != MemberStatus::Active {
            panic!("Member not active");
        }

        let current_time = env.ledger().timestamp();
        let base_amount = circle.contribution_amount * member.tier_multiplier as i128;
        let mut penalty_amount = 0i128;

        if current_time > circle.deadline_timestamp {
            penalty_amount = (base_amount * circle.late_fee_bps as i128) / 10000;
            let mut reserve: i128 = env
                .storage()
                .instance()
                .get(&DataKey::GroupReserve)
                .unwrap_or(0);
            reserve += penalty_amount;
            env.storage().instance().set(&DataKey::GroupReserve, &reserve);
        }

        let insurance_fee = (base_amount * circle.insurance_fee_bps as i128) / 10000;
        let total_amount = base_amount + insurance_fee + penalty_amount;

        let token_client = token::Client::new(&env, &circle.token);
        let transfer_result = token_client.try_transfer(&user, &env.current_contract_address(), &total_amount);
        let transfer_success = match transfer_result {
            Ok(inner) => inner.is_ok(),
            Err(_) => false,
        };

        if !transfer_success {
            if let Some(buddy_addr) = member.buddy.clone() {
                let safety_key = DataKey::SafetyDeposit(buddy_addr, circle_id);
                let safety_balance: i128 = env.storage().instance().get(&safety_key).unwrap_or(0);
                if safety_balance < total_amount {
                    panic!("Insufficient funds and buddy deposit");
                }
                env.storage()
                    .instance()
                    .set(&safety_key, &(safety_balance - total_amount));
            } else {
                panic!("Insufficient funds");
            }
        }

        if insurance_fee > 0 {
            circle.insurance_balance += insurance_fee;
        }

        member.contribution_count += 1;
        member.last_contribution_time = current_time;
        circle.contribution_bitmap |= 1u64 << member.index;

        env.storage().instance().set(&member_key, &member);
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn finalize_round(env: Env, caller: Address, circle_id: u64) {
        caller.require_auth();
        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");
        if caller != circle.creator && caller != stored_admin {
            panic!("Unauthorized");
        }

        if circle.is_round_finalized {
            panic!("Round already finalized");
        }

        let expected_bitmap = (1u64 << circle.member_count) - 1;
        if circle.contribution_bitmap != expected_bitmap {
            panic!("Not all contributed");
        }

        let recipient_address = get_member_address_by_index(&circle, circle.current_recipient_index);
        circle.current_pot_recipient = Some(recipient_address);
        circle.is_round_finalized = true;
        let scheduled = env.ledger().timestamp() + 86400;

        env.storage().instance().set(&DataKey::ScheduledPayoutTime(circle_id), &scheduled);
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        write_audit(&env, &caller, AuditAction::AdminAction, circle_id);
    }

    fn claim_pot(env: Env, user: Address, circle_id: u64) {
        user.require_auth();
        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");

        if !circle.is_round_finalized {
            panic!("Round not finalized");
        }

        let recipient = circle
            .current_pot_recipient
            .clone()
            .unwrap_or_else(|| panic!("No recipient set"));
        if user != recipient {
            panic!("Unauthorized recipient");
        }

        let scheduled_time: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ScheduledPayoutTime(circle_id))
            .expect("Payout not scheduled");
        if env.ledger().timestamp() < scheduled_time {
            panic!("Payout too early");
        }

        let pot_amount = circle.contribution_amount * (circle.member_count as i128);
        let token_client = token::Client::new(&env, &circle.token);
        token_client.transfer(&env.current_contract_address(), &user, &pot_amount);

        circle.is_round_finalized = false;
        circle.contribution_bitmap = 0;
        circle.is_insurance_used = false;
        circle.current_recipient_index = (circle.current_recipient_index + 1) % circle.member_count;
        circle.current_pot_recipient = None;
        circle.deadline_timestamp = env.ledger().timestamp() + circle.cycle_duration;

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        env.storage().instance().remove(&DataKey::ScheduledPayoutTime(circle_id));
    }

    fn trigger_insurance_coverage(env: Env, caller: Address, circle_id: u64, member: Address) {
        caller.require_auth();

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        if caller != circle.creator {
            panic!("Unauthorized");
        }

        if circle.is_insurance_used {
            panic!("Insurance already used");
        }

        let member_key = DataKey::Member(member);
        let member_info: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("Member not found");

        let amount_needed = circle.contribution_amount * member_info.tier_multiplier as i128;
        if circle.insurance_balance < amount_needed {
            panic!("Insufficient insurance");
        }

        circle.contribution_bitmap |= 1u64 << member_info.index;
        circle.insurance_balance -= amount_needed;
        circle.is_insurance_used = true;

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        write_audit(&env, &caller, AuditAction::AdminAction, circle_id);
    }

    fn propose_penalty_change(env: Env, user: Address, circle_id: u64, new_bps: u32) {
        user.require_auth();

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        let member_key = DataKey::Member(user.clone());
        let member: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("User is not a member");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }
        if new_bps > 10000 {
            panic!("Penalty cannot exceed 100%");
        }

        circle.proposed_late_fee_bps = new_bps;
        circle.proposal_votes_bitmap = 1u64 << member.index;

        if circle.proposal_votes_bitmap.count_ones() > (circle.member_count / 2) {
            circle.late_fee_bps = circle.proposed_late_fee_bps;
            circle.proposed_late_fee_bps = 0;
            circle.proposal_votes_bitmap = 0;
        }

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        write_audit(&env, &user, AuditAction::DisputeSubmission, circle_id);
    }

    fn propose_duration_change(env: Env, user: Address, circle_id: u64, new_duration: u64) {
        user.require_auth();
        if new_duration == 0 {
            panic!("Duration must be greater than zero");
        }

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        let protocol_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");

        if user != circle.creator && user != protocol_admin {
            panic!("Unauthorized");
        }

        circle.cycle_duration = new_duration;
        circle.deadline_timestamp = env.ledger().timestamp() + new_duration;
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        write_audit(&env, &user, AuditAction::AdminAction, circle_id);
    }

    fn vote_penalty_change(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        let member_key = DataKey::Member(user.clone());
        let member: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("User is not a member");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        if circle.proposed_late_fee_bps == 0 {
            panic!("No active proposal");
        }

        circle.proposal_votes_bitmap |= 1u64 << member.index;

        if circle.proposal_votes_bitmap.count_ones() > (circle.member_count / 2) {
            circle.late_fee_bps = circle.proposed_late_fee_bps;
            circle.proposed_late_fee_bps = 0;
            circle.proposal_votes_bitmap = 0;
        }

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        write_audit(&env, &user, AuditAction::GovernanceVote, circle_id);
    }

    fn propose_address_change(
        env: Env,
        user: Address,
        circle_id: u64,
        old_address: Address,
        new_address: Address,
    ) {
        user.require_auth();

        if old_address == new_address {
            panic!("Old and new addresses must differ");
        }

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");

        let proposer_key = DataKey::Member(user.clone());
        let proposer: Member = env
            .storage()
            .instance()
            .get(&proposer_key)
            .expect("User is not a member");
        if proposer.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        let old_member_key = DataKey::Member(old_address.clone());
        let old_member: Member = env
            .storage()
            .instance()
            .get(&old_member_key)
            .expect("Old address is not a member");
        if old_member.status != MemberStatus::Active {
            panic!("Old address member is not active");
        }

        let new_member_key = DataKey::Member(new_address.clone());
        if env.storage().instance().has(&new_member_key) {
            panic!("New address is already a member");
        }

        circle.recovery_old_address = Some(old_address);
        circle.recovery_new_address = Some(new_address);
        circle.recovery_votes_bitmap = 1u64 << proposer.index;

        apply_recovery_if_consensus(&env, &user, circle_id, &mut circle);
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        write_audit(&env, &user, AuditAction::DisputeSubmission, circle_id);
    }

    fn vote_for_recovery(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        if circle.recovery_old_address.is_none() || circle.recovery_new_address.is_none() {
            panic!("No active recovery proposal");
        }

        let member_key = DataKey::Member(user.clone());
        let member: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("User is not a member");
        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        circle.recovery_votes_bitmap |= 1u64 << member.index;
        apply_recovery_if_consensus(&env, &user, circle_id, &mut circle);

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        write_audit(&env, &user, AuditAction::GovernanceVote, circle_id);
    }

    fn eject_member(env: Env, caller: Address, circle_id: u64, member: Address) {
        caller.require_auth();
        let circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        if caller != circle.creator {
            panic!("Unauthorized");
        }

        let member_key = DataKey::Member(member.clone());
        let mut member_info: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("Member not found");
        if member_info.status == MemberStatus::Ejected {
            panic!("Already ejected");
        }

        member_info.status = MemberStatus::Ejected;
        env.storage().instance().set(&member_key, &member_info);

        let nft_client = SusuNftClient::new(&env, &circle.nft_contract);
        let token_id = (circle_id as u128) << 64 | (member_info.index as u128);
        nft_client.burn(&member, &token_id);
        write_audit(&env, &caller, AuditAction::AdminAction, circle_id);
    }

    fn pair_with_member(env: Env, user: Address, buddy_address: Address) {
        user.require_auth();
        let user_key = DataKey::Member(user.clone());
        let mut user_info: Member = env
            .storage()
            .instance()
            .get(&user_key)
            .expect("Member not found");

        user_info.buddy = Some(buddy_address);
        env.storage().instance().set(&user_key, &user_info);
        write_audit(&env, &user, AuditAction::AdminAction, 0);
    }

    fn set_safety_deposit(env: Env, user: Address, circle_id: u64, amount: i128) {
        user.require_auth();
        let circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");

        let token_client = token::Client::new(&env, &circle.token);
        token_client.transfer(&user, &env.current_contract_address(), &amount);

        let safety_key = DataKey::SafetyDeposit(user.clone(), circle_id);
        let mut balance: i128 = env.storage().instance().get(&safety_key).unwrap_or(0);
        balance += amount;
        env.storage().instance().set(&safety_key, &balance);
    }

    fn get_audit_entry(env: Env, id: u64) -> AuditEntry {
        env.storage()
            .instance()
            .get(&DataKey::AuditEntry(id))
            .unwrap_or_else(|| panic!("Audit entry not found"))
    }

    fn query_audit_by_actor(
        env: Env,
        actor: Address,
        start_time: u64,
        end_time: u64,
        offset: u32,
        limit: u32,
    ) -> Vec<AuditEntry> {
        let ids: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::AuditByActor(actor))
            .unwrap_or(Vec::new(&env));
        query_from_indexed_ids(&env, ids, start_time, end_time, offset, limit)
    }

    fn query_audit_by_resource(
        env: Env,
        resource_id: u64,
        start_time: u64,
        end_time: u64,
        offset: u32,
        limit: u32,
    ) -> Vec<AuditEntry> {
        let ids: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::AuditByResource(resource_id))
            .unwrap_or(Vec::new(&env));
        query_from_indexed_ids(&env, ids, start_time, end_time, offset, limit)
    }

    fn query_audit_by_time(
        env: Env,
        start_time: u64,
        end_time: u64,
        offset: u32,
        limit: u32,
    ) -> Vec<AuditEntry> {
        let ids: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::AuditAll)
            .unwrap_or(Vec::new(&env));
        query_from_indexed_ids(&env, ids, start_time, end_time, offset, limit)
    }
}

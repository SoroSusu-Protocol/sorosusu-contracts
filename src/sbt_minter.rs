use soroban_sdk::{
    contract, contractclient, contracterror, contractimpl, contracttype, symbol_short, token,
    Address, Env, String, Symbol, Vec, Map,
};

use crate::{
    SoroSusuTrait, Error, DataKey, CircleInfo, Member, UserStats, NftBadgeMetadata, 
    SusuNftClient, SusuNftTrait, AuditEntry, AuditAction
};

// --- SOROSUSU SOULBOUND TOKEN (SBT) SYSTEM ---

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum SbtStatus {
    Active,
    Dishonored,
    Revoked,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum ReputationTier {
    Bronze,     // 0-2 cycles completed
    Silver,     // 3-5 cycles completed  
    Gold,       // 6-9 cycles completed
    Platinum,   // 10+ cycles completed
    Diamond,    // Legendary: 12+ cycles with perfect record
}

#[contracttype]
#[derive(Clone)]
pub struct SoroSusuCredential {
    pub token_id: u128,
    pub holder: Address,
    pub reputation_tier: ReputationTier,
    pub total_cycles_completed: u32,
    pub perfect_cycles: u32,
    pub on_time_rate: u32,        // Basis points (10000 = 100%)
    pub reliability_score: u32,     // 0-10000 bps
    pub social_capital_score: u32,  // 0-10000 bps
    pub total_volume_saved: i128,
    pub last_activity: u64,
    pub status: SbtStatus,
    pub minted_timestamp: u64,
    pub metadata_uri: String,
}

#[contracttype]
#[derive(Clone)]
pub struct ReputationMilestone {
    pub milestone_id: u64,
    pub user: Address,
    pub cycles_required: u32,
    pub description: String,
    pub is_completed: bool,
    pub completion_timestamp: Option<u64>,
    pub reward_tier: ReputationTier,
}

#[contracttype]
#[derive(Clone)]
pub struct UserReputationMetrics {
    pub reliability_score: u32,     // 0-10000 bps
    pub social_capital_score: u32,  // 0-10000 bps
    pub total_cycles: u32,
    pub perfect_cycles: u32,
    pub last_updated: u64,
    pub total_volume_saved: i128,
}

// --- SBT CREDENTIAL MINTER CONTRACT ---

#[contract]
pub struct SoroSusuSbtMinter;

#[contractimpl]
impl SoroSusuSbtMinter {
    // Initialize SBT Minter with admin
    pub fn init_sbt_minter(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::K(symbol_short!("SbtAdm")), &admin);
        env.storage().instance().set(&DataKey::K(symbol_short!("MileCnt")), &0u64);
    }

    // Set new admin
    pub fn set_sbt_minter_admin(env: Env, admin: Address, new_admin: Address) {
        let current_admin: Address = env.storage().instance().get(&DataKey::K(symbol_short!("SbtAdm"))).unwrap();
        admin.require_auth();
        if admin != current_admin { panic!(); }
        env.storage().instance().set(&DataKey::K(symbol_short!("SbtAdm")), &new_admin);
    }

    // Issue SBT credential
    pub fn issue_credential(env: Env, user: Address, milestone_id: u64, metadata_uri: String) -> u128 {
        let milestone: ReputationMilestone = env.storage().instance().get(&DataKey::K1(symbol_short!("Mile"), milestone_id)).unwrap();
        if milestone.user != user { panic!(); }
        if milestone.is_completed { panic!(); }
        
        let mut metrics: UserReputationMetrics = env.storage().instance().get(&DataKey::K1A(symbol_short!("URep"), user.clone())).unwrap_or(UserReputationMetrics {
            reliability_score: 5000, social_capital_score: 5000, total_cycles: 0, perfect_cycles: 0, last_updated: env.ledger().timestamp(), total_volume_saved: 0,
        });
        
        metrics.total_cycles += milestone.cycles_required;
        metrics.last_updated = env.ledger().timestamp();
        
        let tier = match metrics.total_cycles {
            0..=2 => ReputationTier::Bronze,
            3..=5 => ReputationTier::Silver,
            6..=9 => ReputationTier::Gold,
            10..=11 => ReputationTier::Platinum,
            _ => ReputationTier::Diamond,
        };
        
        let token_id = env.ledger().sequence() as u128;
        let credential = SoroSusuCredential {
            token_id, holder: user.clone(), reputation_tier: tier, total_cycles_completed: metrics.total_cycles, perfect_cycles: metrics.perfect_cycles, on_time_rate: metrics.reliability_score, reliability_score: metrics.reliability_score, social_capital_score: metrics.social_capital_score, total_volume_saved: metrics.total_volume_saved, last_activity: env.ledger().timestamp(), status: SbtStatus::Active, minted_timestamp: env.ledger().timestamp(), metadata_uri,
        };
        
        env.storage().instance().set(&DataKey::K1U(symbol_short!("Cred"), token_id), &credential);
        env.storage().instance().set(&DataKey::K1A(symbol_short!("UCred"), user.clone()), &token_id);
        env.storage().instance().set(&DataKey::K1A(symbol_short!("URep"), user.clone()), &metrics);
        
        let mut updated = milestone;
        updated.is_completed = true;
        updated.completion_timestamp = Some(env.ledger().timestamp());
        env.storage().instance().set(&DataKey::K1(symbol_short!("Mile"), milestone_id), &updated);
        
        token_id
    }

    pub fn update_credential_status(env: Env, token_id: u128, new_status: SbtStatus) {
        let admin: Address = env.storage().instance().get(&DataKey::K(symbol_short!("SbtAdm"))).unwrap();
        admin.require_auth();
        let mut credential: SoroSusuCredential = env.storage().instance().get(&DataKey::K1U(symbol_short!("Cred"), token_id)).unwrap();
        credential.status = new_status;
        env.storage().instance().set(&DataKey::K1U(symbol_short!("Cred"), token_id), &credential);
    }

    pub fn get_credential(env: Env, token_id: u128) -> SoroSusuCredential {
        env.storage().instance().get(&DataKey::K1U(symbol_short!("Cred"), token_id)).unwrap()
    }

    pub fn get_user_credential(env: Env, user: Address) -> Option<SoroSusuCredential> {
        let token_id: Option<u128> = env.storage().instance().get(&DataKey::K1A(symbol_short!("UCred"), user));
        token_id.map(|id| env.storage().instance().get(&DataKey::K1U(symbol_short!("Cred"), id)).unwrap())
    }

    pub fn create_reputation_milestone(env: Env, user: Address, cycles: u32, desc: String, tier: ReputationTier) -> u64 {
        let admin: Address = env.storage().instance().get(&DataKey::K(symbol_short!("SbtAdm"))).unwrap();
        admin.require_auth();
        let mut count: u64 = env.storage().instance().get(&DataKey::K(symbol_short!("MileCnt"))).unwrap_or(0);
        count += 1;
        let m = ReputationMilestone { milestone_id: count, user, cycles_required: cycles, description: desc, is_completed: false, completion_timestamp: None, reward_tier: tier };
        env.storage().instance().set(&DataKey::K(symbol_short!("MileCnt")), &count);
        env.storage().instance().set(&DataKey::K1(symbol_short!("Mile"), count), &m);
        count
    }

    pub fn get_reputation_milestone(env: Env, milestone_id: u64) -> ReputationMilestone {
        env.storage().instance().get(&DataKey::K1(symbol_short!("Mile"), milestone_id)).unwrap()
    }

    pub fn get_user_reputation_score(env: Env, user: Address) -> (u32, u32, u32) {
        let metrics: UserReputationMetrics = env.storage().instance().get(&DataKey::K1A(symbol_short!("URep"), user)).unwrap_or(UserReputationMetrics {
            reliability_score: 5000, social_capital_score: 5000, total_cycles: 0, perfect_cycles: 0, last_updated: 0, total_volume_saved: 0,
        });
        (metrics.reliability_score, metrics.social_capital_score, metrics.total_cycles)
    }
}

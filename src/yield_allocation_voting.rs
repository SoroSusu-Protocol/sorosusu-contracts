// Yield-Allocation Voting Module
// Governance hooks for yield distribution with Reliability Index weighting

use soroban_sdk::{Address, Env, Symbol, panic, Vec, i128, u64, u32, Map, BytesN};
use crate::{DataKey, CircleInfo, Member};
use super::yield_strategy_trait::{YieldStrategyTrait, YieldStrategyConfig, StrategyType};

// --- ERROR CODES ---

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VotingError {
    VotingNotActive = 401,
    AlreadyVoted = 402,
    InvalidStrategy = 403,
    VotingPeriodExpired = 404,
    Unauthorized = 405,
    InsufficientReliability = 406,
    InvalidVoteWeight = 407,
    TallyFailed = 408,
}

// --- DATA STRUCTURES ---

#[contracttype]
#[derive(Clone)]
pub struct DistributionStrategy {
    pub strategy_address: Address,
    pub allocation_percentage: u32, // In basis points (10000 = 100%)
    pub strategy_type: StrategyType,
    pub min_apy_bps: u32,
    pub risk_score: u32, // 0-10000, lower is safer
}

#[contracttype]
#[derive(Clone)]
pub struct Vote {
    pub voter: Address,
    pub circle_id: u64,
    pub voted_strategies: Vec<DistributionStrategy>,
    pub reliability_index: u32, // Voter's RI at time of vote (0-10000 bps)
    pub vote_weight: u64, // Calculated weight based on RI
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct VotingSession {
    pub circle_id: u64,
    pub start_timestamp: u64,
    pub end_timestamp: u64, // 48 hours after start
    pub total_votes_weight: u64,
    pub is_active: bool,
    pub winning_strategy: Option<Vec<DistributionStrategy>>,
    pub vote_threshold: u32, // Minimum participation threshold in bps
}

#[contracttype]
#[derive(Clone)]
pub struct VoteTally {
    pub strategy_hash: BytesN<32>, // Hash of the strategy array
    pub total_weight: u64,
    pub voter_count: u32,
}

// --- STORAGE KEYS ---

#[contracttype]
#[derive(Clone)]
pub enum VotingStorageKey {
    VotingSession(u64),      // circle_id -> VotingSession
    Vote(Address, u64),      // (voter, circle_id) -> Vote
    VoteTally(u64, BytesN<32>), // (circle_id, strategy_hash) -> VoteTally
    ActiveStrategies,        // Vec<Address> of available strategies
    VotingConfig,            // VotingConfig
}

#[contracttype]
#[derive(Clone)]
pub struct VotingConfig {
    pub voting_duration_seconds: u64,    // Default: 172800 (48 hours)
    pub min_participation_threshold: u32, // Default: 5000 (50%)
    pub min_reliability_threshold: u32,  // Default: 3000 (30%)
    pub weight_multiplier: u64,           // Multiplier for RI-based weighting
}

// --- CORE VOTING FUNCTIONS ---

/// Initialize voting session for a circle after yield cycle completes
pub fn initialize_voting_session(
    env: &Env,
    circle_id: u64,
    available_strategies: Vec<Address>,
) -> Result<(), VotingError> {
    // Check if circle exists and yield is enabled
    let circle: CircleInfo = env.storage().instance()
        .get(&DataKey::Circle(circle_id))
        .ok_or(VotingError::Unauthorized)?;
    
    if !circle.yield_enabled {
        return Err(VotingError::Unauthorized);
    }

    // Check if voting session already exists
    let session_key = VotingStorageKey::VotingSession(circle_id);
    if env.storage().instance().has(&session_key) {
        return Err(VotingError::VotingNotActive);
    }

    // Get voting config
    let config = get_voting_config(env);
    let current_time = env.ledger().timestamp();

    // Create new voting session
    let session = VotingSession {
        circle_id,
        start_timestamp: current_time,
        end_timestamp: current_time + config.voting_duration_seconds,
        total_votes_weight: 0,
        is_active: true,
        winning_strategy: None,
        vote_threshold: config.min_participation_threshold,
    };

    // Store voting session
    env.storage().instance().set(&session_key, &session);
    
    // Store available strategies for this session
    env.storage().instance().set(&VotingStorageKey::ActiveStrategies, &available_strategies);

    Ok(())
}

/// Cast a vote for yield distribution strategy
pub fn cast_vote(
    env: &Env,
    voter: Address,
    circle_id: u64,
    proposed_strategies: Vec<DistributionStrategy>,
) -> Result<(), VotingError> {
    // Authorization
    voter.require_auth();

    // Validate voting session
    let session_key = VotingStorageKey::VotingSession(circle_id);
    let mut session: VotingSession = env.storage().instance()
        .get(&session_key)
        .ok_or(VotingError::VotingNotActive)?;

    if !session.is_active {
        return Err(VotingError::VotingNotActive);
    }

    let current_time = env.ledger().timestamp();
    if current_time > session.end_timestamp {
        return Err(VotingError::VotingPeriodExpired);
    }

    // Check if already voted
    let vote_key = VotingStorageKey::Vote(voter.clone(), circle_id);
    if env.storage().instance().has(&vote_key) {
        return Err(VotingError::AlreadyVoted);
    }

    // Get voter's Reliability Index
    let reliability_index = get_reliability_index(env, &voter)?;
    let config = get_voting_config(env);
    
    if reliability_index < config.min_reliability_threshold {
        return Err(VotingError::InsufficientReliability);
    }

    // Validate proposed strategies
    validate_proposed_strategies(env, &proposed_strategies)?;

    // Calculate vote weight based on Reliability Index
    let vote_weight = calculate_vote_weight(reliability_index, config.weight_multiplier);

    // Create vote record
    let vote = Vote {
        voter: voter.clone(),
        circle_id,
        voted_strategies: proposed_strategies.clone(),
        reliability_index,
        vote_weight,
        timestamp: current_time,
    };

    // Store the vote
    env.storage().instance().set(&vote_key, &vote);

    // Update vote tally
    let strategy_hash = hash_strategy_array(env, &proposed_strategies);
    update_vote_tally(env, circle_id, strategy_hash, vote_weight, 1)?;

    // Update session totals
    session.total_votes_weight += vote_weight;
    env.storage().instance().set(&session_key, &session);

    Ok(())
}

/// Finalize voting and determine winning strategy
pub fn finalize_voting(
    env: &Env,
    circle_id: u64,
) -> Result<Vec<DistributionStrategy>, VotingError> {
    let session_key = VotingStorageKey::VotingSession(circle_id);
    let mut session: VotingSession = env.storage().instance()
        .get(&session_key)
        .ok_or(VotingError::VotingNotActive)?;

    if !session.is_active {
        return Err(VotingError::VotingNotActive);
    }

    let current_time = env.ledger().timestamp();
    if current_time < session.end_timestamp {
        return Err(VotingError::VotingPeriodExpired); // Still voting
    }

    // Check participation threshold
    let circle: CircleInfo = env.storage().instance()
        .get(&DataKey::Circle(circle_id))
        .ok_or(VotingError::Unauthorized)?;
    
    let max_possible_weight = circle.max_members as u64 * 10000; // Max weight if all have max RI
    let participation_rate = (session.total_votes_weight * 10000) / max_possible_weight;
    
    if participation_rate < session.vote_threshold as u64 {
        return Err(VotingError::TallyFailed);
    }

    // Find winning strategy (highest weighted votes)
    let winning_strategy = determine_winning_strategy(env, circle_id)?;
    
    // Update session
    session.winning_strategy = Some(winning_strategy.clone());
    session.is_active = false;
    env.storage().instance().set(&session_key, &session);

    Ok(winning_strategy)
}

/// Execute the winning distribution strategy
pub fn execute_distribution_strategy(
    env: &Env,
    circle_id: u64,
    total_yield_amount: i128,
) -> Result<(), VotingError> {
    let session_key = VotingStorageKey::VotingSession(circle_id);
    let session: VotingSession = env.storage().instance()
        .get(&session_key)
        .ok_or(VotingError::VotingNotActive)?;

    if session.is_active || session.winning_strategy.is_none() {
        return Err(VotingError::VotingNotActive);
    }

    let winning_strategy = session.winning_strategy.unwrap();

    // Execute each strategy in the winning distribution
    for strategy in winning_strategy.iter() {
        let allocation_amount = (total_yield_amount * strategy.allocation_percentage as i128) / 10000;
        
        if allocation_amount > 0 {
            // Call the yield strategy contract
            // This would be implemented using the YieldStrategyClient
            execute_strategy_allocation(env, &strategy, allocation_amount)?;
        }
    }

    Ok(())
}

// --- HELPER FUNCTIONS ---

fn get_voting_config(env: &Env) -> VotingConfig {
    env.storage().instance()
        .get(&VotingStorageKey::VotingConfig)
        .unwrap_or(VotingConfig {
            voting_duration_seconds: 172800, // 48 hours
            min_participation_threshold: 5000, // 50%
            min_reliability_threshold: 3000,  // 30%
            weight_multiplier: 100,
        })
}

fn get_reliability_index(env: &Env, user: &Address) -> Result<u32, VotingError> {
    // This would integrate with the Reliability Index oracle
    // For now, return a default value
    // In production, this would call: reliability_oracle.get_reliability_index(user)
    Ok(7500) // Default 75% reliability
}

fn calculate_vote_weight(reliability_index: u32, multiplier: u64) -> u64 {
    // Vote weight = RI * multiplier
    // Higher RI = more voting power
    (reliability_index as u64 * multiplier) / 10000
}

fn validate_proposed_strategies(
    env: &Env,
    strategies: &Vec<DistributionStrategy>,
) -> Result<(), VotingError> {
    // Check total allocation equals 100%
    let total_allocation: u32 = strategies.iter()
        .map(|s| s.allocation_percentage)
        .sum();
    
    if total_allocation != 10000 {
        return Err(VotingError::InvalidStrategy);
    }

    // Validate each strategy
    let active_strategies: Vec<Address> = env.storage().instance()
        .get(&VotingStorageKey::ActiveStrategies)
        .unwrap_or_else(|| Vec::new(env));

    for strategy in strategies.iter() {
        if !active_strategies.contains(&strategy.strategy_address) {
            return Err(VotingError::InvalidStrategy);
        }
        
        if strategy.allocation_percentage == 0 || strategy.allocation_percentage > 10000 {
            return Err(VotingError::InvalidStrategy);
        }
    }

    Ok(())
}

fn hash_strategy_array(env: &Env, strategies: &Vec<DistributionStrategy>) -> BytesN<32> {
    // Create a deterministic hash of the strategy array
    let mut hasher = env.crypto().sha256();
    
    for strategy in strategies.iter() {
        // Hash the address and allocation percentage
        let address_bytes = strategy.strategy_address.to_contract_storage_bucket();
        hasher.update(&address_bytes);
        
        let allocation_bytes = strategy.allocation_percentage.to_le_bytes();
        hasher.update(&allocation_bytes);
    }
    
    hasher.finalize()
}

fn update_vote_tally(
    env: &Env,
    circle_id: u64,
    strategy_hash: BytesN<32>,
    weight: u64,
    voter_count: u32,
) -> Result<(), VotingError> {
    let tally_key = VotingStorageKey::VoteTally(circle_id, strategy_hash);
    let mut tally: VoteTally = env.storage().instance()
        .get(&tally_key)
        .unwrap_or(VoteTally {
            strategy_hash,
            total_weight: 0,
            voter_count: 0,
        });

    tally.total_weight += weight;
    tally.voter_count += voter_count;
    
    env.storage().instance().set(&tally_key, &tally);
    Ok(())
}

fn determine_winning_strategy(
    env: &Env,
    circle_id: u64,
) -> Result<Vec<DistributionStrategy>, VotingError> {
    // Find the strategy hash with highest total weight
    let mut max_weight = 0u64;
    let mut winning_hash: Option<BytesN<32>> = None;

    // This would require iterating through all VoteTally entries for the circle
    // For simplicity, we'll use a basic approach here
    // In production, this would be more efficient with proper indexing
    
    // Placeholder implementation - would need proper storage iteration
    // For now, return the first valid strategy found
    panic!("Vote tally determination not fully implemented");
}

fn execute_strategy_allocation(
    env: &Env,
    strategy: &DistributionStrategy,
    amount: i128,
) -> Result<(), VotingError> {
    // This would call the external yield strategy contract
    // Using the YieldStrategyClient interface
    // For now, this is a placeholder
    
    // In production:
    // let client = YieldStrategyClient::new(&env, &strategy.strategy_address);
    // let deposit_params = DepositParams {
    //     amount,
    //     min_apy_bps: Some(strategy.min_apy_bps),
    //     lockup_period: None,
    //     auto_compound: true,
    // };
    // client.deposit(&env.current_contract_address(), amount, deposit_params);
    
    Ok(())
}

// --- ADMIN FUNCTIONS ---

pub fn set_voting_config(env: &Env, admin: Address, config: VotingConfig) -> Result<(), VotingError> {
    // Verify admin authorization
    let stored_admin: Address = env.storage().instance()
        .get(&DataKey::Admin)
        .ok_or(VotingError::Unauthorized)?;
    
    if admin != stored_admin {
        return Err(VotingError::Unauthorized);
    }

    env.storage().instance().set(&VotingStorageKey::VotingConfig, &config);
    Ok(())
}

// --- QUERY FUNCTIONS ---

pub fn get_voting_session(env: &Env, circle_id: u64) -> Result<VotingSession, VotingError> {
    env.storage().instance()
        .get(&VotingStorageKey::VotingSession(circle_id))
        .ok_or(VotingError::VotingNotActive)
}

pub fn get_user_vote(env: &Env, voter: Address, circle_id: u64) -> Result<Vote, VotingError> {
    env.storage().instance()
        .get(&VotingStorageKey::Vote(voter, circle_id))
        .ok_or(VotingError::AlreadyVoted) // Using this as "not found" error
}

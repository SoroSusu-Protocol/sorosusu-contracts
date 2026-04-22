#![no_std]
use soroban_sdk::{contract, contracttype, contractimpl, Address, Env, Symbol, token, String, Vec};

// --- DATA STRUCTURES ---

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Circle(u64),
    Member(Address),
    CircleCount,
    // New: Tracks if a user has paid for a specific circle (CircleID, UserAddress)
    Deposit(u64, Address),
    // New: Tracks Group Reserve balance for penalties
    GroupReserve,
    // New: Tracks next cycle contribution amount for each circle
    NextCycleAmount(u64),
    // New: Tracks claimable balances for each user in each circle
    ClaimableBalance(u64, Address),
    // New: Tracks co-winners configuration for each circle
    CoWinnersConfig(u64),
    // New: Tracks current round winners for each circle
    CurrentWinners(u64),
    // New: Tracks user reputation score for tiered access
    UserReputation(Address),
    // New: Tracks private contribution amounts for privacy masking
    PrivateContribution(u64, Address),
    // New: Tracks voting proposals
    VotingProposal(u64),
    // New: Tracks votes on proposals
    Vote(u64, Address),
    // New: Oracle heartbeat tracking
    OracleHeartbeat,
    // New: Trust mode status
    TrustMode,
    // New: Manual price settings for emergency mode
    ManualPrice(u64),
    // New: Cross-group liquidity vault
    LiquidityVault,
    // New: Loans between circles
    CircleLoan(u64), // loan_id
    // New: Variable interest rates
    CircleRiskLevel(u64),
    // New: Group lead performance bonds
    GroupLeadBond(u64),
    // New: Slashing proposals
    SlashingProposal(u64),
    // New: Staking collateral for high-value rounds
    StakedCollateral(u64, Address), // (circle_id, user_address)
    // New: Vault for collateral storage
    CollateralVault,
    // New: Reliability Index for users
    ReliabilityIndex(Address),
    // New: User activity tracking for RI calculation
    UserActivity(Address),
    // New: Credit tier configuration for RI-based access control
    CreditTierConfig,
    // New: User's current credit limit based on RI
    UserCreditLimit(Address),
    // New: Insurance claim proposals
    InsuranceProposal(u64),
    // New: Circle-specific reserve vaults
    CircleReserve(u64),
}

#[contracttype]
#[derive(Clone)]
pub struct Member {
    pub address: Address,
    pub has_contributed: bool,
    pub contribution_count: u32,
    pub last_contribution_time: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct CircleInfo {
    pub id: u64,
    pub creator: Address,
    pub contribution_amount: i128, // Changed back to i128 for token compatibility
    pub max_members: u32, // Changed from u16 to u32 for Soroban compatibility
    pub member_count: u32, // Changed from u16 to u32 for Soroban compatibility
    pub current_recipient_index: u32, // Changed from u16 to u32 for Soroban compatibility
    pub is_active: bool,
    pub token: Address, // The token used (USDC, XLM)
    pub deadline_timestamp: u64, // Deadline for on-time payments
    pub cycle_duration: u64, // Duration of each payment cycle in seconds
    // New: Fields for co-winners and tiered access
    pub max_co_winners: u32, // Maximum number of co-winners per round
    pub min_reputation_required: u64, // Minimum reputation score to join
}

// --- EVENTS ---

#[contracttype]
#[derive(Clone, Debug)]
pub struct AdminChangedEvent {
    pub old_admin: Address,
    pub new_admin: Address,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct CoWinnersConfig {
    pub enabled: bool,
    pub max_winners: u32,
    pub split_method: u32, // 0 = equal split, 1 = proportional to contributions
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ContributionMaskedEvent {
    pub member_id: Address,
    pub success: bool,
    // Amount is NOT included for privacy
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct VotingProposal {
    pub id: u64,
    pub circle_id: u64,
    pub proposal_type: u32, // 0 = meeting date change, 1 = new member, 2 = other
    pub description: String,
    pub proposer: Address,
    pub created_at: u64,
    pub voting_deadline: u64,
    pub yes_votes: u64,
    pub no_votes: u64,
    pub total_voting_power: u64,
    pub is_executed: bool,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct VoteCast {
    pub proposal_id: u64,
    pub voter: Address,
    pub vote: bool, // true = yes, false = no
    pub voting_power: u64,
}

// New: Oracle heartbeat structure
#[contracttype]
#[derive(Clone, Debug)]
pub struct OracleHeartbeat {
    pub last_heartbeat: u64,
    pub oracle_address: Address,
}

// New: Emergency price setting structure
#[contracttype]
#[derive(Clone, Debug)]
pub struct EmergencyPrice {
    pub circle_id: u64,
    pub price: i128,
    pub set_by: Address,
    pub timestamp: u64,
}

// New: Cross-group liquidity loan structure
#[contracttype]
#[derive(Clone, Debug)]
pub struct CircleLoan {
    pub loan_id: u64,
    pub from_circle_id: u64,
    pub to_circle_id: u64,
    pub amount: i128,
    pub interest_rate: u32, // basis points (100 = 1%)
    pub created_at: u64,
    pub due_at: u64,
    pub is_repaid: bool,
}

// New: Circle risk level for dynamic interest
#[contracttype]
#[derive(Clone, Debug)]
pub struct CircleRiskLevel {
    pub circle_id: u64,
    pub risk_score: u32, // 0-100, higher = riskier
    pub late_payments: u32,
    pub last_updated: u64,
}

// New: Group lead bond structure
#[contracttype]
#[derive(Clone, Debug)]
pub struct GroupLeadBond {
    pub circle_id: u64,
    pub lead_address: Address,
    pub bond_amount: i128,
    pub posted_at: u64,
    pub is_slashed: bool,
}

// New: Slashing proposal structure
#[contracttype]
#[derive(Clone, Debug)]
pub struct SlashingProposal {
    pub proposal_id: u64,
    pub circle_id: u64,
    pub target_lead: Address,
    pub reason: String,
    pub proposed_by: Address,
    pub created_at: u64,
    pub voting_deadline: u64,
    pub yes_votes: u32,
    pub no_votes: u32,
    pub total_members: u32,
    pub is_executed: bool,
}

// New: Collateral stake structure
#[contracttype]
#[derive(Clone, Debug)]
pub struct StakedCollateral {
    pub circle_id: u64,
    pub user: Address,
    pub amount: i128,
    pub staked_at: u64,
    pub is_slashed: bool,
    pub is_released: bool,
}

// New: User activity tracking for Reliability Index
#[contracttype]
#[derive(Clone, Debug)]
pub struct UserActivity {
    pub user: Address,
    pub timely_contributions: u32,
    pub late_contributions: u32,
    pub total_cycles_completed: u32,
    pub total_volume_contributed: i128,
    pub last_activity_time: u64,
    pub consecutive_cycles: u32,
    pub longest_streak: u32,
}

// New: Reliability Index structure
#[contracttype]
#[derive(Clone, Debug)]
pub struct ReliabilityIndex {
    pub user: Address,
    pub score: u32, // 0-1000
    pub last_updated: u64,
    pub decay_rate: u32, // basis points per day of inactivity
}

// New: Credit tier configuration for RI-based access control
#[contracttype]
#[derive(Clone, Debug)]
pub struct CreditTierConfig {
    pub tier_1_max: i128,   // RI 0-200: Max contribution limit
    pub tier_2_max: i128,   // RI 201-400: Max contribution limit
    pub tier_3_max: i128,   // RI 401-600: Max contribution limit
    pub tier_4_max: i128,   // RI 601-800: Max contribution limit
    pub tier_5_max: i128,   // RI 801-1000: Max contribution limit
}

// New: User credit limit structure
#[contracttype]
#[derive(Clone, Debug)]
pub struct UserCreditLimit {
    pub user: Address,
    pub max_contribution: i128,
    pub last_updated: u64,
    pub current_ri: u32,
}

// New: Circle reserve configuration
#[contracttype]
#[derive(Clone, Debug)]
pub struct CircleReserveConfig {
    pub circle_id: u64,
    pub reserve_percentage: u32, // 1-3% in basis points (100-300)
    pub total_reserve: i128,
    pub is_active: bool,
}

// New: Insurance claim proposal structure
#[contracttype]
#[derive(Clone, Debug)]
pub struct InsuranceProposal {
    pub proposal_id: u64,
    pub circle_id: u64,
    pub defaulting_member: Address,
    pub victim_recipient: Address,
    pub claim_amount: i128,
    pub proposer: Address,
    pub created_at: u64,
    pub voting_deadline: u64,
    pub yes_votes: u32,
    pub no_votes: u32,
    pub total_members: u32,
    pub is_executed: bool,
    pub reason: String,
}

// --- CONTRACT TRAIT ---

pub trait SoroSusuTrait {
    // Initialize the contract
    fn init(env: Env, admin: Address);
    
    // Create a new savings circle
    fn create_circle(env: Env, creator: Address, amount: i128, max_members: u32, token: Address, cycle_duration: u64, max_co_winners: u32, min_reputation_required: u64) -> u64;

    // Join an existing circle
    fn join_circle(env: Env, user: Address, circle_id: u64);

    // Make a deposit (Pay your weekly/monthly due)
    fn deposit(env: Env, user: Address, circle_id: u64, privacy_masked: bool);
    
    // Transfer admin role to another user
    fn transfer_admin(env: Env, current_admin: Address, new_admin: Address);
    
    // Set next cycle contribution amount (Admin only)
    fn set_next_cycle_amount(env: Env, admin: Address, circle_id: u64, amount: i128);
    
    // Configure co-winners for a circle (Admin only)
    fn configure_co_winners(env: Env, admin: Address, circle_id: u64, enabled: bool, max_winners: u32, split_method: u32);
    
    // Distribute funds to members with co-winners support (pull pattern)
    fn distribute_funds(env: Env, admin: Address, circle_id: u64, co_winners: Vec<Address>);
    
    // Claim funds from distribution
    fn claim(env: Env, user: Address, circle_id: u64);
    
    // Create a voting proposal
    fn create_proposal(env: Env, proposer: Address, circle_id: u64, proposal_type: u32, description: String, voting_deadline: u64) -> u64;
    
    // Vote on a proposal
    fn vote(env: Env, voter: Address, proposal_id: u64, vote: bool);
    
    // Execute a successful proposal
    fn execute_proposal(env: Env, executor: Address, proposal_id: u64);
    
    // Update user reputation (Admin only)
    fn update_reputation(env: Env, admin: Address, user: Address, reputation_score: u64);
    
    // Get private contribution amount (member only)
    fn get_private_contribution(env: Env, user: Address, circle_id: u64, target_member: Address) -> i128;
    
    // Emergency Manual Revert for Oracle Failure (#205)
    fn update_oracle_heartbeat(env: Env, oracle: Address);
    fn activate_trust_mode(env: Env, admin: Address);
    fn set_emergency_price(env: Env, circle_id: u64, price: i128, setter: Address);
    
    // Cross-Group Liquidity Sharing Vault (#204)
    fn create_liquidity_vault(env: Env, admin: Address);
    fn lend_to_circle(env: Env, from_circle_id: u64, to_circle_id: u64, amount: i128, interest_rate: u32, lead: Address) -> u64;
    fn repay_circle_loan(env: Env, circle_id: u64, loan_id: u64, lead: Address);
    
    // Variable Interest Rate for Internal Susu Lending (#203)
    fn update_circle_risk_level(env: Env, admin: Address, circle_id: u64, late_payments: u32);
    fn get_dynamic_interest_rate(env: Env, circle_id: u64) -> u32;
    
    // Group Lead Performance Bond Slashing (#202)
    fn post_lead_bond(env: Env, circle_id: u64, lead: Address, bond_amount: i128);
    fn create_slashing_proposal(env: Env, circle_id: u64, target_lead: Address, reason: String, proposer: Address) -> u64;
    fn vote_on_slashing(env: Env, voter: Address, proposal_id: u64, vote: bool);
    fn execute_slashing(env: Env, executor: Address, proposal_id: u64);
    
    // Staking-Gated Entry for High-Value Rounds (#268)
    fn stake_collateral(env: Env, user: Address, circle_id: u64, amount: i128);
    fn slash_stake(env: Env, admin: Address, circle_id: u64, target_user: Address, reason: String);
    fn release_collateral(env: Env, admin: Address, circle_id: u64, user: Address);
    fn get_collateral_status(env: Env, circle_id: u64, user: Address) -> StakedCollateral;
    
    // Reliability-Index (RI) Calculation Engine (#266)
    fn calculate_reliability_index(env: Env, user: Address) -> u32;
    fn update_user_activity(env: Env, user: Address, circle_id: u64, contribution_amount: i128, is_timely: bool);
    fn apply_ri_decay(env: Env, user: Address);
    fn get_reliability_index(env: Env, user: Address) -> ReliabilityIndex;
    
    // Credit-Limit Scaling based on Historical RI (#269)
    fn initialize_credit_tiers(env: Env, admin: Address);
    fn get_user_credit_limit(env: Env, user: Address) -> i128;
    fn update_user_credit_limit(env: Env, user: Address);
    fn can_join_pool(env: Env, user: Address, pool_max_contribution: i128) -> bool;
    
    // Emergency-Bailout Fund Logic (Susu Insurance) (#267)
    fn configure_circle_reserve(env: Env, admin: Address, circle_id: u64, reserve_percentage: u32);
    fn create_insurance_proposal(env: Env, proposer: Address, circle_id: u64, defaulting_member: Address, victim_recipient: Address, claim_amount: i128, reason: String) -> u64;
    fn vote_on_insurance(env: Env, voter: Address, proposal_id: u64, vote: bool);
    fn execute_insurance_claim(env: Env, executor: Address, proposal_id: u64);
    fn get_circle_reserve_balance(env: Env, circle_id: u64) -> i128;
}

// --- IMPLEMENTATION ---

#[contract]
pub struct SoroSusu;

#[contractimpl]
impl SoroSusuTrait for SoroSusu {
    fn init(env: Env, admin: Address) {
        // Initialize the circle counter to 0 if it doesn't exist
        if !env.storage().instance().has(&DataKey::CircleCount) {
            env.storage().instance().set(&DataKey::CircleCount, &0u64);
        }
        // Set the admin
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    fn create_circle(env: Env, creator: Address, amount: i128, max_members: u32, token: Address, cycle_duration: u64, max_co_winners: u32, min_reputation_required: u64) -> u64 {
        // 1. Get the current Circle Count
        let mut circle_count: u64 = env.storage().instance().get(&DataKey::CircleCount).unwrap_or(0);
        
        // 2. Increment the ID for the new circle
        circle_count += 1;

        // 3. Create the Circle Data Struct
        let current_time = env.ledger().timestamp();
        let new_circle = CircleInfo {
            id: circle_count,
            creator: creator.clone(),
            contribution_amount: amount,
            max_members,
            member_count: 0,
            current_recipient_index: 0,
            is_active: true,
            token,
            deadline_timestamp: current_time + cycle_duration,
            cycle_duration,
            max_co_winners,
            min_reputation_required,
        };

        // 4. Save the Circle and the new Count
        env.storage().instance().set(&DataKey::Circle(circle_count), &new_circle);
        env.storage().instance().set(&DataKey::CircleCount, &circle_count);

        // 5. Initialize Group Reserve if not exists
        if !env.storage().instance().has(&DataKey::GroupReserve) {
            env.storage().instance().set(&DataKey::GroupReserve, &0u64);
        }

        // 6. Initialize co-winners configuration
        let co_winners_config = CoWinnersConfig {
            enabled: max_co_winners > 1,
            max_winners: max_co_winners,
            split_method: 0, // Default to equal split
        };
        env.storage().instance().set(&DataKey::CoWinnersConfig(circle_count), &co_winners_config);

        // 7. Return the new ID
        circle_count
    }

    fn join_circle(env: Env, user: Address, circle_id: u64) {
        // 1. Authorization: The user MUST sign this transaction
        user.require_auth();

        // 2. Retrieve the circle data
        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();

        // 3. Check if the circle is full
        if circle.member_count >= circle.max_members {
            panic!("Circle is full");
        }

        // 4. Check if user is already a member to prevent duplicates
        let member_key = DataKey::Member(user.clone());
        if env.storage().instance().has(&member_key) {
            panic!("User is already a member");
        }

        // 5. Check user reputation against circle requirements
        if circle.min_reputation_required > 0 {
            let user_reputation: u64 = env.storage().instance().get(&DataKey::UserReputation(user.clone())).unwrap_or(0);
            if user_reputation < circle.min_reputation_required {
                panic!("User reputation is too low to join this circle");
            }
        }

        // 6. NEW: Credit-Limit Scaling based on Historical RI (#269)
        // Check if user can join based on their RI-defined credit limit
        if !Self::can_join_pool(env.clone(), user.clone(), circle.contribution_amount) {
            panic!("User's Reliability Index is insufficient for this pool's contribution amount");
        }

        // 7. NEW: Check for staking requirement for high-value rounds (>5,000 XLM)
        if circle.contribution_amount >= 5000_0000000 { // 5,000 XLM (assuming 7 decimals)
            let collateral_key = DataKey::StakedCollateral(circle_id, user.clone());
            let collateral: StakedCollateral = env.storage().instance().get(&collateral_key)
                .unwrap_or_else(|| panic!("Collateral stake required for high-value rounds"));
            
            if collateral.is_slashed {
                panic!("Collateral has been slashed, cannot join circle");
            }
        }

        // 8. Create and store the new member
        let new_member = Member {
            address: user.clone(),
            has_contributed: false,
            contribution_count: 0,
            last_contribution_time: 0,
        };
        
        // 9. Store the member and update circle count
        env.storage().instance().set(&member_key, &new_member);
        circle.member_count += 1;
        
        // 10. Save the updated circle back to storage
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn deposit(env: Env, user: Address, circle_id: u64, privacy_masked: bool) {
        // 1. Authorization: The user must sign this!
        user.require_auth();

        // 2. Load the Circle Data
        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();

        // 2.1. Check if there is a next cycle amount set
        let next_cycle_amount: Option<i128> = env.storage().instance().get(&DataKey::NextCycleAmount(circle_id));
        let contribution_amount = next_cycle_amount.unwrap_or(circle.contribution_amount);

        // 3. Check if user is actually a member
        let member_key = DataKey::Member(user.clone());
        let mut member: Member = env.storage().instance().get(&member_key)
            .unwrap_or_else(|| panic!("User is not a member of this circle"));

        // 4. Create the Token Client
        let client = token::Client::new(&env, &circle.token);

        // 5. Check if payment is late and apply penalty if needed
        let current_time = env.ledger().timestamp();
        let mut penalty_amount = 0i128;
        let is_timely = current_time <= circle.deadline_timestamp;

        if !is_timely {
            // Calculate 1% penalty
            penalty_amount = contribution_amount / 100; // 1% penalty
            
            // Update Group Reserve balance
            let mut reserve_balance: u64 = env.storage().instance().get(&DataKey::GroupReserve).unwrap_or(0);
            reserve_balance += penalty_amount as u64;
            env.storage().instance().set(&DataKey::GroupReserve, &reserve_balance);
        }

        // 6. NEW: Emergency-Bailout Fund Logic - Calculate and divert reserve percentage
        let circle_reserve_config: CircleReserveConfig = env.storage().instance().get(&DataKey::CircleReserve(circle_id))
            .unwrap_or_else(|| CircleReserveConfig {
                circle_id,
                reserve_percentage: 200, // Default 2%
                total_reserve: 0,
                is_active: true,
            });

        let reserve_amount = if circle_reserve_config.is_active {
            (contribution_amount * circle_reserve_config.reserve_percentage as i128) / 10000
        } else {
            0
        };

        let net_contribution = contribution_amount - reserve_amount;

        // 7. Transfer the full amount from user
        client.transfer(
            &user, 
            &env.current_contract_address(), 
            &contribution_amount
        );

        // 8. Update circle reserve if active
        if circle_reserve_config.is_active && reserve_amount > 0 {
            let mut updated_config = circle_reserve_config.clone();
            updated_config.total_reserve += reserve_amount;
            env.storage().instance().set(&DataKey::CircleReserve(circle_id), &updated_config);
        }

        // 7. Store private contribution amount if privacy is enabled
        if privacy_masked {
            env.storage().instance().set(&DataKey::PrivateContribution(circle_id, user.clone()), &contribution_amount);
        }

        // 8. Update member contribution info
        member.has_contributed = true;
        member.contribution_count += 1;
        member.last_contribution_time = current_time;
        
        // 9. Save updated member info
        env.storage().instance().set(&member_key, &member);

        // 10. Update circle contribution amount and deadline for next cycle
        if next_cycle_amount.is_some() {
            circle.contribution_amount = contribution_amount;
            // Clear the next cycle amount since it has been applied
            env.storage().instance().remove(&DataKey::NextCycleAmount(circle_id));
        }
        circle.deadline_timestamp = current_time + circle.cycle_duration;
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

        // 11. Mark as Paid in the old format for backward compatibility
        env.storage().instance().set(&DataKey::Deposit(circle_id, user.clone()), &true);

        // 13. NEW: Update user activity for reliability index calculation
        Self::update_user_activity(env.clone(), user.clone(), circle_id, net_contribution, is_timely);

        // 14. Emit contribution event (masked if privacy is enabled)
        if privacy_masked {
            let event = ContributionMaskedEvent {
                member_id: user.clone(),
                success: true,
            };
            env.events().publish((Symbol::new(&env, "contribution_masked"),), event);
        } else {
            // Emit regular contribution event with amount
            env.events().publish((Symbol::new(&env, "contribution"),), (user.clone(), net_contribution));
        }

        // 15. Emit reserve contribution event if applicable
        if circle_reserve_config.is_active && reserve_amount > 0 {
            env.events().publish((Symbol::new(&env, "reserve_contribution"),), (circle_id, user, reserve_amount));
        }
    }

    fn transfer_admin(env: Env, current_admin: Address, new_admin: Address) {
        // 1. Authorization: The current admin must sign this transaction
        current_admin.require_auth();

        // 2. Get the current admin from storage
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("No admin set"));

        // 3. Verify the caller is the current admin
        if stored_admin != current_admin {
            panic!("Caller is not the current admin");
        }

        // 4. Update the admin in storage
        env.storage().instance().set(&DataKey::Admin, &new_admin);

        // 5. Emit the AdminChanged event
        let event = AdminChangedEvent {
            old_admin: current_admin,
            new_admin: new_admin,
        };
        env.events().publish((Symbol::new(&env, "admin_changed"),), event);
    }

    fn set_next_cycle_amount(env: Env, admin: Address, circle_id: u64, amount: i128) {
        // 1. Authorization: The admin must sign this transaction
        admin.require_auth();

        // 2. Verify the caller is the admin
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("No admin set"));
        
        if stored_admin != admin {
            panic!("Caller is not the admin");
        }

        // 3. Verify the circle exists
        let _circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));

        // 4. Set the next cycle amount
        env.storage().instance().set(&DataKey::NextCycleAmount(circle_id), &amount);
    }

    fn configure_co_winners(env: Env, admin: Address, circle_id: u64, enabled: bool, max_winners: u32, split_method: u32) {
        // 1. Authorization: The admin must sign this transaction
        admin.require_auth();

        // 2. Verify the caller is the admin
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("No admin set"));
        
        if stored_admin != admin {
            panic!("Caller is not the admin");
        }

        // 3. Verify the circle exists
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));

        // 4. Validate max_winners doesn't exceed member count
        if max_winners > circle.member_count {
            panic!("Max winners cannot exceed member count");
        }

        // 5. Create and store co-winners configuration
        let co_winners_config = CoWinnersConfig {
            enabled,
            max_winners,
            split_method,
        };
        env.storage().instance().set(&DataKey::CoWinnersConfig(circle_id), &co_winners_config);
    }

    fn distribute_funds(env: Env, admin: Address, circle_id: u64, co_winners: Vec<Address>) {
        // 1. Authorization: The admin must sign this transaction
        admin.require_auth();

        // 2. Verify the caller is the admin
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("No admin set"));
        
        if stored_admin != admin {
            panic!("Caller is not the admin");
        }

        // 3. Get the circle info
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));

        // 4. Get co-winners configuration
        let co_winners_config: CoWinnersConfig = env.storage().instance().get(&DataKey::CoWinnersConfig(circle_id))
            .unwrap_or_else(|| CoWinnersConfig {
                enabled: false,
                max_winners: 1,
                split_method: 0,
            });

        // 5. Calculate total pool amount (simplified - in real implementation, 
        // this would track actual deposited funds)
        let total_pool = circle.contribution_amount * circle.member_count as i128;

        // 6. Handle co-winners logic
        if co_winners_config.enabled && !co_winners.is_empty() {
            // Validate co-winners count
            if co_winners.len() as u32 > co_winners_config.max_winners {
                panic!("Too many co-winners specified");
            }

            // Calculate shares based on split method
            let mut dust_amount = 0i128;
            let mut shares = soroban_sdk::Vec::<i128>::new(&env);
            
            if co_winners_config.split_method == 0 {
                // Equal split
                let base_share = total_pool / co_winners.len() as i128;
                dust_amount = total_pool - (base_share * co_winners.len() as i128);
                let co_winners_count = co_winners.len() as u32;
                for _ in 0u32..co_winners_count {
                    shares.push_back(base_share);
                }
            } else {
                // Proportional split based on contributions
                let mut total_private_contributions = 0i128;
                let mut contributions = soroban_sdk::Vec::<i128>::new(&env);
                
                let co_winners_count = co_winners.len() as u32;
                for i in 0u32..co_winners_count {
                    let winner = &co_winners.get_unchecked(i);
                    let contrib: i128 = env.storage().instance()
                        .get(&DataKey::PrivateContribution(circle_id, winner.clone()))
                        .unwrap_or_else(|| circle.contribution_amount);
                    contributions.push_back(contrib);
                    total_private_contributions += contrib;
                }
                
                let contributions_count = contributions.len() as u32;
                for i in 0u32..contributions_count {
                    let contrib = contributions.get_unchecked(i);
                    let share = (contrib * total_pool) / total_private_contributions;
                    shares.push_back(share);
                }
                
                // Calculate dust
                let mut total_distributed = 0i128;
                let shares_count = shares.len() as u32;
                for i in 0u32..shares_count {
                    total_distributed += shares.get_unchecked(i);
                }
                dust_amount = total_pool - total_distributed;
            }

            // Add dust to first co-winner or group reserve
            if dust_amount > 0 {
                let mut updated_shares = shares.clone();
                let first_share = updated_shares.get_unchecked(0) + dust_amount;
                updated_shares.set(0, first_share);
                
                // Set claimable balances for co-winners
                let co_winners_count = co_winners.len() as u32;
                for i in 0u32..co_winners_count {
                    let winner = &co_winners.get_unchecked(i);
                    let share_amount = updated_shares.get_unchecked(i);
                    env.storage().instance().set(&DataKey::ClaimableBalance(circle_id, winner.clone()), &share_amount);
                }
            } else {
                // Set claimable balances for co-winners
                let co_winners_count = co_winners.len() as u32;
                for i in 0u32..co_winners_count {
                    let winner = &co_winners.get_unchecked(i);
                    let share_amount = shares.get_unchecked(i);
                    env.storage().instance().set(&DataKey::ClaimableBalance(circle_id, winner.clone()), &share_amount);
                }
            }

            // Store current winners for record
            env.storage().instance().set(&DataKey::CurrentWinners(circle_id), &co_winners);
        } else {
            // Single winner logic (backwards compatibility)
            let share_per_member = total_pool / circle.member_count as i128;
            env.storage().instance().set(&DataKey::ClaimableBalance(circle_id, admin), &share_per_member);
        }
    }

    fn claim(env: Env, user: Address, circle_id: u64) {
        // 1. Authorization: The user must sign this transaction
        user.require_auth();

        // 2. Get the claimable balance for this user
        let claimable_balance: i128 = env.storage().instance().get(&DataKey::ClaimableBalance(circle_id, user.clone()))
            .unwrap_or_else(|| panic!("No claimable balance for this user"));

        // 3. Get the circle info to get the token address
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));

        // 4. Create the token client
        let client = token::Client::new(&env, &circle.token);

        // 5. Transfer the funds to the user
        client.transfer(
            &env.current_contract_address(),
            &user,
            &claimable_balance,
        );

        // 6. Clear the claimable balance
        env.storage().instance().set(&DataKey::ClaimableBalance(circle_id, user), &0i128);
    }

    fn create_proposal(env: Env, proposer: Address, circle_id: u64, proposal_type: u32, description: String, voting_deadline: u64) -> u64 {
        // 1. Authorization: The proposer must sign this transaction
        proposer.require_auth();

        // 2. Verify the circle exists and user is a member
        let _circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));
        
        let member_key = DataKey::Member(proposer.clone());
        let _member: Member = env.storage().instance().get(&member_key)
            .unwrap_or_else(|| panic!("User is not a member of this circle"));

        // 3. Get proposal ID (increment counter)
        let mut proposal_count: u64 = env.storage().instance().get(&DataKey::CircleCount).unwrap_or(0);
        proposal_count += 1;

        // 4. Calculate proposer's voting power
        let proposer_reputation: u64 = env.storage().instance().get(&DataKey::UserReputation(proposer.clone())).unwrap_or(0);
        let voting_power = proposer_reputation + 100; // Base power + reputation

        // 5. Create the proposal
        let current_time = env.ledger().timestamp();
        let proposal = VotingProposal {
            id: proposal_count,
            circle_id,
            proposal_type,
            description,
            proposer: proposer.clone(),
            created_at: current_time,
            voting_deadline,
            yes_votes: 0,
            no_votes: 0,
            total_voting_power: voting_power,
            is_executed: false,
        };

        // 6. Store the proposal
        env.storage().instance().set(&DataKey::VotingProposal(proposal_count), &proposal);

        // 7. Return proposal ID
        proposal_count
    }

    fn vote(env: Env, voter: Address, proposal_id: u64, vote: bool) {
        // 1. Authorization: The voter must sign this transaction
        voter.require_auth();

        // 2. Get the proposal
        let mut proposal: VotingProposal = env.storage().instance().get(&DataKey::VotingProposal(proposal_id))
            .unwrap_or_else(|| panic!("Proposal does not exist"));

        // 3. Check if voting is still open
        let current_time = env.ledger().timestamp();
        if current_time > proposal.voting_deadline {
            panic!("Voting period has ended");
        }

        // 4. Check if user has already voted
        let vote_key = DataKey::Vote(proposal_id, voter.clone());
        if env.storage().instance().has(&vote_key) {
            panic!("User has already voted on this proposal");
        }

        // 5. Calculate voter's voting power
        let voter_reputation: u64 = env.storage().instance().get(&DataKey::UserReputation(voter.clone())).unwrap_or(0);
        let voting_power = voter_reputation + 100; // Base power + reputation

        // 6. Record the vote
        let vote_record = VoteCast {
            proposal_id,
            voter: voter.clone(),
            vote,
            voting_power,
        };
        env.storage().instance().set(&vote_key, &vote_record);

        // 7. Update proposal vote counts
        if vote {
            proposal.yes_votes += voting_power;
        } else {
            proposal.no_votes += voting_power;
        }
        proposal.total_voting_power += voting_power;

        // 8. Save updated proposal
        env.storage().instance().set(&DataKey::VotingProposal(proposal_id), &proposal);
    }

    fn execute_proposal(env: Env, executor: Address, proposal_id: u64) {
        // 1. Authorization: The executor must sign this transaction
        executor.require_auth();

        // 2. Get the proposal
        let mut proposal: VotingProposal = env.storage().instance().get(&DataKey::VotingProposal(proposal_id))
            .unwrap_or_else(|| panic!("Proposal does not exist"));

        // 3. Check if proposal has already been executed
        if proposal.is_executed {
            panic!("Proposal has already been executed");
        }

        // 4. Check if voting period has ended
        let current_time = env.ledger().timestamp();
        if current_time <= proposal.voting_deadline {
            panic!("Voting period has not ended yet");
        }

        // 5. Check if proposal passed (simple majority)
        if proposal.yes_votes <= proposal.no_votes {
            panic!("Proposal did not pass");
        }

        // 6. Execute proposal based on type
        match proposal.proposal_type {
            0 => {
                // Meeting date change - implementation would go here
                // This is a placeholder for actual logic
            },
            1 => {
                // New member admission - implementation would go here
                // This is a placeholder for actual logic
            },
            _ => {
                // Other proposal types
            }
        }

        // 7. Mark proposal as executed
        proposal.is_executed = true;
        env.storage().instance().set(&DataKey::VotingProposal(proposal_id), &proposal);
    }

    fn update_reputation(env: Env, admin: Address, user: Address, reputation_score: u64) {
        // 1. Authorization: The admin must sign this transaction
        admin.require_auth();

        // 2. Verify the caller is the admin
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("No admin set"));
        
        if stored_admin != admin {
            panic!("Caller is not the admin");
        }

        // 3. Update user reputation
        env.storage().instance().set(&DataKey::UserReputation(user), &reputation_score);
    }

    fn get_private_contribution(env: Env, user: Address, circle_id: u64, target_member: Address) -> i128 {
        // 1. Authorization: The user must sign this transaction
        user.require_auth();

        // 2. Verify the user is a member of the circle
        let member_key = DataKey::Member(user.clone());
        let _member: Member = env.storage().instance().get(&member_key)
            .unwrap_or_else(|| panic!("User is not a member of this circle"));

        // 3. Get the private contribution amount
        let contribution: i128 = env.storage().instance()
            .get(&DataKey::PrivateContribution(circle_id, target_member))
            .unwrap_or_else(|| panic!("Private contribution not found for target member"));

        // 4. Return the contribution amount
        contribution
    }

    // Emergency Manual Revert for Oracle Failure (#205)
    fn update_oracle_heartbeat(env: Env, oracle: Address) {
        oracle.require_auth();
        let current_time = env.ledger().timestamp();
        let heartbeat = OracleHeartbeat {
            last_heartbeat: current_time,
            oracle_address: oracle,
        };
        env.storage().instance().set(&DataKey::OracleHeartbeat, &heartbeat);
    }

    fn activate_trust_mode(env: Env, admin: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("No admin set"));
        
        if stored_admin != admin {
            panic!("Caller is not the admin");
        }

        let heartbeat: OracleHeartbeat = env.storage().instance().get(&DataKey::OracleHeartbeat)
            .unwrap_or_else(|| panic!("No oracle heartbeat found"));
        
        let current_time = env.ledger().timestamp();
        let hours_since_heartbeat = (current_time - heartbeat.last_heartbeat) / 3600;
        
        if hours_since_heartbeat < 72 {
            panic!("Trust mode can only be activated after 72 hours of oracle silence");
        }

        env.storage().instance().set(&DataKey::TrustMode, &true);
    }

    fn set_emergency_price(env: Env, circle_id: u64, price: i128, setter: Address) {
        setter.require_auth();
        
        let trust_mode: bool = env.storage().instance().get(&DataKey::TrustMode).unwrap_or(false);
        if !trust_mode {
            panic!("Emergency pricing only available in trust mode");
        }

        let _circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));

        let current_time = env.ledger().timestamp();
        let emergency_price = EmergencyPrice {
            circle_id,
            price,
            set_by: setter,
            timestamp: current_time,
        };
        env.storage().instance().set(&DataKey::ManualPrice(circle_id), &emergency_price);
    }

    // Cross-Group Liquidity Sharing Vault (#204)
    fn create_liquidity_vault(env: Env, admin: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("No admin set"));
        
        if stored_admin != admin {
            panic!("Caller is not the admin");
        }

        if !env.storage().instance().has(&DataKey::LiquidityVault) {
            env.storage().instance().set(&DataKey::LiquidityVault, &0i128);
        }
    }

    fn lend_to_circle(env: Env, from_circle_id: u64, to_circle_id: u64, amount: i128, interest_rate: u32, lead: Address) -> u64 {
        lead.require_auth();
        
        let from_circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(from_circle_id))
            .unwrap_or_else(|| panic!("Source circle does not exist"));
        let _to_circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(to_circle_id))
            .unwrap_or_else(|| panic!("Target circle does not exist"));

        let current_time = env.ledger().timestamp();
        let loan_duration = 30 * 24 * 3600; // 30 days
        let due_at = current_time + loan_duration;

        let mut loan_count: u64 = env.storage().instance().get(&DataKey::CircleCount).unwrap_or(0);
        loan_count += 1;

        let loan = CircleLoan {
            loan_id: loan_count,
            from_circle_id,
            to_circle_id,
            amount,
            interest_rate,
            created_at: current_time,
            due_at,
            is_repaid: false,
        };

        env.storage().instance().set(&DataKey::CircleLoan(loan_count), &loan);

        let client = token::Client::new(&env, &from_circle.token);
        client.transfer(&env.current_contract_address(), &env.current_contract_address(), &amount);

        loan_count
    }

    fn repay_circle_loan(env: Env, circle_id: u64, loan_id: u64, lead: Address) {
        lead.require_auth();
        
        let mut loan: CircleLoan = env.storage().instance().get(&DataKey::CircleLoan(loan_id))
            .unwrap_or_else(|| panic!("Loan does not exist"));

        if loan.is_repaid {
            panic!("Loan already repaid");
        }

        if loan.to_circle_id != circle_id {
            panic!("This loan does not belong to your circle");
        }

        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));

        let interest = (loan.amount * loan.interest_rate as i128) / 10000;
        let total_repayment = loan.amount + interest;

        let client = token::Client::new(&env, &circle.token);
        client.transfer(&env.current_contract_address(), &env.current_contract_address(), &total_repayment);

        loan.is_repaid = true;
        env.storage().instance().set(&DataKey::CircleLoan(loan_id), &loan);
    }

    // Variable Interest Rate for Internal Susu Lending (#203)
    fn update_circle_risk_level(env: Env, admin: Address, circle_id: u64, late_payments: u32) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("No admin set"));
        
        if stored_admin != admin {
            panic!("Caller is not the admin");
        }

        let _circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));

        let current_time = env.ledger().timestamp();
        let risk_score = if late_payments == 0 { 0 } else if late_payments <= 2 { 25 } else if late_payments <= 5 { 50 } else if late_payments <= 10 { 75 } else { 100 };

        let risk_level = CircleRiskLevel {
            circle_id,
            risk_score,
            late_payments,
            last_updated: current_time,
        };

        env.storage().instance().set(&DataKey::CircleRiskLevel(circle_id), &risk_level);
    }

    fn get_dynamic_interest_rate(env: Env, circle_id: u64) -> u32 {
        let risk_level: CircleRiskLevel = env.storage().instance().get(&DataKey::CircleRiskLevel(circle_id))
            .unwrap_or_else(|| CircleRiskLevel {
                circle_id,
                risk_score: 0,
                late_payments: 0,
                last_updated: 0,
            });

        let base_rate = 200u32; // 2% base rate
        let max_rate = 1000u32; // 10% max rate
        
        let additional_rate = (risk_level.risk_score * (max_rate - base_rate)) / 100;
        base_rate + additional_rate
    }

    // Group Lead Performance Bond Slashing (#202)
    fn post_lead_bond(env: Env, circle_id: u64, lead: Address, bond_amount: i128) {
        lead.require_auth();
        
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));

        if circle.creator != lead {
            panic!("Only circle creator can post bond");
        }

        let current_time = env.ledger().timestamp();
        let bond = GroupLeadBond {
            circle_id,
            lead_address: lead.clone(),
            bond_amount,
            posted_at: current_time,
            is_slashed: false,
        };

        env.storage().instance().set(&DataKey::GroupLeadBond(circle_id), &bond);

        let client = token::Client::new(&env, &circle.token);
        client.transfer(&lead, &env.current_contract_address(), &bond_amount);
    }

    fn create_slashing_proposal(env: Env, circle_id: u64, target_lead: Address, reason: String, proposer: Address) -> u64 {
        proposer.require_auth();
        
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));

        let member_key = DataKey::Member(proposer.clone());
        let _member: Member = env.storage().instance().get(&member_key)
            .unwrap_or_else(|| panic!("User is not a member of this circle"));

        let bond: GroupLeadBond = env.storage().instance().get(&DataKey::GroupLeadBond(circle_id))
            .unwrap_or_else(|| panic!("No bond found for this circle"));

        if bond.lead_address != target_lead {
            panic!("Target is not the lead of this circle");
        }

        let mut proposal_count: u64 = env.storage().instance().get(&DataKey::CircleCount).unwrap_or(0);
        proposal_count += 1;

        let current_time = env.ledger().timestamp();
        let voting_deadline = current_time + (7 * 24 * 3600); // 7 days

        let proposal = SlashingProposal {
            proposal_id: proposal_count,
            circle_id,
            target_lead,
            reason,
            proposed_by: proposer,
            created_at: current_time,
            voting_deadline,
            yes_votes: 0,
            no_votes: 0,
            total_members: circle.member_count,
            is_executed: false,
        };

        env.storage().instance().set(&DataKey::SlashingProposal(proposal_count), &proposal);
        proposal_count
    }

    fn vote_on_slashing(env: Env, voter: Address, proposal_id: u64, vote: bool) {
        voter.require_auth();
        
        let mut proposal: SlashingProposal = env.storage().instance().get(&DataKey::SlashingProposal(proposal_id))
            .unwrap_or_else(|| panic!("Slashing proposal does not exist"));

        let current_time = env.ledger().timestamp();
        if current_time > proposal.voting_deadline {
            panic!("Voting period has ended");
        }

        let member_key = DataKey::Member(voter.clone());
        let _member: Member = env.storage().instance().get(&member_key)
            .unwrap_or_else(|| panic!("User is not a member of this circle"));

        let vote_key = DataKey::Vote(proposal_id, voter.clone());
        if env.storage().instance().has(&vote_key) {
            panic!("User has already voted on this proposal");
        }

        let vote_record = VoteCast {
            proposal_id,
            voter: voter.clone(),
            vote,
            voting_power: 1,
        };
        env.storage().instance().set(&vote_key, &vote_record);

        if vote {
            proposal.yes_votes += 1;
        } else {
            proposal.no_votes += 1;
        }

        env.storage().instance().set(&DataKey::SlashingProposal(proposal_id), &proposal);
    }

    fn execute_slashing(env: Env, executor: Address, proposal_id: u64) {
        executor.require_auth();
        
        let mut proposal: SlashingProposal = env.storage().instance().get(&DataKey::SlashingProposal(proposal_id))
            .unwrap_or_else(|| panic!("Slashing proposal does not exist"));

        if proposal.is_executed {
            panic!("Proposal has already been executed");
        }

        let current_time = env.ledger().timestamp();
        if current_time <= proposal.voting_deadline {
            panic!("Voting period has not ended yet");
        }

        let required_votes = (proposal.total_members * 90) / 100; // 90% threshold
        if proposal.yes_votes < required_votes {
            panic!("Insufficient votes for slashing (90% required)");
        }

        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(proposal.circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));

        let mut bond: GroupLeadBond = env.storage().instance().get(&DataKey::GroupLeadBond(proposal.circle_id))
            .unwrap_or_else(|| panic!("No bond found for this circle"));

        if bond.is_slashed {
            panic!("Bond already slashed");
        }

        let _client = token::Client::new(&env, &circle.token);
        
        let _slash_per_member = bond.bond_amount / proposal.total_members as i128;
        
        env.events().publish((Symbol::new(&env, "bond_slashed"),), (proposal.circle_id, bond.bond_amount, proposal.total_members));
        
        bond.is_slashed = true;
        env.storage().instance().set(&DataKey::GroupLeadBond(proposal.circle_id), &bond);

        proposal.is_executed = true;
        env.storage().instance().set(&DataKey::SlashingProposal(proposal_id), &proposal);
    }

    // Staking-Gated Entry for High-Value Rounds (#268)
    fn stake_collateral(env: Env, user: Address, circle_id: u64, amount: i128) {
        // 1. Authorization: The user must sign this transaction
        user.require_auth();

        // 2. Verify the circle exists
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));

        // 3. Check if this is a high-value round
        if circle.contribution_amount < 5000_0000000 {
            panic!("Collateral staking only required for high-value rounds (>5,000 XLM)");
        }

        // 4. Check if user already has collateral staked
        let collateral_key = DataKey::StakedCollateral(circle_id, user.clone());
        if env.storage().instance().has(&collateral_key) {
            panic!("Collateral already staked for this circle");
        }

        // 5. Calculate minimum collateral requirement (20% of contribution amount)
        let min_collateral = (circle.contribution_amount * 20) / 100;
        if amount < min_collateral {
            panic!("Collateral amount below minimum requirement");
        }

        // 6. Create collateral stake record
        let current_time = env.ledger().timestamp();
        let collateral = StakedCollateral {
            circle_id,
            user: user.clone(),
            amount,
            staked_at: current_time,
            is_slashed: false,
            is_released: false,
        };

        // 7. Transfer collateral to vault
        let client = token::Client::new(&env, &circle.token);
        client.transfer(&user, &env.current_contract_address(), &amount);

        // 8. Update collateral vault balance
        let mut vault_balance: i128 = env.storage().instance().get(&DataKey::CollateralVault).unwrap_or(0);
        vault_balance += amount;
        env.storage().instance().set(&DataKey::CollateralVault, &vault_balance);

        // 9. Store collateral stake
        env.storage().instance().set(&collateral_key, &collateral);

        // 10. Emit staking event
        env.events().publish((Symbol::new(&env, "collateral_staked"),), (circle_id, user, amount));
    }

    fn slash_stake(env: Env, admin: Address, circle_id: u64, target_user: Address, reason: String) {
        // 1. Authorization: The admin must sign this transaction
        admin.require_auth();

        // 2. Verify the caller is the admin
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("No admin set"));
        
        if stored_admin != admin {
            panic!("Caller is not the admin");
        }

        // 3. Get the collateral stake
        let collateral_key = DataKey::StakedCollateral(circle_id, target_user.clone());
        let mut collateral: StakedCollateral = env.storage().instance().get(&collateral_key)
            .unwrap_or_else(|| panic!("No collateral found for this user"));

        // 4. Check if collateral is already slashed or released
        if collateral.is_slashed {
            panic!("Collateral already slashed");
        }
        if collateral.is_released {
            panic!("Collateral already released");
        }

        // 5. Mark collateral as slashed
        collateral.is_slashed = true;
        env.storage().instance().set(&collateral_key, &collateral);

        // 6. Update vault balance (remove slashed amount)
        let mut vault_balance: i128 = env.storage().instance().get(&DataKey::CollateralVault).unwrap_or(0);
        vault_balance -= collateral.amount;
        env.storage().instance().set(&DataKey::CollateralVault, &vault_balance);

        // 7. Transfer slashed collateral to group reserve for victim compensation
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));
        
        let client = token::Client::new(&env, &circle.token);
        client.transfer(&env.current_contract_address(), &env.current_contract_address(), &collateral.amount);

        // 8. Update group reserve
        let mut reserve_balance: u64 = env.storage().instance().get(&DataKey::GroupReserve).unwrap_or(0);
        reserve_balance += collateral.amount as u64;
        env.storage().instance().set(&DataKey::GroupReserve, &reserve_balance);

        // 9. Emit slashing event
        env.events().publish((Symbol::new(&env, "collateral_slashed"),), (circle_id, target_user, collateral.amount, reason));
    }

    fn release_collateral(env: Env, admin: Address, circle_id: u64, user: Address) {
        // 1. Authorization: The admin must sign this transaction
        admin.require_auth();

        // 2. Verify the caller is the admin
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("No admin set"));
        
        if stored_admin != admin {
            panic!("Caller is not the admin");
        }

        // 3. Get the collateral stake
        let collateral_key = DataKey::StakedCollateral(circle_id, user.clone());
        let mut collateral: StakedCollateral = env.storage().instance().get(&collateral_key)
            .unwrap_or_else(|| panic!("No collateral found for this user"));

        // 4. Check if collateral is already released or slashed
        if collateral.is_released {
            panic!("Collateral already released");
        }
        if collateral.is_slashed {
            panic!("Cannot release slashed collateral");
        }

        // 5. Mark collateral as released
        collateral.is_released = true;
        env.storage().instance().set(&collateral_key, &collateral);

        // 6. Update vault balance
        let mut vault_balance: i128 = env.storage().instance().get(&DataKey::CollateralVault).unwrap_or(0);
        vault_balance -= collateral.amount;
        env.storage().instance().set(&DataKey::CollateralVault, &vault_balance);

        // 7. Transfer collateral back to user
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));
        
        let client = token::Client::new(&env, &circle.token);
        client.transfer(&env.current_contract_address(), &user, &collateral.amount);

        // 8. Emit release event
        env.events().publish((Symbol::new(&env, "collateral_released"),), (circle_id, user, collateral.amount));
    }

    fn get_collateral_status(env: Env, circle_id: u64, user: Address) -> StakedCollateral {
        let collateral_key = DataKey::StakedCollateral(circle_id, user);
        env.storage().instance().get(&collateral_key)
            .unwrap_or_else(|| panic!("No collateral found for this user"))
    }

    // Reliability-Index (RI) Calculation Engine (#266)
    fn calculate_reliability_index(env: Env, user: Address) -> u32 {
        // 1. Get user activity data
        let activity_key = DataKey::UserActivity(user.clone());
        let activity: UserActivity = env.storage().instance().get(&activity_key)
            .unwrap_or_else(|| UserActivity {
                user: user.clone(),
                timely_contributions: 0,
                late_contributions: 0,
                total_cycles_completed: 0,
                total_volume_contributed: 0,
                last_activity_time: 0,
                consecutive_cycles: 0,
                longest_streak: 0,
            });

        // 2. Calculate timeliness score (40% weight)
        let total_contributions = activity.timely_contributions + activity.late_contributions;
        let timeliness_score = if total_contributions > 0 {
            (activity.timely_contributions * 400) / total_contributions
        } else {
            0
        };

        // 3. Calculate volume score (30% weight) - based on total volume contributed
        let volume_score = {
            let volume_factor = activity.total_volume_contributed / 1000_0000000; // Convert to XLM units
            if volume_factor > 1000 {
                300 // Max volume score
            } else {
                ((volume_factor * 300) / 1000) as u32
            }
        };

        // 4. Calculate frequency score (20% weight) - based on cycles completed and streak
        let frequency_score = {
            let cycles_score = if activity.total_cycles_completed > 50 {
                100 // Max cycles score
            } else {
                (activity.total_cycles_completed * 2) // 2 points per cycle
            };
            let streak_score = if activity.longest_streak > 20 {
                100 // Max streak score
            } else {
                activity.longest_streak * 5 // 5 points per streak
            };
            (cycles_score + streak_score).min(200)
        };

        // 5. Calculate consistency score (10% weight) - based on consecutive cycles
        let consistency_score = if activity.consecutive_cycles > 10 {
            100 // Max consistency score
        } else {
            activity.consecutive_cycles * 10 // 10 points per consecutive cycle
        };

        // 6. Combine all scores
        let base_score = timeliness_score + volume_score + frequency_score + consistency_score;
        
        // 7. Apply decay for inactivity
        let current_time = env.ledger().timestamp();
        let days_inactive = if activity.last_activity_time > 0 {
            (current_time - activity.last_activity_time) / (24 * 3600) // Convert to days
        } else {
            0
        };
        
        let decay_amount = if days_inactive > 30 {
            // Start decay after 30 days of inactivity
            let extra_days = days_inactive - 30;
            (extra_days * 5) // 5 points per day of inactivity
        } else {
            0
        };

        // 8. Final score calculation
        let final_score = if base_score > decay_amount as u32 {
            base_score - decay_amount as u32
        } else {
            0
        }.min(1000); // Cap at 1000

        // 9. Store the updated reliability index
        let ri = ReliabilityIndex {
            user: user.clone(),
            score: final_score,
            last_updated: current_time,
            decay_rate: 5, // 5 basis points per day
        };
        env.storage().instance().set(&DataKey::ReliabilityIndex(user), &ri);

        final_score
    }

    fn update_user_activity(env: Env, user: Address, _circle_id: u64, contribution_amount: i128, is_timely: bool) {
        // 1. Get existing activity or create new
        let activity_key = DataKey::UserActivity(user.clone());
        let mut activity: UserActivity = env.storage().instance().get(&activity_key)
            .unwrap_or_else(|| UserActivity {
                user: user.clone(),
                timely_contributions: 0,
                late_contributions: 0,
                total_cycles_completed: 0,
                total_volume_contributed: 0,
                last_activity_time: 0,
                consecutive_cycles: 0,
                longest_streak: 0,
            });

        // 2. Update contribution tracking
        if is_timely {
            activity.timely_contributions += 1;
        } else {
            activity.late_contributions += 1;
        }

        // 3. Update volume tracking
        activity.total_volume_contributed += contribution_amount;

        // 4. Update cycles completed
        activity.total_cycles_completed += 1;

        // 5. Update consecutive cycles and streak
        let current_time = env.ledger().timestamp();
        let one_day = 24 * 3600;
        
        if activity.last_activity_time > 0 && (current_time - activity.last_activity_time) <= (one_day * 7) {
            // If last activity was within 7 days, continue streak
            activity.consecutive_cycles += 1;
            if activity.consecutive_cycles > activity.longest_streak {
                activity.longest_streak = activity.consecutive_cycles;
            }
        } else {
            // Reset consecutive cycles
            activity.consecutive_cycles = 1;
        }

        // 6. Update last activity time
        activity.last_activity_time = current_time;

        // 7. Store updated activity
        env.storage().instance().set(&activity_key, &activity);

        // 8. Trigger reliability index recalculation
        Self::calculate_reliability_index(env, user);
    }

    fn apply_ri_decay(env: Env, user: Address) {
        // 1. Get current reliability index
        let ri_key = DataKey::ReliabilityIndex(user.clone());
        let mut ri: ReliabilityIndex = env.storage().instance().get(&ri_key)
            .unwrap_or_else(|| ReliabilityIndex {
                user: user.clone(),
                score: 0,
                last_updated: 0,
                decay_rate: 5,
            });

        // 2. Calculate days since last update
        let current_time = env.ledger().timestamp();
        let days_since_update = if ri.last_updated > 0 {
            (current_time - ri.last_updated) / (24 * 3600)
        } else {
            0
        };

        // 3. Apply decay if inactive for more than 30 days
        if days_since_update > 30 {
            let inactive_days = days_since_update - 30;
            let decay_amount = (inactive_days * ri.decay_rate as u64) as u32;
            
            if ri.score > decay_amount {
                ri.score -= decay_amount;
            } else {
                ri.score = 0;
            }
        }

        // 4. Update last updated time
        ri.last_updated = current_time;

        // 5. Store updated reliability index
        env.storage().instance().set(&ri_key, &ri);
    }

    fn get_reliability_index(env: Env, user: Address) -> ReliabilityIndex {
        let ri_key = DataKey::ReliabilityIndex(user.clone());
        env.storage().instance().get(&ri_key)
            .unwrap_or_else(|| ReliabilityIndex {
                user,
                score: 0,
                last_updated: 0,
                decay_rate: 5,
            })
    }

    // Credit-Limit Scaling based on Historical RI (#269)
    fn initialize_credit_tiers(env: Env, admin: Address) {
        // 1. Authorization: The admin must sign this transaction
        admin.require_auth();

        // 2. Verify the caller is the admin
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("No admin set"));
        
        if stored_admin != admin {
            panic!("Caller is not the admin");
        }

        // 3. Initialize credit tier configuration
        let credit_tiers = CreditTierConfig {
            tier_1_max: 100_0000000,    // 100 XLM for RI 0-200
            tier_2_max: 500_0000000,    // 500 XLM for RI 201-400
            tier_3_max: 2000_0000000,   // 2,000 XLM for RI 401-600
            tier_4_max: 5000_0000000,   // 5,000 XLM for RI 601-800
            tier_5_max: 10000_0000000,  // 10,000 XLM for RI 801-1000
        };

        // 4. Store the credit tier configuration
        env.storage().instance().set(&DataKey::CreditTierConfig, &credit_tiers);
    }

    fn get_user_credit_limit(env: Env, user: Address) -> i128 {
        // 1. Get user's reliability index
        let ri = Self::get_reliability_index(env.clone(), user.clone());
        
        // 2. Get credit tier configuration
        let credit_tiers: CreditTierConfig = env.storage().instance().get(&DataKey::CreditTierConfig)
            .unwrap_or_else(|| panic!("Credit tiers not initialized"));

        // 3. Determine credit limit based on RI score
        let credit_limit = match ri.score {
            0..=200 => credit_tiers.tier_1_max,
            201..=400 => credit_tiers.tier_2_max,
            401..=600 => credit_tiers.tier_3_max,
            601..=800 => credit_tiers.tier_4_max,
            801..=1000 => credit_tiers.tier_5_max,
            _ => credit_tiers.tier_1_max, // Default to lowest tier for invalid scores
        };

        credit_limit
    }

    fn update_user_credit_limit(env: Env, user: Address) {
        // 1. Get current RI and calculate new credit limit
        let ri = Self::get_reliability_index(env.clone(), user.clone());
        let new_credit_limit = Self::get_user_credit_limit(env.clone(), user.clone());
        
        // 2. Create user credit limit record
        let current_time = env.ledger().timestamp();
        let user_credit_limit = UserCreditLimit {
            user: user.clone(),
            max_contribution: new_credit_limit,
            last_updated: current_time,
            current_ri: ri.score,
        };

        // 3. Store the updated credit limit
        env.storage().instance().set(&DataKey::UserCreditLimit(user), &user_credit_limit);
    }

    fn can_join_pool(env: Env, user: Address, pool_max_contribution: i128) -> bool {
        // 1. Get user's current credit limit
        let user_credit_limit = Self::get_user_credit_limit(env.clone(), user.clone());
        
        // 2. Check if pool's contribution amount exceeds user's credit limit
        if pool_max_contribution > user_credit_limit {
            return false;
        }

        // 3. Update user's credit limit (in case RI has changed)
        Self::update_user_credit_limit(env.clone(), user.clone());

        // 4. Double-check with updated credit limit
        let updated_credit_limit = Self::get_user_credit_limit(env, user);
        pool_max_contribution <= updated_credit_limit
    }

    // Emergency-Bailout Fund Logic (Susu Insurance) (#267)
    fn configure_circle_reserve(env: Env, admin: Address, circle_id: u64, reserve_percentage: u32) {
        // 1. Authorization: The admin must sign this transaction
        admin.require_auth();

        // 2. Verify the caller is the admin
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("No admin set"));
        
        if stored_admin != admin {
            panic!("Caller is not the admin");
        }

        // 3. Validate reserve percentage (1-3% = 100-300 basis points)
        if reserve_percentage < 100 || reserve_percentage > 300 {
            panic!("Reserve percentage must be between 1% and 3% (100-300 basis points)");
        }

        // 4. Verify the circle exists
        let _circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));

        // 5. Get existing configuration or create new
        let existing_config: Option<CircleReserveConfig> = env.storage().instance().get(&DataKey::CircleReserve(circle_id));
        
        let reserve_config = CircleReserveConfig {
            circle_id,
            reserve_percentage,
            total_reserve: existing_config.map(|c| c.total_reserve).unwrap_or(0),
            is_active: true,
        };

        // 6. Store the updated configuration
        env.storage().instance().set(&DataKey::CircleReserve(circle_id), &reserve_config);

        // 7. Emit configuration event
        env.events().publish((Symbol::new(&env, "reserve_configured"),), (circle_id, reserve_percentage));
    }

    fn create_insurance_proposal(env: Env, proposer: Address, circle_id: u64, defaulting_member: Address, victim_recipient: Address, claim_amount: i128, reason: String) -> u64 {
        // 1. Authorization: The proposer must sign this transaction
        proposer.require_auth();

        // 2. Verify the circle exists and proposer is a member
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));
        
        let member_key = DataKey::Member(proposer.clone());
        let _member: Member = env.storage().instance().get(&member_key)
            .unwrap_or_else(|| panic!("User is not a member of this circle"));

        // 3. Verify circle reserve is active and has sufficient funds
        let reserve_config: CircleReserveConfig = env.storage().instance().get(&DataKey::CircleReserve(circle_id))
            .unwrap_or_else(|| panic!("Circle reserve not configured"));
        
        if !reserve_config.is_active {
            panic!("Circle reserve is not active");
        }
        
        if reserve_config.total_reserve < claim_amount {
            panic!("Insufficient funds in circle reserve");
        }

        // 4. Get proposal ID (increment counter)
        let mut proposal_count: u64 = env.storage().instance().get(&DataKey::CircleCount).unwrap_or(0);
        proposal_count += 1;

        // 5. Create the insurance proposal
        let current_time = env.ledger().timestamp();
        let voting_deadline = current_time + (3 * 24 * 3600); // 3 days voting period

        let proposal = InsuranceProposal {
            proposal_id: proposal_count,
            circle_id,
            defaulting_member: defaulting_member.clone(),
            victim_recipient: victim_recipient.clone(),
            claim_amount,
            proposer: proposer.clone(),
            created_at: current_time,
            voting_deadline,
            yes_votes: 0,
            no_votes: 0,
            total_members: circle.member_count,
            is_executed: false,
            reason,
        };

        // 6. Store the proposal
        env.storage().instance().set(&DataKey::InsuranceProposal(proposal_count), &proposal);

        // 7. Emit proposal creation event
        env.events().publish((Symbol::new(&env, "insurance_proposal_created"),), (proposal_count, circle_id, defaulting_member, victim_recipient, claim_amount));

        proposal_count
    }

    fn vote_on_insurance(env: Env, voter: Address, proposal_id: u64, vote: bool) {
        // 1. Authorization: The voter must sign this transaction
        voter.require_auth();

        // 2. Get the insurance proposal
        let mut proposal: InsuranceProposal = env.storage().instance().get(&DataKey::InsuranceProposal(proposal_id))
            .unwrap_or_else(|| panic!("Insurance proposal does not exist"));

        // 3. Check if voting is still open
        let current_time = env.ledger().timestamp();
        if current_time > proposal.voting_deadline {
            panic!("Voting period has ended");
        }

        // 4. Check if user is a member of the circle
        let member_key = DataKey::Member(voter.clone());
        let _member: Member = env.storage().instance().get(&member_key)
            .unwrap_or_else(|| panic!("User is not a member of this circle"));

        // 5. Check if user has already voted
        let vote_key = DataKey::Vote(proposal_id, voter.clone());
        if env.storage().instance().has(&vote_key) {
            panic!("User has already voted on this proposal");
        }

        // 6. Record the vote
        let vote_record = VoteCast {
            proposal_id,
            voter: voter.clone(),
            vote,
            voting_power: 1, // Each member gets equal voting power
        };
        env.storage().instance().set(&vote_key, &vote_record);

        // 7. Update proposal vote counts
        if vote {
            proposal.yes_votes += 1;
        } else {
            proposal.no_votes += 1;
        }

        // 8. Save updated proposal
        env.storage().instance().set(&DataKey::InsuranceProposal(proposal_id), &proposal);

        // 9. Emit vote event
        env.events().publish((Symbol::new(&env, "insurance_vote_cast"),), (proposal_id, voter, vote));
    }

    fn execute_insurance_claim(env: Env, executor: Address, proposal_id: u64) {
        // 1. Authorization: The executor must sign this transaction
        executor.require_auth();

        // 2. Get the insurance proposal
        let mut proposal: InsuranceProposal = env.storage().instance().get(&DataKey::InsuranceProposal(proposal_id))
            .unwrap_or_else(|| panic!("Insurance proposal does not exist"));

        // 3. Check if proposal has already been executed
        if proposal.is_executed {
            panic!("Proposal has already been executed");
        }

        // 4. Check if voting period has ended
        let current_time = env.ledger().timestamp();
        if current_time <= proposal.voting_deadline {
            panic!("Voting period has not ended yet");
        }

        // 5. Check if proposal passed (requires 75% majority)
        let required_votes = (proposal.total_members * 75) / 100; // 75% threshold
        if proposal.yes_votes < required_votes {
            panic!("Insufficient votes for insurance claim (75% required)");
        }

        // 6. Get circle and reserve configuration
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(proposal.circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));
        
        let mut reserve_config: CircleReserveConfig = env.storage().instance().get(&DataKey::CircleReserve(proposal.circle_id))
            .unwrap_or_else(|| panic!("Circle reserve not configured"));

        // 7. Check if sufficient reserve funds are still available
        if reserve_config.total_reserve < proposal.claim_amount {
            panic!("Insufficient funds in circle reserve for claim");
        }

        // 8. Transfer insurance claim to victim recipient
        let client = token::Client::new(&env, &circle.token);
        client.transfer(&env.current_contract_address(), &proposal.victim_recipient, &proposal.claim_amount);

        // 9. Update reserve balance
        reserve_config.total_reserve -= proposal.claim_amount;
        env.storage().instance().set(&DataKey::CircleReserve(proposal.circle_id), &reserve_config);

        // 10. Mark proposal as executed
        proposal.is_executed = true;
        env.storage().instance().set(&DataKey::InsuranceProposal(proposal_id), &proposal);

        // 11. Emit execution event
        env.events().publish((Symbol::new(&env, "insurance_claim_executed"),), (proposal_id, proposal.circle_id, proposal.victim_recipient, proposal.claim_amount));
    }

    fn get_circle_reserve_balance(env: Env, circle_id: u64) -> i128 {
        let reserve_config: CircleReserveConfig = env.storage().instance().get(&DataKey::CircleReserve(circle_id))
            .unwrap_or_else(|| CircleReserveConfig {
                circle_id,
                reserve_percentage: 200,
                total_reserve: 0,
                is_active: false,
            });
        
        reserve_config.total_reserve
    }
}

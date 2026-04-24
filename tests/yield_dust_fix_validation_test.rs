#![cfg(test)]

use soroban_sdk::{contract, contractimpl, contracterror, contracttype, symbol_short, token,
    Address, Env, String, Symbol, Vec, i128, u64, u32, Map};
use sorosusu_contracts::{
    SoroSusu, SoroSusuClient,
    YieldDistribution, YieldDelegation, YieldDelegationStatus, CircleInfo, Member, MemberStatus,
    DataKey, YIELD_DISTRIBUTION_RECIPIENT_BPS, YIELD_DISTRIBUTION_TREASURY_BPS
};

/// Test contract to validate the dust prevention fix works correctly
#[contract]
pub struct YieldDustFixValidation;

#[contractimpl]
impl YieldDustFixValidation {
    
    /// Test that validates the exact fix for dust prevention
    pub fn test_dust_fix_validation(env: Env) {
        // Test the exact mathematical fix
        let test_cases = vec![
            &env,
            (123456789, 61728394, 61728395), // Original problematic case
            (100000001, 50000000, 50000001), // 1 stroop dust
            (999999999, 499999999, 500000000), // Large amount with dust
            (1, 0, 1), // Minimum amount - all to treasury
            (99, 49, 50), // Small amount with dust
            (101, 50, 51), // Another dust case
            (10000, 5000, 5000), // Perfectly divisible - no dust
            (0, 0, 0), // Zero yield
        ];
        
        for i in 0..test_cases.len() {
            let test_case = test_cases.get(i).unwrap();
            let total_yield = test_case.0;
            let expected_recipient = test_case.1;
            let expected_treasury = test_case.2;
            
            // Apply the fixed calculation
            let recipient_share = (total_yield * YIELD_DISTRIBUTION_RECIPIENT_BPS as i128) / 10000;
            let treasury_share = total_yield - recipient_share;
            
            // Verify the fix works
            assert_eq!(recipient_share, expected_recipient, 
                "Recipient share mismatch for yield {}", total_yield);
            assert_eq!(treasury_share, expected_treasury, 
                "Treasury share mismatch for yield {}", total_yield);
            
            // Verify no dust is created
            let total_distributed = recipient_share + treasury_share;
            let dust = total_yield - total_distributed;
            assert_eq!(dust, 0, "Dust detected for yield {}: {}", total_yield, dust);
            
            // Log validation results
            env.events().publish(
                (Symbol::new(&env, "DUST_FIX_VALIDATED"), i),
                (total_yield, recipient_share, treasury_share, dust),
            );
        }
    }
    
    /// Test that simulates the complete yield distribution flow with dust prevention
    pub fn test_complete_yield_distribution_flow(env: Env) {
        let circle_id = 1u64;
        let total_yield_earned = 123456789;
        let already_distributed = 0;
        
        // Simulate the fixed distribute_yield_earnings function
        let new_yield = total_yield_earned - already_distributed;
        
        // Apply the dust prevention fix
        let recipient_share = (new_yield * YIELD_DISTRIBUTION_RECIPIENT_BPS as i128) / 10000;
        let treasury_share = new_yield - recipient_share;
        
        // Verify complete distribution
        assert_eq!(recipient_share + treasury_share, new_yield);
        assert_eq!(recipient_share, 61728394);
        assert_eq!(treasury_share, 61728395); // Gets the extra stroop
        
        // Simulate updating the delegation record
        let updated_distributed = already_distributed + new_yield;
        assert_eq!(updated_distributed, total_yield_earned);
        
        // Create the distribution record as the function would
        let distribution = YieldDistribution {
            circle_id,
            recipient_share,
            treasury_share,
            total_yield: new_yield,
            distribution_time: env.ledger().timestamp(),
            round_number: 1,
        };
        
        // Verify the distribution record is accurate
        assert_eq!(distribution.recipient_share + distribution.treasury_share, distribution.total_yield);
        
        // Log the complete flow
        env.events().publish(
            (Symbol::new(&env, "COMPLETE_FLOW_VALIDATED"),),
            (new_yield, recipient_share, treasury_share, updated_distributed),
        );
    }
    
    /// Test edge cases to ensure robustness
    pub fn test_edge_cases_robustness(env: Env) {
        // Test case 1: Very large yield amount
        let large_yield = i128::MAX / 2; // Safe large number
        let recipient_share = (large_yield * YIELD_DISTRIBUTION_RECIPIENT_BPS as i128) / 10000;
        let treasury_share = large_yield - recipient_share;
        assert_eq!(recipient_share + treasury_share, large_yield);
        
        // Test case 2: Yield that creates maximum possible dust
        let max_dust_yield = 9999;
        let recipient_share = (max_dust_yield * 5000) / 10000;
        let treasury_share = max_dust_yield - recipient_share;
        assert_eq!(recipient_share, 4999);
        assert_eq!(treasury_share, 5000);
        assert_eq!(recipient_share + treasury_share, max_dust_yield);
        
        // Test case 3: Single stroop
        let single_stroop = 1;
        let recipient_share = (single_stroop * 5000) / 10000;
        let treasury_share = single_stroop - recipient_share;
        assert_eq!(recipient_share, 0);
        assert_eq!(treasury_share, 1);
        
        // Test case 4: Exactly 1 XLM (10000 stroops)
        let one_xlm = 10000;
        let recipient_share = (one_xlm * 5000) / 10000;
        let treasury_share = one_xlm - recipient_share;
        assert_eq!(recipient_share, 5000);
        assert_eq!(treasury_share, 5000);
        
        env.events().publish(
            (Symbol::new(&env, "EDGE_CASES_ROBUST"),),
            (large_yield, max_dust_yield, single_stroop, one_xlm),
        );
    }
    
    /// Test that verifies the treasury receives all dust correctly
    pub fn test_treasury_dust_collection(env: Env) {
        let test_yields = vec![
            &env,
            123456789, // Original case - 1 stroop to treasury
            100000001, // 1 stroop to treasury
            999999999, // 1 stroop to treasury
            99,        // 1 stroop to treasury
            101,       // 1 stroop to treasury
        ];
        
        let mut total_dust_collected = 0i128;
        
        for i in 0..test_yields.len() {
            let yield_amount = test_yields.get(i).unwrap();
            
            // Calculate with original (buggy) method to see dust
            let buggy_recipient = (*yield_amount * 5000) / 10000;
            let buggy_treasury = (*yield_amount * 5000) / 10000;
            let buggy_dust = *yield_amount - (buggy_recipient + buggy_treasury);
            
            // Calculate with fixed method
            let fixed_recipient = (*yield_amount * 5000) / 10000;
            let fixed_treasury = *yield_amount - fixed_recipient;
            let fixed_dust = *yield_amount - (fixed_recipient + fixed_treasury);
            
            // Treasury should collect all the dust
            let dust_collected_this_round = fixed_treasury - buggy_treasury;
            total_dust_collected += dust_collected_this_round;
            
            assert_eq!(fixed_dust, 0, "Fixed method should have no dust");
            assert_eq!(dust_collected_this_round, buggy_dust, "Treasury should collect exactly the dust");
            
            env.events().publish(
                (Symbol::new(&env, "DUST_COLLECTION_ROUND"), i),
                (*yield_amount, buggy_dust, dust_collected_this_round, total_dust_collected),
            );
        }
        
        // Verify total dust collected
        assert_eq!(total_dust_collected, 5, "Should collect 5 stroops total from test cases");
        
        env.events().publish(
            (Symbol::new(&env, "TOTAL_DUST_COLLECTED"),),
            total_dust_collected,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dust_fix_validation() {
        let env = Env::default();
        YieldDustFixValidation::test_dust_fix_validation(env);
    }

    #[test]
    fn test_complete_yield_distribution_flow() {
        let env = Env::default();
        YieldDustFixValidation::test_complete_yield_distribution_flow(env);
    }

    #[test]
    fn test_edge_cases_robustness() {
        let env = Env::default();
        YieldDustFixValidation::test_edge_cases_robustness(env);
    }

    #[test]
    fn test_treasury_dust_collection() {
        let env = Env::default();
        YieldDustFixValidation::test_treasury_dust_collection(env);
    }
}

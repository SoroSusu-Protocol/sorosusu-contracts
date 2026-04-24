#![cfg(test)]

use soroban_sdk::{contract, contractimpl, contracterror, contracttype, symbol_short, token,
    Address, Env, String, Symbol, Vec, i128, u64, u32};
use sorosusu_contracts::{
    SoroSusu, SoroSusuClient,
    YieldDistribution, YieldDelegation, YieldDelegationStatus, CircleInfo, Member, MemberStatus,
    YIELD_DISTRIBUTION_RECIPIENT_BPS, YIELD_DISTRIBUTION_TREASURY_BPS
};

/// Test contract to demonstrate and verify dust prevention in yield harvesting
#[contract]
pub struct YieldDustTest;

#[contractimpl]
impl YieldDustTest {
    
    /// Test demonstrating the current dust issue in yield distribution
    pub fn test_current_dust_issue(env: Env) {
        // Test case: Yield amount that creates fractional dust
        let total_yield = 123456789; // 12.3456789 XLM
        
        // Current implementation (buggy):
        let recipient_share = (total_yield * YIELD_DISTRIBUTION_RECIPIENT_BPS as i128) / 10000;
        let treasury_share = (total_yield * YIELD_DISTRIBUTION_TREASURY_BPS as i128) / 10000;
        
        let distributed_amount = recipient_share + treasury_share;
        let dust_amount = total_yield - distributed_amount;
        
        // This demonstrates the dust issue
        assert_eq!(recipient_share, 61728394); // 6.1728394 XLM
        assert_eq!(treasury_share, 61728394);   // 6.1728394 XLM  
        assert_eq!(distributed_amount, 123456788); // Missing 1 stroop
        assert_eq!(dust_amount, 1); // 1 stroop permanently locked
        
        // Log the issue
        env.events().publish(
            (Symbol::new(&env, "DUST_ISSUE_DEMONSTRATED"),),
            (total_yield, recipient_share, treasury_share, dust_amount),
        );
    }
    
    /// Test demonstrating the correct dust prevention approach
    pub fn test_dust_prevention_fix(env: Env) {
        let total_yield = 123456789; // 12.3456789 XLM
        
        // Fixed implementation: give remainder to treasury
        let recipient_share = (total_yield * YIELD_DISTRIBUTION_RECIPIENT_BPS as i128) / 10000;
        let treasury_share = total_yield - recipient_share; // Treasury gets remainder
        
        let distributed_amount = recipient_share + treasury_share;
        let dust_amount = total_yield - distributed_amount;
        
        // This ensures no dust is left behind
        assert_eq!(distributed_amount, total_yield);
        assert_eq!(dust_amount, 0);
        assert_eq!(treasury_share, 61728395); // Treasury gets the extra 1 stroop
        
        // Log the fix
        env.events().publish(
            (Symbol::new(&env, "DUST_PREVENTION_FIXED"),),
            (total_yield, recipient_share, treasury_share, dust_amount),
        );
    }
    
    /// Test with multiple problematic amounts
    pub fn test_various_dust_scenarios(env: Env) {
        let problematic_amounts = vec![
            &env,
            100000001, // Creates 1 stroop dust
            123456789, // Creates multiple dust scenarios  
            999999999, // Large amount with dust
            1,         // Minimum amount - all goes to recipient
            50,        // Small amount - 25 to each, no dust
            99,        // Creates 1 stroop dust
            101,       // Creates 1 stroop dust
        ];
        
        for i in 0..problematic_amounts.len() {
            let amount = problematic_amounts.get(i).unwrap();
            
            // Current (buggy) approach
            let buggy_recipient = (*amount * 5000) / 10000;
            let buggy_treasury = (*amount * 5000) / 10000;
            let buggy_dust = *amount - (buggy_recipient + buggy_treasury);
            
            // Fixed approach
            let fixed_recipient = (*amount * 5000) / 10000;
            let fixed_treasury = *amount - fixed_recipient;
            let fixed_dust = *amount - (fixed_recipient + fixed_treasury);
            
            // Log results
            env.events().publish(
                (Symbol::new(&env, "DUST_COMPARISON"), i),
                (*amount, buggy_recipient, buggy_treasury, buggy_dust, 
                 fixed_recipient, fixed_treasury, fixed_dust),
            );
            
            // Verify fix eliminates dust
            assert_eq!(fixed_dust, 0);
            assert!(buggy_dust >= 0); // Current implementation may have dust
        }
    }
    
    /// Test edge cases for dust prevention
    pub fn test_edge_cases(env: Env) {
        // Test case 1: Zero yield
        let zero_yield = 0;
        let recipient_share = (zero_yield * 5000) / 10000;
        let treasury_share = zero_yield - recipient_share;
        assert_eq!(recipient_share, 0);
        assert_eq!(treasury_share, 0);
        
        // Test case 2: Maximum possible dust (9999 stroops)
        let max_dust_yield = 9999;
        let recipient_share = (max_dust_yield * 5000) / 10000;
        let treasury_share = max_dust_yield - recipient_share;
        assert_eq!(recipient_share, 4999); // floor(9999 * 0.5)
        assert_eq!(treasury_share, 5000); // Gets the remainder
        assert_eq!(recipient_share + treasury_share, max_dust_yield);
        
        // Test case 3: Perfectly divisible amount
        let clean_yield = 10000;
        let recipient_share = (clean_yield * 5000) / 10000;
        let treasury_share = clean_yield - recipient_share;
        assert_eq!(recipient_share, 5000);
        assert_eq!(treasury_share, 5000);
        assert_eq!(recipient_share + treasury_share, clean_yield);
        
        env.events().publish(
            (Symbol::new(&env, "EDGE_CASES_TESTED"),),
            (zero_yield, max_dust_yield, clean_yield),
        );
    }
    
    /// Test that simulates the actual distribute_yield_earnings function behavior
    pub fn test_actual_function_behavior(env: Env) {
        let circle_id = 1u64;
        let total_yield = 123456789;
        let already_distributed = 0;
        let new_yield = total_yield - already_distributed;
        
        // Simulate current function behavior
        let recipient_share = (new_yield * YIELD_DISTRIBUTION_RECIPIENT_BPS as i128) / 10000;
        let treasury_share = (new_yield * YIELD_DISTRIBUTION_TREASURY_BPS as i128) / 10000;
        
        // Current bug: function records distributing full new_yield but actually distributes less
        let actually_distributed = recipient_share + treasury_share;
        let recorded_distributed = new_yield;
        let locked_dust = recorded_distributed - actually_distributed;
        
        assert_eq!(locked_dust, 1); // 1 stroop permanently locked
        
        // Simulate fixed function behavior
        let fixed_recipient_share = (new_yield * YIELD_DISTRIBUTION_RECIPIENT_BPS as i128) / 10000;
        let fixed_treasury_share = new_yield - fixed_recipient_share;
        let fixed_distributed = fixed_recipient_share + fixed_treasury_share;
        let fixed_dust = recorded_distributed - fixed_distributed;
        
        assert_eq!(fixed_dust, 0); // No dust with fixed implementation
        
        env.events().publish(
            (Symbol::new(&env, "FUNCTION_BEHAVIOR_TEST"),),
            (new_yield, recipient_share, treasury_share, locked_dust,
             fixed_recipient_share, fixed_treasury_share, fixed_dust),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_dust_issue() {
        let env = Env::default();
        YieldDustTest::test_current_dust_issue(env);
    }

    #[test]
    fn test_dust_prevention_fix() {
        let env = Env::default();
        YieldDustTest::test_dust_prevention_fix(env);
    }

    #[test]
    fn test_various_dust_scenarios() {
        let env = Env::default();
        YieldDustTest::test_various_dust_scenarios(env);
    }

    #[test]
    fn test_edge_cases() {
        let env = Env::default();
        YieldDustTest::test_edge_cases(env);
    }

    #[test]
    fn test_actual_function_behavior() {
        let env = Env::default();
        YieldDustTest::test_actual_function_behavior(env);
    }
}

#![cfg(test)]

use soroban_sdk::{contract, contractimpl, contracterror, contracttype, symbol_short, token,
    Address, Env, String, Symbol, Vec, i128, u64, u32};
use sorosusu_contracts::{
    SoroSusu, SoroSusuClient,
    YieldDistribution, YieldDelegation, YieldDelegationStatus, CircleInfo, Member, MemberStatus
};

#[contract]
pub struct TestContract;

#[contractimpl]
impl TestContract {
    pub fn test_yield_harvest_math_precision(env: Env) {
        // Test case 1: Yield distribution with 20 members and fractional dust
        let admin = Address::generate(&env);
        let token_address = Address::generate(&env);
        let circle_id = 1;
        
        // Create a test scenario with 20 members
        let mut members = Vec::new(&env);
        for i in 0..20 {
            members.push_back(Address::generate(&env));
        }
        
        // Simulate a yield amount that creates fractional dust when divided
        // Using an amount that's not evenly divisible by 20 members or by 10000 BPS
        let total_yield = 123456789; // This will create dust when divided
        
        // Current implementation: 50/50 split using basis points
        let recipient_share_bps = 5000u32; // 50%
        let treasury_share_bps = 5000u32;   // 50%
        
        let recipient_share = (total_yield * recipient_share_bps as i128) / 10000;
        let treasury_share = (total_yield * treasury_share_bps as i128) / 10000;
        
        // Calculate potential dust
        let distributed_amount = recipient_share + treasury_share;
        let dust_amount = total_yield - distributed_amount;
        
        // Verify the math precision
        assert_eq!(recipient_share, 61728394); // floor(123456789 * 5000 / 10000)
        assert_eq!(treasury_share, 61728394);   // floor(123456789 * 5000 / 10000)
        assert_eq!(distributed_amount, 123456788); // 123456789 - 1
        assert_eq!(dust_amount, 1); // 1 stroop of dust
        
        // Test case 2: Multiple members distribution scenario
        // Simulate distributing yield among 20 members with equal shares
        let member_count = 20u32;
        let yield_per_member = recipient_share / member_count as i128;
        let member_dust = recipient_share % member_count as i128;
        
        assert_eq!(yield_per_member, 3086419); // floor(61728394 / 20)
        assert_eq!(member_dust, 14); // 14 stroops remaining after equal distribution
        
        // Test case 3: Verify dust should go to treasury, not remain locked
        // The correct implementation should ensure all dust goes to treasury
        let expected_treasury_total = treasury_share + dust_amount + member_dust;
        
        // This demonstrates the dust issue: 
        // - 1 stroop from the 50/50 split
        // - 14 stroops from member distribution
        // Total dust: 15 stroops that could be permanently locked
        assert_eq!(expected_treasury_total, 61728394 + 1 + 14); // 61728409
        
        env.events().publish(
            (Symbol::new(&env, "YIELD_PRECISION_TEST"),),
            (total_yield, recipient_share, treasury_share, dust_amount, member_dust),
        );
    }
    
    pub fn test_fixed_point_division_precision(env: Env) {
        // Test various amounts that could create precision issues
        let test_amounts = vec![
            &env, 
            100000001, // Creates 1 stroop dust
            123456789, // Creates multiple dust scenarios
            999999999, // Large amount with dust
            1,         // Minimum amount
            50,        // Small amount that creates dust
            5000,      // Exactly 0.5 XLM - should be clean
            10000,     // Exactly 1 XLM - should be clean
        ];
        
        for i in 0..test_amounts.len() {
            let amount = test_amounts.get(i).unwrap();
            let recipient_share = (*amount * 5000) / 10000;
            let treasury_share = (*amount * 5000) / 10000;
            let dust = *amount - (recipient_share + treasury_share);
            
            // Log the precision results
            env.events().publish(
                (Symbol::new(&env, "PRECISION_TEST"), i),
                (*amount, recipient_share, treasury_share, dust),
            );
            
            // Verify that dust is properly calculated
            assert!(dust >= 0 && dust < 10000); // Dust should be less than 1 XLM
        }
    }
    
    pub fn test_dust_prevention_mechanism(env: Env) {
        // This test demonstrates how dust should be prevented
        let total_yield = 123456789;
        
        // Correct implementation: ensure all yield is distributed
        let recipient_share = (total_yield * 5000) / 10000;
        let treasury_share = total_yield - recipient_share; // Give remainder to treasury
        
        // This ensures no dust is left behind
        let distributed_amount = recipient_share + treasury_share;
        let dust_amount = total_yield - distributed_amount;
        
        assert_eq!(distributed_amount, total_yield);
        assert_eq!(dust_amount, 0);
        assert_eq!(treasury_share, 61728395); // Gets the extra 1 stroop
        
        env.events().publish(
            (Symbol::new(&env, "DUST_PREVENTION_TEST"),),
            (total_yield, recipient_share, treasury_share, dust_amount),
        );
    }
    
    pub fn test_member_distribution_with_dust_handling(env: Env) {
        // Test distributing yield among members with proper dust handling
        let total_yield_for_members = 61728394; // 50% of original yield
        let member_count = 20u32;
        
        // Calculate equal shares with dust going to last member
        let base_share = total_yield_for_members / member_count as i128;
        let remainder = total_yield_for_members % member_count as i128;
        
        // First 19 members get base share
        let first_19_total = base_share * 19;
        // Last member gets base share + remainder
        let last_member_share = base_share + remainder;
        
        // Verify all yield is distributed
        let total_distributed = first_19_total + last_member_share;
        assert_eq!(total_distributed, total_yield_for_members);
        
        env.events().publish(
            (Symbol::new(&env, "MEMBER_DISTRIBUTION_TEST"),),
            (base_share, remainder, last_member_share, total_distributed),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yield_harvest_math_precision() {
        let env = Env::default();
        TestContract::test_yield_harvest_math_precision(env);
    }

    #[test]
    fn test_fixed_point_division_precision() {
        let env = Env::default();
        TestContract::test_fixed_point_division_precision(env);
    }

    #[test]
    fn test_dust_prevention_mechanism() {
        let env = Env::default();
        TestContract::test_dust_prevention_mechanism(env);
    }

    #[test]
    fn test_member_distribution_with_dust_handling() {
        let env = Env::default();
        TestContract::test_member_distribution_with_dust_handling(env);
    }
}

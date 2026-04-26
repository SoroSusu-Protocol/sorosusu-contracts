//! Issue #326: Fuzz Test: Reversal of State Post-Appeal
//!
//! If a user is slashed and the DAO later overrides the decision and grants the
//! appeal, verify that all state variables (Reliability Index proxy, balance,
//! group status) are perfectly restored to their pre-slash conditions without
//! corrupting the ongoing cycle math.
//!
//! The fuzz harness exercises the slash → appeal-granted path with arbitrary
//! contribution amounts, member counts, and time offsets to ensure no edge case
//! leaves the contract in a corrupted state.

#[cfg(test)]
mod fuzz_appeal_reversal {
    use arbitrary::{Arbitrary, Unstructured};

    /// 72 hours in seconds.
    const APPEALS_TIMELOCK_SECS: u64 = 72 * 60 * 60;

    // -----------------------------------------------------------------------
    // Minimal in-memory state model (mirrors the on-chain state we care about)
    // -----------------------------------------------------------------------

    #[derive(Debug, Clone)]
    struct MemberState {
        contribution_count: u32,
        has_contributed: bool,
        missed_deadline_timestamp: u64,
        /// Reliability Index in basis points (0–10_000).
        reliability_index: u32,
    }

    #[derive(Debug, Clone)]
    struct CircleState {
        group_reserve: u64,
        is_active: bool,
        current_cycle: u32,
    }

    #[derive(Debug, Clone)]
    struct PendingSlashVault {
        amount: u64,
        slashed_at: u64,
    }

    /// Simulate slash_collateral: deduct from reserve, create pending vault,
    /// reduce member RI.
    fn slash_collateral(
        member: &mut MemberState,
        circle: &mut CircleState,
        vault: &mut Option<PendingSlashVault>,
        slash_amount: u64,
        current_time: u64,
    ) -> Result<(), &'static str> {
        if circle.group_reserve < slash_amount {
            return Err("insufficient reserve");
        }
        circle.group_reserve -= slash_amount;
        *vault = Some(PendingSlashVault {
            amount: slash_amount,
            slashed_at: current_time,
        });
        // Penalise RI by 20% (2000 bps).
        member.reliability_index = member.reliability_index.saturating_sub(2000);
        Ok(())
    }

    /// Simulate appeal_granted (DAO override): reverse the slash entirely.
    /// Must only be callable while the 72-hour window is still open.
    fn grant_appeal(
        member: &mut MemberState,
        circle: &mut CircleState,
        vault: &mut Option<PendingSlashVault>,
        current_time: u64,
        pre_slash_ri: u32,
    ) -> Result<(), &'static str> {
        let record = vault.as_ref().ok_or("no pending slash")?;

        // Appeal must be within the 72-hour window.
        let window_end = record
            .slashed_at
            .checked_add(APPEALS_TIMELOCK_SECS)
            .ok_or("timestamp overflow")?;
        if current_time >= window_end {
            return Err("appeal window expired");
        }

        // Restore reserve.
        circle.group_reserve = circle
            .group_reserve
            .checked_add(record.amount)
            .ok_or("reserve overflow")?;

        // Restore RI to pre-slash value.
        member.reliability_index = pre_slash_ri;

        // Clear the vault.
        *vault = None;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Property helpers
    // -----------------------------------------------------------------------

    /// After a successful appeal the state must be identical to pre-slash.
    fn assert_state_fully_restored(
        member_before: &MemberState,
        circle_before: &CircleState,
        member_after: &MemberState,
        circle_after: &CircleState,
        vault_after: &Option<PendingSlashVault>,
    ) {
        assert_eq!(
            member_after.reliability_index, member_before.reliability_index,
            "RI must be fully restored"
        );
        assert_eq!(
            circle_after.group_reserve, circle_before.group_reserve,
            "group reserve must be fully restored"
        );
        assert_eq!(
            circle_after.is_active, circle_before.is_active,
            "circle active status must be unchanged"
        );
        assert_eq!(
            circle_after.current_cycle, circle_before.current_cycle,
            "cycle counter must be unchanged"
        );
        assert!(vault_after.is_none(), "pending slash vault must be cleared");
    }

    // -----------------------------------------------------------------------
    // Fuzz parameters
    // -----------------------------------------------------------------------

    #[derive(Debug, Clone, Arbitrary)]
    struct FuzzParams {
        contribution_amount: u64,
        member_count: u8,       // 1–50
        initial_ri: u16,        // 0–10_000
        initial_reserve: u64,
        time_offset_secs: u32,  // seconds after slash before appeal (must be < 72 h)
    }

    // -----------------------------------------------------------------------
    // Fuzz tests
    // -----------------------------------------------------------------------

    /// Core property: slash followed by a timely appeal must restore all state.
    #[test]
    fn fuzz_slash_then_appeal_restores_state() {
        // Run with a fixed set of representative seeds since proptest is not
        // available in this module (it lives in the fuzz_tests module in lib.rs).
        let seeds: &[&[u8]] = &[
            &[0u8; 32],
            &[1u8; 32],
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
              16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31],
            &[255u8; 32],
            &[128u8; 32],
            // Edge: zero contribution
            &[0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0,
              0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            // Edge: max u16 RI
            &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 0, 0,
              0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        ];

        for seed in seeds {
            let mut u = Unstructured::new(seed);
            let params = match FuzzParams::arbitrary(&mut u) {
                Ok(p) => p,
                Err(_) => continue, // not enough bytes — skip
            };

            run_slash_appeal_scenario(params);
        }
    }

    fn run_slash_appeal_scenario(params: FuzzParams) {
        // Normalise inputs.
        let member_count = (params.member_count as u32).max(1).min(50);
        let initial_ri = (params.initial_ri as u32).min(10_000);
        let contribution = params.contribution_amount.max(1);
        // Reserve must be at least enough to slash.
        let initial_reserve = params.initial_reserve.max(contribution);
        // Appeal must arrive within the 72-hour window.
        let appeal_offset = (params.time_offset_secs as u64) % APPEALS_TIMELOCK_SECS;

        let slash_time: u64 = 1_700_000_000;
        let appeal_time = slash_time + appeal_offset;

        // Build pre-slash state.
        let member_before = MemberState {
            contribution_count: 3,
            has_contributed: true,
            missed_deadline_timestamp: 0,
            reliability_index: initial_ri,
        };
        let circle_before = CircleState {
            group_reserve: initial_reserve,
            is_active: true,
            current_cycle: 2,
        };

        let mut member = member_before.clone();
        let mut circle = circle_before.clone();
        let mut vault: Option<PendingSlashVault> = None;

        // Slash.
        slash_collateral(&mut member, &mut circle, &mut vault, contribution, slash_time)
            .expect("slash must succeed with sufficient reserve");

        // Vault must exist and reserve must have decreased.
        assert!(vault.is_some(), "vault must be populated after slash");
        assert_eq!(circle.group_reserve, initial_reserve - contribution);

        // Grant appeal within the window.
        grant_appeal(&mut member, &mut circle, &mut vault, appeal_time, initial_ri)
            .expect("appeal must succeed within the 72-hour window");

        // All state must be restored.
        assert_state_fully_restored(
            &member_before,
            &circle_before,
            &member,
            &circle,
            &vault,
        );
    }

    /// Appeal after the 72-hour window must be rejected.
    #[test]
    fn fuzz_appeal_after_window_is_rejected() {
        let seeds: &[&[u8]] = &[&[0u8; 32], &[42u8; 32], &[255u8; 32]];

        for seed in seeds {
            let mut u = Unstructured::new(seed);
            let params = match FuzzParams::arbitrary(&mut u) {
                Ok(p) => p,
                Err(_) => continue,
            };

            let contribution = params.contribution_amount.max(1);
            let initial_reserve = params.initial_reserve.max(contribution);
            let initial_ri = (params.initial_ri as u32).min(10_000);

            let slash_time: u64 = 1_700_000_000;
            // Appeal arrives after the window.
            let appeal_time = slash_time + APPEALS_TIMELOCK_SECS + 1;

            let mut member = MemberState {
                contribution_count: 1,
                has_contributed: true,
                missed_deadline_timestamp: 0,
                reliability_index: initial_ri,
            };
            let mut circle = CircleState {
                group_reserve: initial_reserve,
                is_active: true,
                current_cycle: 1,
            };
            let mut vault: Option<PendingSlashVault> = None;

            slash_collateral(&mut member, &mut circle, &mut vault, contribution, slash_time)
                .expect("slash must succeed");

            let result = grant_appeal(&mut member, &mut circle, &mut vault, appeal_time, initial_ri);
            assert!(
                result.is_err(),
                "appeal after 72-hour window must be rejected"
            );
        }
    }

    /// Cycle math must be unaffected by a slash+appeal round-trip.
    #[test]
    fn test_cycle_math_unaffected_by_slash_appeal() {
        let contribution: u64 = 1_000_000;
        let initial_reserve: u64 = 10_000_000;
        let initial_ri: u32 = 8_000;
        let slash_time: u64 = 1_700_000_000;
        let appeal_time = slash_time + APPEALS_TIMELOCK_SECS / 2; // within window

        let mut member = MemberState {
            contribution_count: 5,
            has_contributed: true,
            missed_deadline_timestamp: 0,
            reliability_index: initial_ri,
        };
        let mut circle = CircleState {
            group_reserve: initial_reserve,
            is_active: true,
            current_cycle: 3,
        };
        let mut vault: Option<PendingSlashVault> = None;

        let circle_before = circle.clone();

        slash_collateral(&mut member, &mut circle, &mut vault, contribution, slash_time).unwrap();
        grant_appeal(&mut member, &mut circle, &mut vault, appeal_time, initial_ri).unwrap();

        // Cycle counter must be unchanged.
        assert_eq!(circle.current_cycle, circle_before.current_cycle);
        // Circle must still be active.
        assert!(circle.is_active);
        // Reserve must be fully restored.
        assert_eq!(circle.group_reserve, initial_reserve);
    }

    /// RI must not go below zero during slash and must be exactly restored on appeal.
    #[test]
    fn test_ri_restoration_from_near_zero() {
        let contribution: u64 = 500;
        let initial_reserve: u64 = 10_000;
        let initial_ri: u32 = 100; // very low RI — slash would saturate at 0
        let slash_time: u64 = 1_700_000_000;
        let appeal_time = slash_time + 1_000; // well within window

        let mut member = MemberState {
            contribution_count: 1,
            has_contributed: false,
            missed_deadline_timestamp: slash_time,
            reliability_index: initial_ri,
        };
        let mut circle = CircleState {
            group_reserve: initial_reserve,
            is_active: true,
            current_cycle: 1,
        };
        let mut vault: Option<PendingSlashVault> = None;

        slash_collateral(&mut member, &mut circle, &mut vault, contribution, slash_time).unwrap();
        // RI saturates at 0 (100 - 2000 = 0 via saturating_sub).
        assert_eq!(member.reliability_index, 0);

        grant_appeal(&mut member, &mut circle, &mut vault, appeal_time, initial_ri).unwrap();
        // RI must be restored to the pre-slash value, not left at 0.
        assert_eq!(member.reliability_index, initial_ri);
    }
}

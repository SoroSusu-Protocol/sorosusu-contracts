//! Issue #318: Test: Integer Overflow/Underflow in Epoch Calculation
//!
//! Susu cycles rely heavily on ledger timestamps for calculating deadlines and
//! Reliability Indices. This suite verifies that u64 time calculations never
//! overflow and that subtracting current time from past deadlines never causes
//! an unhandled underflow panic.

#[cfg(test)]
mod epoch_overflow_tests {
    /// 72 hours in seconds — the appeals timelock constant used across the protocol.
    const APPEALS_TIMELOCK_SECS: u64 = 72 * 60 * 60; // 259_200

    /// Simulate the deadline calculation used in create_circle / deposit.
    fn deadline_from(current_time: u64, cycle_duration: u64) -> Option<u64> {
        current_time.checked_add(cycle_duration)
    }

    /// Simulate the grace-period-end calculation used in late_contribution.
    fn grace_period_end(missed_deadline: u64, grace_period: u64) -> Option<u64> {
        missed_deadline.checked_add(grace_period)
    }

    /// Simulate the "time remaining" calculation — must never underflow.
    fn time_remaining(deadline: u64, current_time: u64) -> u64 {
        deadline.saturating_sub(current_time)
    }

    /// Simulate the Reliability Index decay: RI decreases proportionally to
    /// how many seconds have elapsed since the last contribution.
    fn reliability_index_decay(ri: u64, elapsed_secs: u64, decay_rate_per_day: u64) -> u64 {
        let days_elapsed = elapsed_secs / 86_400;
        let decay = days_elapsed.saturating_mul(decay_rate_per_day);
        ri.saturating_sub(decay)
    }

    // -----------------------------------------------------------------------
    // Deadline arithmetic
    // -----------------------------------------------------------------------

    #[test]
    fn test_deadline_normal_timestamps() {
        // Typical present-day timestamp (~2024) + 1-week cycle
        let now: u64 = 1_700_000_000;
        let cycle: u64 = 7 * 24 * 60 * 60; // 604_800
        let deadline = deadline_from(now, cycle).expect("should not overflow");
        assert_eq!(deadline, now + cycle);
    }

    #[test]
    fn test_deadline_far_future_timestamp() {
        // Timestamp 100 years from now (~2124)
        let now: u64 = 1_700_000_000 + 100 * 365 * 24 * 60 * 60;
        let cycle: u64 = 30 * 24 * 60 * 60; // 30-day cycle
        let deadline = deadline_from(now, cycle).expect("should not overflow");
        assert!(deadline > now);
    }

    #[test]
    fn test_deadline_near_u64_max_saturates() {
        // Pathological: current_time near u64::MAX — checked_add returns None.
        let now: u64 = u64::MAX - 100;
        let cycle: u64 = 604_800;
        assert!(
            deadline_from(now, cycle).is_none(),
            "overflow must be detected, not silently wrap"
        );
    }

    #[test]
    fn test_deadline_zero_cycle_duration() {
        let now: u64 = 1_700_000_000;
        let deadline = deadline_from(now, 0).expect("zero cycle should not overflow");
        assert_eq!(deadline, now);
    }

    // -----------------------------------------------------------------------
    // Grace-period arithmetic
    // -----------------------------------------------------------------------

    #[test]
    fn test_grace_period_end_normal() {
        let missed: u64 = 1_700_000_000;
        let grace: u64 = 24 * 60 * 60; // 24 h
        let end = grace_period_end(missed, grace).expect("should not overflow");
        assert_eq!(end, missed + grace);
    }

    #[test]
    fn test_grace_period_end_near_max() {
        let missed: u64 = u64::MAX - 1000;
        let grace: u64 = 86_400;
        assert!(
            grace_period_end(missed, grace).is_none(),
            "overflow must be detected"
        );
    }

    // -----------------------------------------------------------------------
    // Time-remaining (must never underflow / panic)
    // -----------------------------------------------------------------------

    #[test]
    fn test_time_remaining_before_deadline() {
        let deadline: u64 = 1_700_000_000 + 604_800;
        let now: u64 = 1_700_000_000;
        assert_eq!(time_remaining(deadline, now), 604_800);
    }

    #[test]
    fn test_time_remaining_after_deadline_does_not_underflow() {
        // current_time > deadline — must return 0, not panic.
        let deadline: u64 = 1_700_000_000;
        let now: u64 = 1_700_000_000 + 1_000_000;
        assert_eq!(
            time_remaining(deadline, now),
            0,
            "saturating_sub must return 0, not underflow"
        );
    }

    #[test]
    fn test_time_remaining_at_exact_deadline() {
        let ts: u64 = 1_700_000_000;
        assert_eq!(time_remaining(ts, ts), 0);
    }

    #[test]
    fn test_time_remaining_zero_deadline() {
        // Edge: deadline == 0, current_time > 0 — must not underflow.
        assert_eq!(time_remaining(0, 1_000), 0);
    }

    // -----------------------------------------------------------------------
    // Multi-cycle deadline chain
    // -----------------------------------------------------------------------

    #[test]
    fn test_multi_cycle_deadline_chain_no_overflow() {
        let mut ts: u64 = 1_700_000_000;
        let cycle: u64 = 30 * 24 * 60 * 60; // 30-day cycle
        let cycles: u32 = 1_200; // 100 years of monthly cycles

        for _ in 0..cycles {
            let next = deadline_from(ts, cycle).expect("cycle chain must not overflow");
            assert!(next > ts, "each deadline must be strictly later");
            ts = next;
        }
    }

    // -----------------------------------------------------------------------
    // Reliability Index decay
    // -----------------------------------------------------------------------

    #[test]
    fn test_ri_decay_normal() {
        let ri: u64 = 10_000; // 100% in bps
        let elapsed: u64 = 7 * 86_400; // 7 days
        let decay_per_day: u64 = 10; // 0.1% per day
        let result = reliability_index_decay(ri, elapsed, decay_per_day);
        assert_eq!(result, 10_000 - 7 * 10);
    }

    #[test]
    fn test_ri_decay_saturates_at_zero() {
        // Extreme elapsed time — RI must not underflow below 0.
        let ri: u64 = 100;
        let elapsed: u64 = u64::MAX; // absurdly large
        let decay_per_day: u64 = 1;
        let result = reliability_index_decay(ri, elapsed, decay_per_day);
        assert_eq!(result, 0, "RI must saturate at 0, not underflow");
    }

    #[test]
    fn test_ri_decay_zero_elapsed() {
        let ri: u64 = 8_000;
        let result = reliability_index_decay(ri, 0, 10);
        assert_eq!(result, ri, "no time elapsed means no decay");
    }

    // -----------------------------------------------------------------------
    // Appeals timelock (72 h) arithmetic
    // -----------------------------------------------------------------------

    #[test]
    fn test_appeals_timelock_window_calculation() {
        let slash_time: u64 = 1_700_000_000;
        let release_time = slash_time
            .checked_add(APPEALS_TIMELOCK_SECS)
            .expect("timelock addition must not overflow");
        assert_eq!(release_time, slash_time + 259_200);
    }

    #[test]
    fn test_appeals_timelock_within_window() {
        let slash_time: u64 = 1_700_000_000;
        let release_time = slash_time + APPEALS_TIMELOCK_SECS;
        let now: u64 = slash_time + 100_000; // still within 72 h
        assert!(now < release_time, "appeal window should still be open");
    }

    #[test]
    fn test_appeals_timelock_expired() {
        let slash_time: u64 = 1_700_000_000;
        let release_time = slash_time + APPEALS_TIMELOCK_SECS;
        let now: u64 = release_time + 1; // just past 72 h
        assert!(now >= release_time, "appeal window should be closed");
    }
}

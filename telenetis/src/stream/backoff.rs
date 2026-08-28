//! Server-authoritative reconnect policy for the live stream (plan P2).
//!
//! The Mini App keeps WS `/ws` as its primary live channel and falls back to
//! SSE `/events` only after the WS has failed repeatedly. Both sides (Rust
//! server, JS browser) must agree on the retry schedule, so the schedule is
//! defined here and served to the client via `GET /api/live/config`. The delay
//! uses exponential backoff with a deterministic jitter so a fleet of
//! reconnecting clients does not thundering-herd, while staying fully testable.

use std::time::Duration;

/// How often the WS server sends a keep-alive heartbeat frame. Without it a
/// silent-but-open socket can linger for minutes behind proxies/NATs, so the
/// client cannot tell "blocked" from "dead". The heartbeat lets the client
/// (and intermediate firewalls) treat a quiet socket as afresh within seconds.
pub const WS_KEEPALIVE_SECS: u64 = 25;

/// First WS reconnect wait (no growth) in milliseconds.
pub const RECONNECT_BASE_MS: u64 = 1_000;

/// Upper bound on the reconnect delay in milliseconds.
pub const RECONNECT_CAP_MS: u64 = 30_000;

/// Consecutive WS failures before the client falls back to SSE `/events`.
pub const RECONNECT_MAX_ATTEMPTS: u16 = 6;

/// Deterministic jitter width as a fraction of the capped delay, in tenths.
/// `delay = capped * (1 + jitter_tenths/10)` where `jitter_tenths` is in 0..=4.
const JITTER_TENTHS_MAX: u64 = 4;

/// Exponential-backoff reconnect policy. Delay before retry `attempt`
/// (1-based) is `min(base * 2^(attempt-1), cap)` plus a deterministic jitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectPolicy {
    pub base_ms: u64,
    pub cap_ms: u64,
    pub max_attempts: u16,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            base_ms: RECONNECT_BASE_MS,
            cap_ms: RECONNECT_CAP_MS,
            max_attempts: RECONNECT_MAX_ATTEMPTS,
        }
    }
}

impl ReconnectPolicy {
    /// Delay in ms before retry `attempt` (1-based). Returns `None` when
    /// `attempt` is out of range (0 or > max_attempts) — the caller should
    /// fall back to another channel or stop retrying.
    pub fn delay_ms(&self, attempt: u32, seed: u64) -> Option<u64> {
        if attempt == 0 || attempt > self.max_attempts as u32 {
            return None;
        }
        let exp = (attempt - 1).min(20);
        let base = self.base_ms.saturating_mul(1u64 << exp);
        let capped = base.min(self.cap_ms);
        let extra = capped.saturating_mul(self.jitter_tenths(seed, attempt)) / 10;
        Some(capped.saturating_add(extra))
    }

    /// Deterministic jitter in tenths (`0..=4`) derived only from `seed` and
    /// `attempt` so the schedule is reproducible in tests while still breaking
    /// any client-synchronised reconnect storm.
    pub fn jitter_tenths(&self, seed: u64, attempt: u32) -> u64 {
        // SplitMix64 finalizer — cheap, deterministic, no RNG state to share.
        const P1: u64 = 0x9E37_79B9_7F4A_7C15;
        const P2: u64 = 0xBF58_476D_1CE4_E5B9;
        const P3: u64 = 0x94D0_49BB_1331_11EB;
        let mut z = seed.wrapping_add((attempt as u64).wrapping_mul(P1));
        z = (z ^ (z >> 30)).wrapping_mul(P2);
        z = (z ^ (z >> 27)).wrapping_mul(P3);
        z ^= z >> 31;
        z % (JITTER_TENTHS_MAX + 1)
    }
}

/// Convenience for the JS bridge: the runtime keep-alive duration as a
/// [`Duration`], and the default policy's serialisable shape.
pub fn keep_alive_duration() -> Duration {
    Duration::from_secs(WS_KEEPALIVE_SECS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn out_of_range_attempts_are_none() {
        let p = ReconnectPolicy::default();
        assert_eq!(p.delay_ms(0, 0), None);
        assert_eq!(p.delay_ms(p.max_attempts as u32 + 1, 0), None);
    }

    #[test]
    fn low_attempts_grow_exponentially() {
        let p = ReconnectPolicy::default();
        let d1 = p.delay_ms(1, 0).unwrap();
        let d2 = p.delay_ms(2, 0).unwrap();
        let d3 = p.delay_ms(3, 0).unwrap();
        assert!(d1 < d2);
        assert!(d2 < d3);
        // attempt i in [base*2^(i-1), base*2^(i-1) * 1.4]
        let upper = |base: u64| base * (10 + JITTER_TENTHS_MAX) / 10;
        assert!(d1 >= p.base_ms && d1 <= upper(p.base_ms));
        assert!(d2 >= p.base_ms * 2 && d2 <= upper(p.base_ms * 2));
        assert!(d3 >= p.base_ms * 4 && d3 <= upper(p.base_ms * 4));
    }

    #[test]
    fn delay_never_exceeds_cap_plus_jitter() {
        let p = ReconnectPolicy::default();
        for attempt in 1..=p.max_attempts {
            let d = p.delay_ms(attempt as u32, 0).unwrap();
            let max = p.cap_ms + (p.cap_ms * JITTER_TENTHS_MAX) / 10;
            assert!(d <= max, "attempt {} delay {} > max {}", attempt, d, max);
        }
        // ...and sufficiently high attempts saturate at the capacity.
        let saturated = p.delay_ms(p.max_attempts as u32, 0).unwrap();
        assert!(
            saturated >= p.cap_ms,
            "saturated delay {} < cap {}",
            saturated,
            p.cap_ms
        );
    }

    #[test]
    fn delay_is_deterministic_for_same_seed_and_attempt() {
        let p = ReconnectPolicy::default();
        for attempt in 1..=p.max_attempts {
            assert_eq!(
                p.delay_ms(attempt as u32, 42),
                p.delay_ms(attempt as u32, 42)
            );
        }
    }

    #[test]
    fn different_seeds_produce_different_jitter() {
        let p = ReconnectPolicy::default();
        let a = p.delay_ms(3, 1).unwrap();
        let b = p.delay_ms(3, 2).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn jitter_tenths_stays_in_bounds() {
        let p = ReconnectPolicy::default();
        for seed in 0..10 {
            for attempt in 1..=p.max_attempts {
                let j = p.jitter_tenths(seed, attempt as u32);
                assert!(j <= JITTER_TENTHS_MAX);
            }
        }
    }

    #[test]
    fn keep_alive_duration_matches_seconds() {
        assert_eq!(
            keep_alive_duration(),
            Duration::from_secs(WS_KEEPALIVE_SECS)
        );
    }
}

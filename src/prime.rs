//! Small integer prime utilities for lab-api demos.
//!
//! All public helpers are intended for modest inputs. Callers (HTTP handlers)
//! must enforce [`MAX_PRIME_PI_N`] and [`MAX_PRIME_SCAN_N`] before expensive work.

/// Maximum `n` for the prime-counting function π(n) (sieve allocates O(n) memory).
pub const MAX_PRIME_PI_N: u64 = 1_000_000;

/// Maximum starting value for next-prime / previous-prime / gap scans.
/// Keeps worst-case trial division bounded on a small host.
pub const MAX_PRIME_SCAN_N: u64 = 1_000_000;

pub fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 || n == 3 {
        return true;
    }
    if n % 2 == 0 || n % 3 == 0 {
        return false;
    }

    let mut i = 5u64;
    while i <= n / i {
        if n % i == 0 || (i <= u64::MAX - 2 && n % (i + 2) == 0) {
            return false;
        }
        // 6k ± 1 wheel
        match i.checked_add(6) {
            Some(next) => i = next,
            None => break,
        }
    }
    true
}

pub fn next_prime(n: u64) -> Option<u64> {
    if n < 2 {
        return Some(2);
    }

    let mut candidate = n.checked_add(1)?;
    while !is_prime(candidate) {
        candidate = candidate.checked_add(1)?;
    }
    Some(candidate)
}

pub fn previous_prime(n: u64) -> Option<u64> {
    if n <= 2 {
        return None;
    }

    let mut candidate = n - 1;
    while candidate >= 2 {
        if is_prime(candidate) {
            return Some(candidate);
        }
        candidate -= 1;
    }
    None
}

/// Gap between the largest prime &lt; `n` and the smallest prime ≥ `n`
/// (for composite `n`, next is the least prime strictly greater than `n` via
/// [`next_prime`], which starts at `n + 1`).
///
/// Returns `None` if `n ≤ 2` or no neighbor exists in `u64`.
pub fn prime_gap(n: u64) -> Option<(u64, u64, u64)> {
    let previous = previous_prime(n)?;
    let next = next_prime(n)?;
    Some((previous, next, next - previous))
}

/// Prime-counting function **π(n)**: number of primes ≤ `n`.
///
/// Uses a simple sieve. Caller must ensure `n ≤ MAX_PRIME_PI_N`.
pub fn prime_pi(n: u64) -> u64 {
    if n < 2 {
        return 0;
    }

    let limit = n as usize;
    let mut sieve = vec![true; limit + 1];
    sieve[0] = false;
    sieve[1] = false;

    let mut p = 2usize;
    while p * p <= limit {
        if sieve[p] {
            let mut i = p * p;
            while i <= limit {
                sieve[i] = false;
                i += p;
            }
        }
        p += 1;
    }

    sieve.iter().filter(|&&b| b).count() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_prime_cases() {
        assert!(is_prime(2));
        assert!(is_prime(3));
        assert!(is_prime(97));
        assert!(!is_prime(0));
        assert!(!is_prime(1));
        assert!(!is_prime(100));
    }

    #[test]
    fn next_prime_cases() {
        assert_eq!(next_prime(0), Some(2));
        assert_eq!(next_prime(100), Some(101));
        assert_eq!(next_prime(u64::MAX), None);
    }

    #[test]
    fn previous_prime_cases() {
        assert_eq!(previous_prime(2), None);
        assert_eq!(previous_prime(100), Some(97));
    }

    #[test]
    fn prime_pi_cases() {
        assert_eq!(prime_pi(0), 0);
        assert_eq!(prime_pi(1), 0);
        assert_eq!(prime_pi(10), 4);
        assert_eq!(prime_pi(100), 25);
        assert_eq!(prime_pi(1000), 168);
    }

    #[test]
    fn prime_gap_around_1000() {
        let (prev, next, gap) = prime_gap(1000).unwrap();
        assert_eq!(prev, 997);
        assert_eq!(next, 1009);
        assert_eq!(gap, 12);
    }
}

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

    let mut i = 5;
    // FIXED: Division avoids multiplication overflow
    while i <= n / i {
        // FIXED: Safe checking for i + 2 to prevent overflow panics
        if n % i == 0 || (i <= u64::MAX - 2 && n % (i + 2) == 0) {
            return false;
        }
        i += 6;
    }
    true
}

pub fn next_prime(n: u64) -> Option<u64> {
    if n < 2 {
        return Some(2);
    }

    // FIXED: Uses checked_add to safely return None if we exceed u64 bounds
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

pub fn prime_gap(n: u64) -> Option<(u64, u64, u64)> {
    let previous = previous_prime(n)?;
    let next = next_prime(n)?; // Updated to match Option return type

    Some((previous, next, next - previous))
}

/// Computes the prime-counting function π(n) using an efficient Sieve of Eratosthenes.
/// Time Complexity: O(n log log n) instead of O(n sqrt(n))
pub fn prime_pi(n: u64) -> u64 {
    if n < 2 {
        return 0;
    }

    // Cast to usize for memory allocation index tracking
    let limit = n as usize;
    let mut is_prime_bitset = vec![true; limit + 1];
    is_prime_bitset[0] = false;
    is_prime_bitset[1] = false;

    let mut p = 2;
    while p * p <= limit {
        if is_prime_bitset[p] {
            let mut i = p * p;
            while i <= limit {
                is_prime_bitset[i] = false;
                i += p;
            }
        }
        p += 1;
    }

    is_prime_bitset.iter().filter(|&&b| b).count() as u64
}

/// Maximum input accepted by the public factorisation demo.
pub const MAX_FACTOR_N: u64 = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimeFactor {
    pub prime: u64,
    pub power: u32,
}

pub fn factorise(mut n: u64) -> Vec<PrimeFactor> {
    let mut factors = Vec::new();

    if n < 2 {
        return factors;
    }

    let mut count = 0;
    while n % 2 == 0 {
        n /= 2;
        count += 1;
    }

    if count > 0 {
        factors.push(PrimeFactor {
            prime: 2,
            power: count,
        });
    }

    let mut p = 3;
    while p <= n / p {
        count = 0;

        while n % p == 0 {
            n /= p;
            count += 1;
        }

        if count > 0 {
            factors.push(PrimeFactor {
                prime: p,
                power: count,
            });
        }

        p += 2;
    }

    if n > 1 {
        factors.push(PrimeFactor { prime: n, power: 1 });
    }

    factors
}

pub fn totient(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }

    let mut result = n;
    for factor in factorise(n) {
        result = result / factor.prime * (factor.prime - 1);
    }
    result
}

pub fn mobius(n: u64) -> i8 {
    if n == 0 {
        return 0;
    }

    let factors = factorise(n);
    if factors.iter().any(|factor| factor.power > 1) {
        return 0;
    }

    if factors.len() % 2 == 0 {
        1
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factorise_one() {
        assert!(factorise(0).is_empty());
        assert!(factorise(1).is_empty());
    }

    #[test]
    fn factorise_two() {
        assert_eq!(factorise(2), vec![PrimeFactor { prime: 2, power: 1 }]);
    }

    #[test]
    fn factorise_prime() {
        assert_eq!(
            factorise(97),
            vec![PrimeFactor {
                prime: 97,
                power: 1,
            }]
        );
    }

    #[test]
    fn factorise_composite() {
        assert_eq!(
            factorise(360),
            vec![
                PrimeFactor { prime: 2, power: 3 },
                PrimeFactor { prime: 3, power: 2 },
                PrimeFactor { prime: 5, power: 1 },
            ]
        );
    }

    #[test]
    fn totient_cases() {
        assert_eq!(totient(0), 0);
        assert_eq!(totient(1), 1);
        assert_eq!(totient(10), 4);
        assert_eq!(totient(36), 12);
        assert_eq!(totient(97), 96);
    }

    #[test]
    fn mobius_cases() {
        assert_eq!(mobius(0), 0);
        assert_eq!(mobius(1), 1);
        assert_eq!(mobius(30), -1);
        assert_eq!(mobius(36), 0);
        assert_eq!(mobius(210), 1);
    }
}

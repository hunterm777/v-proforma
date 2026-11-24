//! Integer cents and the one rounding rule.

/// Integer cents.
pub type Cents = i64;

/// Rounds an amount in cents to whole cents, half away from zero.
///
/// Same result as `f64::round` for every finite input (`x - x.trunc()` is
/// exact), but built on `trunc`, which is one instruction in WebAssembly,
/// where `round` is a software routine.
pub fn round(x: f64) -> Cents {
    let t = x.trunc();
    let r = if (x - t).abs() >= 0.5 {
        t + x.signum()
    } else {
        t
    };
    r as Cents
}

/// Dollars to cents, half away from zero.
pub fn cents(dollars: f64) -> Cents {
    round(dollars * 100.0)
}

/// Cents to dollars (for report output only).
pub fn dollars(c: Cents) -> f64 {
    c as f64 / 100.0
}

/// `base^exp` through libm so every build gives the same bits.
pub fn pow(base: f64, exp: f64) -> f64 {
    libm::pow(base, exp)
}

/// `(1 + g)^years` for annual growth.
pub fn grow(g: f64, years: i64) -> f64 {
    libm::pow(1.0 + g, years as f64)
}

#[cfg(test)]
mod tests {
    #[test]
    fn round_matches_std_round() {
        let xs = [
            0.5,
            -0.5,
            1.5,
            2.5,
            -2.5,
            0.49999999999999994,
            -0.49999999999999994,
            4503599627370495.5,
            1e15 + 0.5,
            12345.4999,
            -7.5000001,
            0.0,
            -0.0,
        ];
        for x in xs {
            assert_eq!(super::round(x), x.round() as i64, "{x}");
        }
        let mut s: u64 = 0x9E3779B97F4A7C15;
        for _ in 0..200_000 {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            let x = (s as f64 / u64::MAX as f64 - 0.5) * 2e9;
            assert_eq!(super::round(x), x.round() as i64, "{x}");
            let h = (x as i64) as f64 + 0.5;
            assert_eq!(super::round(h), h.round() as i64, "{h}");
        }
    }

    use super::*;

    #[test]
    fn rounds_half_away_from_zero() {
        assert_eq!(round(0.5), 1);
        assert_eq!(round(-0.5), -1);
        assert_eq!(round(1.4999), 1);
        assert_eq!(cents(12.34), 1234);
        assert_eq!(cents(0.29), 29);
        assert_eq!(cents(-7.01), -701);
        assert_eq!(dollars(1235), 12.35);
        assert_eq!(grow(0.03, 2), libm::pow(1.03, 2.0));
        assert_eq!(pow(2.0, 10.0), 1024.0);
    }
}

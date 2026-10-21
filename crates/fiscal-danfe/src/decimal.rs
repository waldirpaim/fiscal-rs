//! Exact decimal numbers for fiscal amounts.
//!
//! NF-e amounts are written in the XML as plain decimals. They are kept exact (no binary
//! floating point) so the values printed on the DANFE are the ones in the document.

use std::cmp::Ordering;
use std::ops::{Add, Neg, Sub};

/// A decimal number: `mantissa / 10^scale`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Decimal {
    mantissa: i128,
    scale: u32,
}

/// Largest scale kept; more digits are rounded away.
const MAX_SCALE: u32 = 18;

fn pow10(n: u32) -> i128 {
    10i128.pow(n)
}

impl Decimal {
    /// Zero.
    pub const ZERO: Self = Self {
        mantissa: 0,
        scale: 0,
    };

    /// Parses `[-+]digits[.digits]`; surrounding spaces are ignored. A decimal comma is
    /// accepted too. Returns `None` for anything else (including the empty string).
    pub fn parse(text: &str) -> Option<Self> {
        let t = text.trim();
        let (negative, t) = match t.as_bytes().first()? {
            b'-' => (true, &t[1..]),
            b'+' => (false, &t[1..]),
            _ => (false, t),
        };
        let (int_part, frac_part) = match t.split_once(['.', ',']) {
            Some((i, f)) => (i, f),
            None => (t, ""),
        };
        if (int_part.is_empty() && frac_part.is_empty())
            || !int_part.bytes().all(|b| b.is_ascii_digit())
            || !frac_part.bytes().all(|b| b.is_ascii_digit())
        {
            return None;
        }
        // Drop digits beyond the supported scale (rounding half up on the first dropped one).
        let (frac_kept, round_up) = if frac_part.len() as u32 > MAX_SCALE {
            let (kept, rest) = frac_part.split_at(MAX_SCALE as usize);
            (kept, rest.as_bytes()[0] >= b'5')
        } else {
            (frac_part, false)
        };
        let digits = format!("{int_part}{frac_kept}");
        let mut mantissa: i128 = if digits.is_empty() {
            0
        } else {
            digits.parse().ok()?
        };
        if round_up {
            mantissa += 1;
        }
        Some(Self {
            mantissa: if negative { -mantissa } else { mantissa },
            scale: frac_kept.len() as u32,
        })
    }

    /// Builds a decimal from an integer.
    pub fn from_i64(value: i64) -> Self {
        Self {
            mantissa: i128::from(value),
            scale: 0,
        }
    }

    /// `true` for zero.
    pub fn is_zero(&self) -> bool {
        self.mantissa == 0
    }

    /// `true` when greater than zero.
    pub fn is_positive(&self) -> bool {
        self.mantissa > 0
    }

    /// The nearest `f64` (used only where the original works in floating point, such as
    /// percentages).
    pub fn to_f64(self) -> f64 {
        self.mantissa as f64 / pow10(self.scale) as f64
    }

    /// Rounds half away from zero to `places` decimal places.
    pub fn round(self, places: u32) -> Self {
        if self.scale <= places {
            return self;
        }
        let drop = self.scale - places;
        let divisor = pow10(drop);
        let half = divisor / 2;
        let abs = self.mantissa.abs();
        let rounded = (abs + half) / divisor;
        Self {
            mantissa: if self.mantissa < 0 { -rounded } else { rounded },
            scale: places,
        }
    }

    /// Rescales to exactly `scale` decimals (no loss: only used with a scale at least as
    /// large as the current one).
    fn with_scale(self, scale: u32) -> i128 {
        self.mantissa * pow10(scale - self.scale)
    }

    /// Formats like Delphi's `FormatFloat('#,0.00', v)` with the Brazilian separators:
    /// `decimals` digits, `.` as thousands separator and `,` as decimal separator.
    pub fn format_br(self, decimals: u32, thousands: bool) -> String {
        let r = self.round(decimals);
        // `round` leaves at most `decimals` decimal places.
        let scaled = r.mantissa.abs() * pow10(decimals - r.scale);
        let int_part = scaled / pow10(decimals);
        let frac_part = scaled % pow10(decimals);
        let mut int_str = int_part.to_string();
        if thousands {
            let mut grouped = String::new();
            for (i, c) in int_str.chars().rev().enumerate() {
                if i > 0 && i % 3 == 0 {
                    grouped.push('.');
                }
                grouped.push(c);
            }
            int_str = grouped.chars().rev().collect();
        }
        let negative = r.mantissa < 0 && scaled != 0;
        let mut out = String::new();
        if negative {
            out.push('-');
        }
        out.push_str(&int_str);
        if decimals > 0 {
            out.push(',');
            out.push_str(&format!("{frac_part:0width$}", width = decimals as usize));
        }
        out
    }

    /// `#,0.00`
    pub fn money(self) -> String {
        self.format_br(2, true)
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Decimal {}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Decimal {
    fn cmp(&self, other: &Self) -> Ordering {
        let scale = self.scale.max(other.scale);
        self.with_scale(scale).cmp(&other.with_scale(scale))
    }
}

impl Add for Decimal {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        let scale = self.scale.max(rhs.scale);
        Self {
            mantissa: self.with_scale(scale) + rhs.with_scale(scale),
            scale,
        }
    }
}

impl Sub for Decimal {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self + (-rhs)
    }
}

impl Neg for Decimal {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            mantissa: -self.mantissa,
            scale: self.scale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        Decimal::parse(s).expect("decimal")
    }

    #[test]
    fn parses_plain_decimals() {
        assert_eq!(d("12.5").to_f64(), 12.5);
        assert_eq!(d("  -0.0150 ").to_f64(), -0.015);
        assert_eq!(d("7").to_f64(), 7.0);
        assert_eq!(d("3,25").to_f64(), 3.25);
        assert_eq!(d(".5").to_f64(), 0.5);
        assert!(Decimal::parse("").is_none());
        assert!(Decimal::parse("abc").is_none());
        assert!(Decimal::parse("1.2.3").is_none());
        assert!(Decimal::parse("-").is_none());
    }

    #[test]
    fn compares_across_scales() {
        assert_eq!(d("1.50"), d("1.5"));
        assert!(d("1.51") > d("1.5"));
        assert!(d("-1") < d("0"));
        assert!(Decimal::ZERO.is_zero());
        assert!(d("0.0001").is_positive());
    }

    #[test]
    fn adds_and_subtracts_exactly() {
        assert_eq!(d("0.1") + d("0.2"), d("0.3"));
        assert_eq!(d("10.00") - d("0.0001"), d("9.9999"));
    }

    #[test]
    fn rounds_half_away_from_zero() {
        assert_eq!(d("1.005").round(2), d("1.01"));
        assert_eq!(d("-1.005").round(2), d("-1.01"));
        assert_eq!(d("2.5").round(0), d("3"));
        assert_eq!(d("1.2").round(4), d("1.2"));
    }

    #[test]
    fn formats_like_delphi_with_brazilian_separators() {
        assert_eq!(d("1234567.891").format_br(2, true), "1.234.567,89");
        assert_eq!(d("0").money(), "0,00");
        assert_eq!(d("-1234.5").money(), "-1.234,50");
        assert_eq!(d("12.3456").format_br(4, true), "12,3456");
        assert_eq!(d("1234.5").format_br(0, true), "1.235");
        assert_eq!(d("1234.5678").format_br(3, true), "1.234,568");
        assert_eq!(d("999.995").money(), "1.000,00");
        assert_eq!(d("1234.5").format_br(2, false), "1234,50");
        assert_eq!(d("-0.001").money(), "0,00");
        assert_eq!(d("100").money(), "100,00");
    }

    #[test]
    fn keeps_ten_decimal_unit_values_exact() {
        assert_eq!(d("1234.5678901234").format_br(4, true), "1.234,5679");
        assert_eq!(d("0.0000000001").format_br(10, false), "0,0000000001");
    }
}

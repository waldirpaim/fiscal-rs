//! Number formatting compatible with the Delphi/FPC `Format('%.Nf')` and `FloatToStr` calls
//! the original library uses to write PDF operands.
//!
//! Delphi first reduces a `Double` to 15 significant decimal digits and then rounds half
//! away from zero; Rust's `{:.N}` rounds the exact binary value half-to-even. Matching the
//! Delphi behaviour keeps the content streams byte-identical to the original engine.

/// Splits `v > 0` into 15 significant decimal digits and the decimal exponent of the first
/// digit (`v = d1.d2d3... * 10^exp`).
fn digits15(v: f64) -> (Vec<u8>, i32) {
    let s = format!("{v:.14e}");
    let (mantissa, exp) = s.split_once('e').unwrap_or((s.as_str(), "0"));
    let digits = mantissa
        .bytes()
        .filter(u8::is_ascii_digit)
        .map(|b| b - b'0')
        .collect();
    (digits, exp.parse().unwrap_or(0))
}

/// Equivalent of `Format('%.<decimals>f', [v])` with a `.` decimal separator.
pub(crate) fn fixed(v: f64, decimals: usize) -> String {
    if !v.is_finite() || v == 0.0 {
        return zero_fixed(decimals);
    }
    let negative = v < 0.0;
    let (mut digits, exp) = digits15(v.abs());
    // Number of digits before the decimal point.
    let mut point = exp + 1;
    if point <= 0 {
        let zeros = (1 - point) as usize;
        digits.splice(0..0, std::iter::repeat_n(0, zeros));
        point = 1;
    }
    let mut point = point as usize;
    let keep = point + decimals;
    if digits.len() < keep {
        digits.resize(keep, 0);
    } else if digits.len() > keep {
        let round_up = digits[keep] >= 5;
        digits.truncate(keep);
        if round_up {
            let mut i = keep;
            loop {
                if i == 0 {
                    digits.insert(0, 1);
                    point += 1;
                    break;
                }
                i -= 1;
                if digits[i] == 9 {
                    digits[i] = 0;
                } else {
                    digits[i] += 1;
                    break;
                }
            }
        }
    }
    let all_zero = digits.iter().all(|d| *d == 0);
    let mut out = String::with_capacity(digits.len() + 2);
    if negative && !all_zero {
        out.push('-');
    }
    out.extend(digits[..point].iter().map(|d| char::from(b'0' + d)));
    if decimals > 0 {
        out.push('.');
        out.extend(digits[point..].iter().map(|d| char::from(b'0' + d)));
    }
    out
}

fn zero_fixed(decimals: usize) -> String {
    if decimals == 0 {
        "0".to_owned()
    } else {
        format!("0.{}", "0".repeat(decimals))
    }
}

/// Equivalent of `FloatToStr(v)` (general format, 15 significant digits, `.` separator).
pub(crate) fn float_to_str(v: f64) -> String {
    if !v.is_finite() {
        return "0".to_owned();
    }
    if v == 0.0 {
        return "0".to_owned();
    }
    let (mut digits, exp) = digits15(v.abs());
    if !(-5..15).contains(&exp) {
        return format!("{v}");
    }
    while digits.len() > 1 && digits.last() == Some(&0) {
        digits.pop();
    }
    let int_len = exp + 1;
    let mut out = String::new();
    if v < 0.0 {
        out.push('-');
    }
    let push = |out: &mut String, ds: &[u8]| out.extend(ds.iter().map(|d| char::from(b'0' + d)));
    if int_len <= 0 {
        out.push_str("0.");
        out.push_str(&"0".repeat((-int_len) as usize));
        push(&mut out, &digits);
    } else if digits.len() <= int_len as usize {
        push(&mut out, &digits);
        out.push_str(&"0".repeat(int_len as usize - digits.len()));
    } else {
        push(&mut out, &digits[..int_len as usize]);
        out.push('.');
        push(&mut out, &digits[int_len as usize..]);
    }
    out
}

/// Delphi `RoundTo(value, digit)`: rounds to `10^digit` with `Round`, that is, half to even
/// (`digit` is `0` for integers, `-2` for hundredths). The ACBr compiled with Free Pascal and
/// the compatibility `RoundTo` of the ACBr itself (`Round(AValue / LFactor) * LFactor`) do the
/// same: `RoundTo(30.5, 0)` is `30`.
pub fn round_to(value: f64, digit: i32) -> f64 {
    let factor = 10f64.powi(digit);
    (value / factor).round_ties_even() * factor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_rounds_half_away_from_zero() {
        assert_eq!(fixed(0.125, 2), "0.13");
        assert_eq!(fixed(1.005, 2), "1.01");
        assert_eq!(fixed(-1.005, 2), "-1.01");
        assert_eq!(fixed(2.5, 0), "3");
    }

    #[test]
    fn fixed_handles_small_large_and_carry() {
        assert_eq!(fixed(0.0, 2), "0.00");
        assert_eq!(fixed(0.004, 2), "0.00");
        assert_eq!(fixed(-0.004, 2), "0.00");
        assert_eq!(fixed(0.996, 2), "1.00");
        assert_eq!(fixed(99.995, 2), "100.00");
        assert_eq!(fixed(595.2755905511812, 2), "595.28");
        assert_eq!(fixed(841.89, 3), "841.890");
        assert_eq!(fixed(12.0, 2), "12.00");
        assert_eq!(fixed(123456.789, 1), "123456.8");
    }

    #[test]
    fn round_to_rounds_half_to_even_like_delphi() {
        // Half to even, as `Round` does: 30.5 is 30 (the destinatário of the DANFE depends on it).
        assert_eq!(round_to(30.5, 0), 30.0);
        assert_eq!(round_to(31.5, 0), 32.0);
        assert_eq!(round_to(2.5, 0), 2.0);
        assert_eq!(round_to(3.5, 0), 4.0);
        assert_eq!(round_to(0.5, 0), 0.0);
        assert_eq!(round_to(-2.5, 0), -2.0);
        assert_eq!(round_to(-3.5, 0), -4.0);
        assert_eq!(round_to(84.46, 0), 84.0);
        assert_eq!(round_to(84.5001, 0), 85.0);
        assert!((round_to(12.3449, -2) - 12.34).abs() < 1e-9);
        assert!((round_to(12.346, -2) - 12.35).abs() < 1e-9);
    }

    #[test]
    fn float_to_str_matches_delphi_general_format() {
        assert_eq!(float_to_str(595.28), "595.28");
        assert_eq!(float_to_str(595.2755905511812), "595.275590551181");
        assert_eq!(float_to_str(1.3), "1.3");
        assert_eq!(float_to_str(612.0), "612");
        assert_eq!(float_to_str(0.5), "0.5");
        assert_eq!(float_to_str(0.0), "0");
        assert_eq!(float_to_str(-3.25), "-3.25");
    }
}

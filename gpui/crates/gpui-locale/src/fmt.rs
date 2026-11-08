//! Locale-aware number formatting: the port of `src/utils/localeNumber.ts`.
//!
//! `toFixed` always writes `.` as the decimal separator, so a German UI would
//! show `28.03 MB` next to a plural that prints `560.464 Polygone`. The React
//! side routes every user-visible number through `Intl.NumberFormat`; the GPUI
//! port routes every user-visible number through this module — nothing else in
//! the crate builds digits with `format!`, and call sites should not either.
//!
//! # What is reproduced, and how
//!
//! `Intl.NumberFormat` is not available, and the six shipped locales
//! (`en, es, de, fr, sk, cs`) need only a decimal separator, a group separator
//! and grouping thresholds — all of which live on [`Locale`]. Two behaviours are
//! less obvious and are reproduced on purpose:
//!
//! - **Rounding follows the shortest decimal form of the double, not its exact
//!   binary expansion.** `Intl.NumberFormat` formats `1.005` with two fraction
//!   digits as `1.01` (the shortest form is exactly `1.005`, which ties away
//!   from zero), while `(1.005).toFixed(2)` is `1.00` (the exact expansion is
//!   `1.00499999…`). Ties round away from zero: `2.675 → 2.68`, `1.25 → 1.3`,
//!   `-2.5 → -3`. [`fixed`] and [`number`] therefore agree with `Intl` rather
//!   than with `toFixed`.
//! - **Non-finite values print as `NaN`, `∞` and `-∞`**, which is what
//!   `Intl.NumberFormat` emits for every shipped locale.
//!
//! The digits themselves come from [`ryu`], which prints the shortest string
//! that round-trips to the same double — the same decimal form `Intl` rounds.
//! Rounding is then integer arithmetic on those digits, so it cannot inherit a
//! second floating-point error.

use std::borrow::Cow;

use crate::{Locale, SharedString};

/// The subset of `Intl.NumberFormatOptions` the port needs.
///
/// `localeNumber.ts` passes raw `Intl.NumberFormatOptions` through. The GPUI
/// port sets only fraction digits and grouping, so currency, percent,
/// notation and the sign/compact options are deliberately absent: no shipped
/// catalogue needs them, and adding them without CLDR data would be a guess.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NumberOptions {
    minimum_fraction_digits: usize,
    maximum_fraction_digits: usize,
    use_grouping: bool,
}

impl NumberOptions {
    /// `Intl.NumberFormat` with no options: up to three fraction digits,
    /// grouped.
    pub const fn new() -> Self {
        Self { minimum_fraction_digits: 0, maximum_fraction_digits: 3, use_grouping: true }
    }

    /// `formatFixed`'s `{ minimumFractionDigits: n, maximumFractionDigits: n }`.
    pub const fn fraction_digits(digits: usize) -> Self {
        Self { minimum_fraction_digits: digits, maximum_fraction_digits: digits, use_grouping: true }
    }

    /// Grouping off, as `formatFieldNumber`'s `{ useGrouping: false }` does.
    pub const fn without_grouping(mut self) -> Self {
        self.use_grouping = false;
        self
    }

    /// The fewest fraction digits always shown.
    pub const fn minimum_fraction_digits(&self) -> usize {
        self.minimum_fraction_digits
    }

    /// The most fraction digits ever shown; further digits are rounded away.
    pub const fn maximum_fraction_digits(&self) -> usize {
        self.maximum_fraction_digits
    }

    /// Whether thousands are grouped, e.g. `1,234.5` vs `1234.5`.
    pub const fn grouped(&self) -> bool {
        self.use_grouping
    }
}

impl Default for NumberOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Formats `value` for `locale` — `Intl.NumberFormat(locale, options).format`.
pub fn number(locale: &Locale, value: f64, options: NumberOptions) -> SharedString {
    render(
        locale,
        value,
        options.minimum_fraction_digits,
        options.maximum_fraction_digits,
        options.use_grouping,
        true,
    )
    .into()
}

/// The locale-aware counterpart of `value.toFixed(fraction_digits)`.
///
/// Exactly `fraction_digits` fraction digits, grouped: German
/// `fixed(1234.5, 2)` is `1.234,50`, English is `1,234.50`.
pub fn fixed(locale: &Locale, value: f64, fraction_digits: usize) -> SharedString {
    render(locale, value, fraction_digits, fraction_digits, true, true).into()
}

/// A value as an editable field shows it: no grouping, no trailing zeros, and
/// never `-0`.
///
/// The React version rounds through `toFixed` first
/// (`Number(value.toFixed(max)) || 0`) and formats the result. This port rounds
/// the formatted digits instead, which matches `Intl.NumberFormat`; the two
/// disagree only where a value's shortest decimal form straddles a rounding
/// boundary (`1.005` at two digits), and this port stays consistent with
/// [`fixed`].
pub fn field(locale: &Locale, value: f64, max_fraction_digits: usize) -> SharedString {
    render(locale, value, 0, max_fraction_digits, false, false).into()
}

/// The character `locale` writes between a number's whole and its fraction.
pub fn decimal_separator(locale: &Locale) -> &'static str {
    locale.decimal_separator()
}

/// Reads a number typed into a field: the locale's decimal separator, or `.`,
/// which is what a number pad or a habit often types. `NaN` when the text, all
/// of it, is not a number.
pub fn parse(locale: &Locale, text: &str) -> f64 {
    let text = text.trim();
    if text.is_empty() {
        return f64::NAN;
    }

    let separator = locale.decimal_separator();
    let candidate = if separator == "." {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(text.replace(separator, "."))
    };
    candidate.parse::<f64>().unwrap_or(f64::NAN)
}

/// Number formatting bound to one locale — `localeFormat(locale)` in
/// `localeNumber.ts`, and the `fmt` a React component gets from
/// `useLocaleFormat()`.
///
/// Cheaper than the TS object it replaces (no `Intl` formatter cache): a
/// [`Locale`] is a small string, so building one per call site is fine, and
/// keeping one around avoids re-parsing the tag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocaleFormat {
    locale: Locale,
}

impl LocaleFormat {
    /// The formatters for `locale`.
    pub fn new(locale: &Locale) -> Self {
        Self { locale: locale.clone() }
    }

    /// The locale these formatters write for.
    pub fn locale(&self) -> &Locale {
        &self.locale
    }

    /// `Intl.NumberFormat(locale).format(value)`.
    pub fn number(&self, value: f64) -> SharedString {
        number(&self.locale, value, NumberOptions::new())
    }

    /// `formatNumber` with explicit options.
    pub fn number_with(&self, value: f64, options: NumberOptions) -> SharedString {
        number(&self.locale, value, options)
    }

    /// `fixed` for this locale.
    pub fn fixed(&self, value: f64, fraction_digits: usize) -> SharedString {
        fixed(&self.locale, value, fraction_digits)
    }

    /// `field` for this locale.
    pub fn field(&self, value: f64, max_fraction_digits: usize) -> SharedString {
        field(&self.locale, value, max_fraction_digits)
    }

    /// `parse` for this locale.
    pub fn parse(&self, text: &str) -> f64 {
        parse(&self.locale, text)
    }

    /// `decimal_separator` for this locale.
    pub fn decimal_separator(&self) -> &'static str {
        self.locale.decimal_separator()
    }
}

/// The one path from an `f64` to a user-visible number.
fn render(
    locale: &Locale,
    value: f64,
    minimum_fraction_digits: usize,
    maximum_fraction_digits: usize,
    use_grouping: bool,
    keep_negative_zero: bool,
) -> String {
    if value.is_nan() {
        return String::from("NaN");
    }
    if value.is_infinite() {
        return String::from(if value.is_sign_negative() { "-∞" } else { "∞" });
    }

    let minimum = minimum_fraction_digits.min(maximum_fraction_digits);
    let negative = value.is_sign_negative();
    let (digits, exponent) = decimal_digits(value);
    let rounded = round_half_away(&digits, exponent, maximum_fraction_digits);
    let (integer, mut fraction) = split_fraction(&rounded, maximum_fraction_digits);
    while fraction.len() > minimum && fraction.last() == Some(&b'0') {
        fraction.pop();
    }

    // `Intl` keeps the sign of a negative zero (`fixed(-0.0, 2)` is `-0.00`) but
    // an editable field shows `0`.
    let zero = integer
        .iter()
        .chain(fraction.iter())
        .all(|digit| *digit == b'0');
    let signed = negative && (keep_negative_zero || !zero);

    let mut out = String::with_capacity(integer.len() + fraction.len() + 16);
    if signed {
        out.push('-');
    }
    push_integer(&mut out, &integer, locale, use_grouping);
    if !fraction.is_empty() {
        out.push_str(locale.decimal_separator());
        push_digits(&mut out, &fraction);
    }
    out
}

/// The shortest round-tripping decimal digits of `value.abs()`.
///
/// Returns the significant digits (no leading or trailing zeros) and the power
/// of ten they are scaled by: `1234.5` becomes `(["1","2","3","4","5"], -1)`
/// and `100` becomes `(["1"], 2)`.
fn decimal_digits(value: f64) -> (Vec<u8>, i32) {
    let mut buffer = ryu::Buffer::new();
    let repr = buffer.format(value.abs());

    let (mantissa, exponent) = match repr.find('e') {
        Some(index) => (&repr[..index], repr[index + 1..].parse::<i32>().unwrap_or(0)),
        None => (repr, 0),
    };
    let (integer, fraction) = match mantissa.find('.') {
        Some(index) => (&mantissa[..index], &mantissa[index + 1..]),
        None => (mantissa, ""),
    };

    let mut digits: Vec<u8> = integer.bytes().chain(fraction.bytes()).collect();
    let exponent = exponent - fraction.len() as i32;

    let Some(first) = digits.iter().position(|digit| *digit != b'0') else {
        return (vec![b'0'], 0);
    };
    let mut digits = digits.split_off(first);

    let mut trailing = 0;
    while digits.last() == Some(&b'0') {
        digits.pop();
        trailing += 1;
    }
    (digits, exponent + trailing)
}

/// `digits × 10^(exponent + fraction_digits)`, rounded half away from zero.
///
/// Because `digits` carries the shortest form and therefore never ends in `0`,
/// a `5` at the cut is either an exact tie or, when digits follow it, more than
/// half — both round up. That is `Intl`'s rounding.
fn round_half_away(digits: &[u8], exponent: i32, fraction_digits: usize) -> Vec<u8> {
    let shift = exponent + fraction_digits as i32;

    if shift >= 0 {
        let mut rounded = digits.to_vec();
        rounded.resize(rounded.len() + shift as usize, b'0');
        return rounded;
    }

    let dropped = (-shift) as usize;
    if dropped > digits.len() {
        // Less than a tenth of one unit at this precision.
        return vec![b'0'];
    }
    if dropped == digits.len() {
        return if digits[0] >= b'5' { vec![b'1'] } else { vec![b'0'] };
    }

    let kept = digits.len() - dropped;
    let mut rounded = digits[..kept].to_vec();
    if digits[kept] >= b'5' {
        increment(&mut rounded);
    }
    rounded
}

/// Adds one to a decimal digit string, carrying as needed (`999` becomes
/// `1000`).
fn increment(digits: &mut Vec<u8>) {
    for digit in digits.iter_mut().rev() {
        if *digit == b'9' {
            *digit = b'0';
        } else {
            *digit += 1;
            return;
        }
    }
    digits.insert(0, b'1');
}

/// Splits rounded digits into the whole part and exactly `fraction_digits`
/// fraction digits, padding with zeros: `"1235"` at two digits is
/// `("12", "35")`, `"1"` at two digits is `("0", "01")`.
fn split_fraction(digits: &[u8], fraction_digits: usize) -> (Vec<u8>, Vec<u8>) {
    if digits.len() > fraction_digits {
        let split = digits.len() - fraction_digits;
        (digits[..split].to_vec(), digits[split..].to_vec())
    } else {
        let mut fraction = vec![b'0'; fraction_digits - digits.len()];
        fraction.extend_from_slice(digits);
        (vec![b'0'], fraction)
    }
}

fn push_integer(out: &mut String, digits: &[u8], locale: &Locale, use_grouping: bool) {
    const GROUP: usize = 3;

    if !use_grouping || digits.len() < GROUP + locale.minimum_grouping_digits() {
        push_digits(out, digits);
        return;
    }

    let head = digits.len() % GROUP;
    if head != 0 {
        push_digits(out, &digits[..head]);
    }

    let mut index = head;
    while index < digits.len() {
        if index != 0 {
            out.push_str(locale.group_separator());
        }
        push_digits(out, &digits[index..index + GROUP]);
        index += GROUP;
    }
}

fn push_digits(out: &mut String, digits: &[u8]) {
    // Digits are ASCII by construction: ryu emits them and rounding only adds
    // `b'0'..=b'9'`.
    out.push_str(std::str::from_utf8(digits).expect("decimal digits are ASCII"));
}

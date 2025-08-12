// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! From the GNU docs:
//!
//! > If you precede a number with ‘@’, it represents an internal timestamp as
//! > a count of seconds.  The number can contain an internal decimal point
//! > (either ‘.’ or ‘,’); any excess precision not supported by the internal
//! > representation is truncated toward minus infinity.  Such a number cannot
//! > be combined with any other date item, as it specifies a complete
//! > timestamp.
//! >
//! >    On most hosts, these counts ignore the presence of leap seconds.  For
//! > example, on most hosts ‘@1483228799’ represents 2016-12-31 23:59:59 UTC,
//! > ‘@1483228800’ represents 2017-01-01 00:00:00 UTC, and there is no way to
//! > represent the intervening leap second 2016-12-31 23:59:60 UTC.

use winnow::{
    ascii::digit1,
    combinator::{opt, preceded},
    token::one_of,
    ModalResult, Parser,
};

use super::primitive::{dec_uint, s};

/// Represents a timestamp with nanosecond accuracy.
///
/// # Invariants
///
/// - `nanosecond` is always in the range of `0..1_000_000_000`.
/// - Negative timestamps are represented by a negative `second` value and a
///   positive `nanosecond` value.
#[derive(Debug, PartialEq)]
pub(crate) struct Timestamp {
    pub(crate) second: i64,
    pub(crate) nanosecond: u32,
}

/// Parse a timestamp in the form of `1234567890` or `-1234567890.12345` or
/// `1234567890,12345`.
pub(crate) fn parse(input: &mut &str) -> ModalResult<Timestamp> {
    (s("@"), opt(s(one_of(['-', '+']))), sec_and_nsec)
        .verify_map(|(_, sign, (sec, nsec))| {
            let sec = i64::try_from(sec).ok()?;
            let (second, nanosecond) = match (sign, nsec) {
                (Some('-'), 0) => (-sec, 0),
                // Truncate towards minus infinity.
                (Some('-'), _) => ((-sec).checked_sub(1)?, 1_000_000_000 - nsec),
                _ => (sec, nsec),
            };
            Some(Timestamp { second, nanosecond })
        })
        .parse_next(input)
}

/// Parse a second value in the form of `1234567890` or `1234567890.12345` or
/// `1234567890,12345`.
///
/// The first part represents whole seconds. The optional second part represents
/// fractional seconds, parsed as a nanosecond value from up to 9 digits
/// (padded with zeros on the right if fewer digits are present). If the second
/// part is omitted, it defaults to 0 nanoseconds.
pub(super) fn sec_and_nsec(input: &mut &str) -> ModalResult<(u64, u32)> {
    (s(dec_uint), opt(preceded(one_of(['.', ',']), digit1)))
        .verify_map(|(sec, opt_nsec_str)| match opt_nsec_str {
            Some(nsec_str) if nsec_str.len() >= 9 => Some((sec, nsec_str[..9].parse().ok()?)),
            Some(nsec_str) => {
                let multiplier = 10_u32.pow(9 - nsec_str.len() as u32);
                Some((sec, nsec_str.parse::<u32>().ok()?.checked_mul(multiplier)?))
            }
            None => Some((sec, 0)),
        })
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sec_and_nsec_test() {
        let mut input = "1234567890";
        assert_eq!(sec_and_nsec(&mut input).unwrap(), (1234567890, 0));

        let mut input = "1234567890.12345";
        assert_eq!(sec_and_nsec(&mut input).unwrap(), (1234567890, 123450000));

        let mut input = "1234567890,12345";
        assert_eq!(sec_and_nsec(&mut input).unwrap(), (1234567890, 123450000));

        let mut input = "1234567890.1234567890123";
        assert_eq!(sec_and_nsec(&mut input).unwrap(), (1234567890, 123456789));
    }

    #[test]
    fn timestamp() {
        let mut input = "@1234567890";
        assert_eq!(
            parse(&mut input).unwrap(),
            Timestamp {
                second: 1234567890,
                nanosecond: 0,
            }
        );

        let mut input = "@ 1234567890";
        assert_eq!(
            parse(&mut input).unwrap(),
            Timestamp {
                second: 1234567890,
                nanosecond: 0,
            }
        );

        let mut input = "@ -1234567890";
        assert_eq!(
            parse(&mut input).unwrap(),
            Timestamp {
                second: -1234567890,
                nanosecond: 0,
            }
        );

        let mut input = "@ - 1234567890";
        assert_eq!(
            parse(&mut input).unwrap(),
            Timestamp {
                second: -1234567890,
                nanosecond: 0,
            }
        );

        let mut input = "@1234567890.12345";
        assert_eq!(
            parse(&mut input).unwrap(),
            Timestamp {
                second: 1234567890,
                nanosecond: 123450000,
            }
        );

        let mut input = "@1234567890,12345";
        assert_eq!(
            parse(&mut input).unwrap(),
            Timestamp {
                second: 1234567890,
                nanosecond: 123450000,
            }
        );

        let mut input = "@-1234567890.12345";
        assert_eq!(
            parse(&mut input).unwrap(),
            Timestamp {
                second: -1234567891,
                nanosecond: 876550000,
            }
        );
    }
}

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

use winnow::{combinator::preceded, ModalResult, Parser};

use super::primitive::{float, s};

/// Parse a timestamp in the form of `@1234567890`.
pub fn parse(input: &mut &str) -> ModalResult<f64> {
    s(preceded("@", float)).parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::parse;

    fn float_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < f64::EPSILON
    }

    #[test]
    fn float() {
        let mut input = "@1234567890";
        assert!(float_eq(parse(&mut input).unwrap(), 1234567890.0));

        let mut input = "@1234567890.12345";
        assert!(float_eq(parse(&mut input).unwrap(), 1234567890.12345));

        let mut input = "@1234567890,12345";
        assert!(float_eq(parse(&mut input).unwrap(), 1234567890.12345));

        let mut input = "@-1234567890.12345";
        assert_eq!(parse(&mut input).unwrap(), -1234567890.12345);
    }
}

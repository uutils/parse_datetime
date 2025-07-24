// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

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

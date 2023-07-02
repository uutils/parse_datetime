// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use nom::combinator::all_consuming;
use nom::{bytes::complete::take, character::complete, combinator::map_parser, Parser};

use crate::parse_items::PError;

macro_rules! fixed_number_impl {
        ($($t:ident),+) => {$(
            #[allow(dead_code)]
            pub fn $t<'i>(width: usize) -> impl Parser<&'i str, $t, PError<'i>> {
                move |input: &'i str| {
                    map_parser(take(width), all_consuming(complete::$t)).parse(input)
                }
            }
        )+};
    }

fixed_number_impl! { u8, u16, u32, u64, u128 }

#[cfg(test)]
mod tests {
    use crate::parse_items::{tests::ptest, PResult};

    use super::*;

    #[test]
    fn zero_width() {
        let result = u32(0).parse("1234");
        assert!(result.is_err(), "{:?}", result);
    }

    #[test]
    fn one_width() {
        assert_eq!(u32(1).parse("1234"), Ok(("234", 1)));
    }

    #[test]
    fn does_not_fit_type() {
        let result = u8(4).parse("1234");
        assert!(result.is_err(), "{:?}", result);
    }

    #[test]
    fn does_not_fit_negative() {
        let result = u8(3).parse("-123");
        assert!(result.is_err(), "{:?}", result);
    }

    #[test]
    fn input_too_short() {
        let result = u32(6).parse("1234");
        assert!(result.is_err(), "{:?}", result);
    }

    #[test]
    fn three() {
        assert_eq!(u32(3).parse("123abc"), Ok(("abc", 123)));
    }

    #[test]
    fn leading_zeroes() {
        assert_eq!(u32(3).parse("00123"), Ok(("23", 1)));
    }

    #[test]
    fn non_digits() {
        let result = u32(4).parse("123abc");
        assert!(result.is_err(), "{:?}", result);
    }
}

// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Module to parser relative time strings.
//!
//! Grammar definition:
//!
//! ```ebnf
//! ordinal = "last" | "this" | "next"
//!         | "first" | "third" | "fourth" | "fifth"
//!         | "sixth" | "seventh" | "eighth" | "ninth"
//!         | "tenth" | "eleventh" | "twelfth" ;
//!
//! integer = [ sign ] , digit , { digit } ;
//!
//! sign = { ("+" | "-") , { whitespace } } ;
//!
//! digit = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" ;
//! ```

use nom::{
    bytes::complete::take_while1,
    character::complete::{digit1, multispace0, one_of},
    combinator::{map_res, opt},
    multi::fold_many1,
    sequence::terminated,
    IResult, Parser,
};

use super::find_in_pairs;

const ORDINALS: &[(&str, i64)] = &[
    ("last", -1),
    ("this", 0),
    ("next", 1),
    ("first", 1),
    // Unfortunately we can't use "second" as ordinal, the keyword is overloaded
    ("third", 3),
    ("fourth", 4),
    ("fifth", 5),
    ("sixth", 6),
    ("seventh", 7),
    ("eighth", 8),
    ("ninth", 9),
    ("tenth", 10),
    ("eleventh", 11),
    ("twelfth", 12),
];

pub(super) fn ordinal(input: &str) -> IResult<&str, i64> {
    map_res(take_while1(|c: char| c.is_alphabetic()), |s: &str| {
        find_in_pairs(ORDINALS, s).ok_or("unknown ordinal")
    })
    .parse(input)
}

pub(super) fn integer(input: &str) -> IResult<&str, i64> {
    let (rest, sign) = opt(sign).parse(input)?;
    let (rest, num) = map_res(digit1, str::parse::<i64>).parse(rest)?;
    if sign == Some('-') {
        Ok((rest, -num))
    } else {
        Ok((rest, num))
    }
}

/// Parses a sign (either + or -) from the input string. The input string must
/// start with a sign character followed by arbitrary number of interleaving
/// sign characters and whitespace characters. All but the last sign character
/// is ignored, and the last sign character is returned as the result. This
/// quirky behavior is to stay consistent with GNU date.
pub(super) fn sign(input: &str) -> IResult<&str, char> {
    fold_many1(
        terminated(one_of("+-"), multispace0),
        || '+',
        |acc, c| if "+-".contains(c) { c } else { acc },
    )
    .parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ordinal() {
        assert!(ordinal("").is_err());
        assert!(ordinal("invalid").is_err());
        assert!(ordinal(" last").is_err());

        assert_eq!(ordinal("last"), Ok(("", -1)));
        assert_eq!(ordinal("this"), Ok(("", 0)));
        assert_eq!(ordinal("next"), Ok(("", 1)));
        assert_eq!(ordinal("first"), Ok(("", 1)));
        assert_eq!(ordinal("third"), Ok(("", 3)));
        assert_eq!(ordinal("fourth"), Ok(("", 4)));
        assert_eq!(ordinal("fifth"), Ok(("", 5)));
        assert_eq!(ordinal("sixth"), Ok(("", 6)));
        assert_eq!(ordinal("seventh"), Ok(("", 7)));
        assert_eq!(ordinal("eighth"), Ok(("", 8)));
        assert_eq!(ordinal("ninth"), Ok(("", 9)));
        assert_eq!(ordinal("tenth"), Ok(("", 10)));
        assert_eq!(ordinal("eleventh"), Ok(("", 11)));
        assert_eq!(ordinal("twelfth"), Ok(("", 12)));

        // Boundary
        assert_eq!(ordinal("last123"), Ok(("123", -1)));
        assert_eq!(ordinal("last abc"), Ok((" abc", -1)));
        assert!(ordinal("lastabc").is_err());

        // Case insensitive
        assert_eq!(ordinal("THIS"), Ok(("", 0)));
        assert_eq!(ordinal("This"), Ok(("", 0)));
    }

    #[test]
    fn test_integer() {
        assert!(integer("").is_err());
        assert!(integer("invalid").is_err());
        assert!(integer(" 123").is_err());

        assert_eq!(integer("123"), Ok(("", 123)));
        assert_eq!(integer("+123"), Ok(("", 123)));
        assert_eq!(integer("- 123"), Ok(("", -123)));

        // Boundary
        assert_eq!(integer("- 123abc"), Ok(("abc", -123)));
        assert_eq!(integer("- +- 123abc"), Ok(("abc", -123)));
    }

    #[test]
    fn test_sign() {
        assert!(sign("").is_err());
        assert!(sign("invalid").is_err());
        assert!(sign(" +").is_err());

        assert_eq!(sign("+"), Ok(("", '+')));
        assert_eq!(sign("-"), Ok(("", '-')));
        assert_eq!(sign("- + - "), Ok(("", '-')));

        // Boundary
        assert_eq!(sign("- + - abc"), Ok(("abc", '-')));
    }
}

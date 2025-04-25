// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Module to parser timestamp strings.
//!
//! Grammar definition:
//!
//! ```ebnf
//! timestamp = "@" seconds ;
//!
//! seconds = [ sign ] , { digit } , [ ("." | ",") , { digit } ] ;
//! ```

use nom::{
    bytes::complete::tag,
    character::complete::{digit1, one_of},
    combinator::{all_consuming, map_res, opt, recognize},
    sequence::preceded,
    IResult, Parser,
};

use super::primitive::sign;

pub(super) fn timestamp(input: &str) -> IResult<&str, f64> {
    all_consuming(preceded(tag("@"), seconds)).parse(input)
}

fn seconds(input: &str) -> IResult<&str, f64> {
    let (rest, sign) = opt(sign).parse(input)?;
    let (rest, num) = map_res(
        recognize((digit1, opt((one_of(".,"), digit1)))),
        |s: &str| {
            s.replace(",", ".")
                .parse::<f64>()
                .map_err(|_| "invalid seconds")
        },
    )
    .parse(rest)?;

    let num = if sign == Some('-') { -num } else { num };
    Ok((rest, num))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp() {
        assert!(timestamp("invalid").is_err());

        assert_eq!(timestamp("@-1234.567"), Ok(("", -1234.567)));
        assert_eq!(timestamp("@1234,567"), Ok(("", 1234.567)));
        assert_eq!(timestamp("@- 1234.567"), Ok(("", -1234.567)));
        assert_eq!(timestamp("@-+- 1234,567"), Ok(("", -1234.567)));
        assert_eq!(timestamp("@1234"), Ok(("", 1234.0)));
        assert_eq!(timestamp("@-1234"), Ok(("", -1234.0)));
    }

    #[test]
    fn test_seconds() {
        assert!(seconds("").is_err());
        assert!(seconds("invalid").is_err());

        assert_eq!(seconds("-1234.567"), Ok(("", -1234.567)));
        assert_eq!(seconds("1234,567"), Ok(("", 1234.567)));
        assert_eq!(seconds("- 1234.567"), Ok(("", -1234.567)));
        assert_eq!(seconds("-+- 1234,567"), Ok(("", -1234.567)));
        assert_eq!(seconds("1234"), Ok(("", 1234.0)));
        assert_eq!(seconds("-1234"), Ok(("", -1234.0)));
        assert_eq!(seconds("1234.567abc"), Ok(("abc", 1234.567)));
    }
}

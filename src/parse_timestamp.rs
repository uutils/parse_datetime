// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use core::fmt;
use std::error::Error;
use std::fmt::Display;
use std::num::ParseIntError;

use nom::branch::alt;
use nom::character::complete::{char, digit1};
use nom::combinator::all_consuming;
use nom::multi::fold_many0;
use nom::sequence::preceded;
use nom::sequence::tuple;
use nom::{self, IResult};

#[derive(Debug, PartialEq)]
pub enum ParseTimestampError {
    InvalidNumber(ParseIntError),
    InvalidInput,
}

impl Display for ParseTimestampError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput => {
                write!(f, "Invalid input string: cannot be parsed as a timestamp")
            }
            Self::InvalidNumber(err) => {
                write!(f, "Invalid timestamp number: {err}")
            }
        }
    }
}

impl Error for ParseTimestampError {}

// TODO is this necessary
impl From<ParseIntError> for ParseTimestampError {
    fn from(err: ParseIntError) -> Self {
        Self::InvalidNumber(err)
    }
}

type NomError<'a> = nom::Err<nom::error::Error<&'a str>>;

impl<'a> From<NomError<'a>> for ParseTimestampError {
    fn from(_err: NomError<'a>) -> Self {
        Self::InvalidInput
    }
}

pub(crate) fn parse_timestamp(s: &str) -> Result<i64, ParseTimestampError> {
    let s = s.trim().to_lowercase();
    let s = s.as_str();

    let res: IResult<&str, (char, &str)> = all_consuming(preceded(
        char('@'),
        tuple((
            // Note: to stay compatible with gnu date this code allows
            // multiple + and - and only considers the last one
            fold_many0(
                // parse either + or -
                alt((char('+'), char('-'))),
                // start with a +
                || '+',
                // whatever we get (+ or -), update the accumulator to that value
                |_, c| c,
            ),
            digit1,
        )),
    ))(s);

    let (_, (sign, number_str)) = res?;

    let mut number = number_str.parse::<i64>()?;

    if sign == '-' {
        number *= -1;
    }

    Ok(number)
}

#[cfg(test)]
mod tests {

    use crate::parse_timestamp::parse_timestamp;

    #[test]
    fn test_valid_timestamp() {
        assert_eq!(parse_timestamp("@1234"), Ok(1234));
        assert_eq!(parse_timestamp("@99999"), Ok(99999));
        assert_eq!(parse_timestamp("@-4"), Ok(-4));
        assert_eq!(parse_timestamp("@-99999"), Ok(-99999));
        assert_eq!(parse_timestamp("@+4"), Ok(4));
        assert_eq!(parse_timestamp("@0"), Ok(0));

        // gnu date accepts numbers signs and uses the last sign
        assert_eq!(parse_timestamp("@---+12"), Ok(12));
        assert_eq!(parse_timestamp("@+++-12"), Ok(-12));
        assert_eq!(parse_timestamp("@+----+12"), Ok(12));
        assert_eq!(parse_timestamp("@++++-123"), Ok(-123));
    }

    #[test]
    fn test_invalid_timestamp() {
        assert!(parse_timestamp("@").is_err());
        assert!(parse_timestamp("@+--+").is_err());
        assert!(parse_timestamp("@+1ab2").is_err());
    }
}

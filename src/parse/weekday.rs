// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Module to parser weekday strings.
//!
//! Grammar definition:
//!
//! ```ebnf
//! weekday = (integer | ordinal) , day | day , [ "," ] ;
//!
//! day = "sunday" | "sun"
//!     | "monday" | "mon"
//!     | "tuesday" | "tues" | "tue"
//!     | "wednesday" | "wednes" | "wed"
//!     | "thursday" | "thurs" | "thur" | "thu"
//!     | "friday" | "fri"
//!     | "saturday" | "sat" ;
//! ```

use chrono::Weekday;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::multispace0,
    combinator::{all_consuming, map_res, opt},
    sequence::{preceded, terminated},
    IResult, Parser,
};

use super::{
    find_in_pairs,
    primitive::{integer, ordinal},
};

const DAYS: &[(&str, Weekday)] = &[
    ("sunday", Weekday::Sun),
    ("sun", Weekday::Sun),
    ("monday", Weekday::Mon),
    ("mon", Weekday::Mon),
    ("tuesday", Weekday::Tue),
    ("tues", Weekday::Tue),
    ("tue", Weekday::Tue),
    ("wednesday", Weekday::Wed),
    ("wednes", Weekday::Wed),
    ("wed", Weekday::Wed),
    ("thursday", Weekday::Thu),
    ("thurs", Weekday::Thu),
    ("thur", Weekday::Thu),
    ("thu", Weekday::Thu),
    ("friday", Weekday::Fri),
    ("fri", Weekday::Fri),
    ("saturday", Weekday::Sat),
    ("sat", Weekday::Sat),
];

#[derive(Debug, PartialEq)]
pub(crate) struct WeekdayItem {
    pub weekday: Weekday,
    pub ordinal: Option<i64>,
}

pub(super) fn weekday(input: &str) -> IResult<&str, WeekdayItem> {
    all_consuming(alt((ordinal_day, day_comma))).parse(input)
}

fn ordinal_day(input: &str) -> IResult<&str, WeekdayItem> {
    map_res(
        (alt((ordinal, integer)), preceded(multispace0, day)),
        |(ordinal, weekday): (i64, Weekday)| {
            Ok::<WeekdayItem, &str>(WeekdayItem {
                weekday,
                ordinal: Some(ordinal),
            })
        },
    )
    .parse(input)
}

fn day_comma(input: &str) -> IResult<&str, WeekdayItem> {
    map_res(
        terminated(day, opt(preceded(multispace0, tag(",")))),
        |s: Weekday| {
            Ok::<WeekdayItem, &str>(WeekdayItem {
                weekday: s,
                ordinal: None,
            })
        },
    )
    .parse(input)
}

fn day(input: &str) -> IResult<&str, Weekday> {
    map_res(take_while1(|c: char| c.is_alphabetic()), |s: &str| {
        find_in_pairs(DAYS, s).ok_or("unknown weekday")
    })
    .parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weekday() {
        assert!(weekday("").is_err());
        assert!(weekday("invalid").is_err());
        assert!(weekday(" sun").is_err());

        assert_eq!(
            weekday("last sunday"),
            Ok((
                "",
                WeekdayItem {
                    weekday: Weekday::Sun,
                    ordinal: Some(-1)
                }
            ))
        );
        assert_eq!(
            weekday("sunday ,"),
            Ok((
                "",
                WeekdayItem {
                    weekday: Weekday::Sun,
                    ordinal: None
                }
            ))
        );
        assert!(weekday("next sunday,").is_err());
    }

    #[test]
    fn test_ordinal_day() {
        assert!(ordinal_day("").is_err());
        assert!(ordinal_day("invalid").is_err());
        assert!(ordinal_day(" sun").is_err());

        assert_eq!(
            ordinal_day("last sunday"),
            Ok((
                "",
                WeekdayItem {
                    weekday: Weekday::Sun,
                    ordinal: Some(-1)
                }
            ))
        );
        assert_eq!(
            ordinal_day("2 sun"),
            Ok((
                "",
                WeekdayItem {
                    weekday: Weekday::Sun,
                    ordinal: Some(2)
                }
            ))
        );
        assert_eq!(
            ordinal_day("2sun"),
            Ok((
                "",
                WeekdayItem {
                    weekday: Weekday::Sun,
                    ordinal: Some(2)
                }
            ))
        );
        assert!(ordinal_day("nextsun").is_err());
    }

    #[test]
    fn test_day_comma() {
        assert!(day_comma("").is_err());
        assert!(day_comma("invalid").is_err());
        assert!(day_comma(" sun").is_err());

        assert_eq!(
            day_comma("sunday"),
            Ok((
                "",
                WeekdayItem {
                    weekday: Weekday::Sun,
                    ordinal: None
                }
            ))
        );
        assert_eq!(
            day_comma("sun,"),
            Ok((
                "",
                WeekdayItem {
                    weekday: Weekday::Sun,
                    ordinal: None
                }
            ))
        );
        assert_eq!(
            day_comma("sun ,"),
            Ok((
                "",
                WeekdayItem {
                    weekday: Weekday::Sun,
                    ordinal: None
                }
            ))
        );
    }

    #[test]
    fn test_day() {
        assert!(day("").is_err());
        assert!(day("invalid").is_err());
        assert!(day(" sun").is_err());

        assert_eq!(day("sunday"), Ok(("", Weekday::Sun)));
        assert_eq!(day("sun"), Ok(("", Weekday::Sun)));
        assert_eq!(day("monday"), Ok(("", Weekday::Mon)));
        assert_eq!(day("mon"), Ok(("", Weekday::Mon)));
        assert_eq!(day("tuesday"), Ok(("", Weekday::Tue)));
        assert_eq!(day("tues"), Ok(("", Weekday::Tue)));
        assert_eq!(day("tue"), Ok(("", Weekday::Tue)));
        assert_eq!(day("wednesday"), Ok(("", Weekday::Wed)));
        assert_eq!(day("wednes"), Ok(("", Weekday::Wed)));
        assert_eq!(day("wed"), Ok(("", Weekday::Wed)));
        assert_eq!(day("thursday"), Ok(("", Weekday::Thu)));
        assert_eq!(day("thur"), Ok(("", Weekday::Thu)));
        assert_eq!(day("thu"), Ok(("", Weekday::Thu)));
        assert_eq!(day("friday"), Ok(("", Weekday::Fri)));
        assert_eq!(day("fri"), Ok(("", Weekday::Fri)));
        assert_eq!(day("saturday"), Ok(("", Weekday::Sat)));
        assert_eq!(day("sat"), Ok(("", Weekday::Sat)));

        // Boundary
        assert_eq!(day("sun123"), Ok(("123", Weekday::Sun)));
        assert_eq!(day("sun abc"), Ok((" abc", Weekday::Sun)));
        assert!(day("sunabc").is_err());

        // Case insensitive
        assert_eq!(day("MONDAY"), Ok(("", Weekday::Mon)));
        assert_eq!(day("Monday"), Ok(("", Weekday::Mon)));
    }
}

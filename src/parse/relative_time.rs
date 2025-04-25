// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Module to parser relative time strings.
//!
//! Grammar definition:
//!
//! ```ebnf
//! relative_times = relative_time , { ("," | "and") , relative_time } ;
//!
//! relative_time = displacement | day_shift ;
//!
//! displacement = (integer | ordinal) unit [ "ago" ] ;
//!
//! day_shift = "now" | "today" | "tomorrow" | "yesterday" ;
//!
//! unit = "seconds" | "second" | "secs" | "sec" | "s"
//!      | "minutes" | "minute" | "mins" | "min" | "m"
//!      | "hours" | "hour" | "h"
//!      | "days" | "day"
//!      | "weeks" | "week"
//!      | "fortnights" | "fortnight"
//!      | "months" | "month"
//!      | "years" | "year" ;
//!
//! ordinal = "last" | "this" | "next"
//!         | "first" | "third" | "fourth" | "fifth"
//!         | "sixth" | "seventh" | "eighth" | "ninth"
//!         | "tenth" | "eleventh" | "twelfth" ;
//!
//! integer = [ sign ] , digit , { digit } ;
//!
//! sign = { ("+" | "-") { whitespace } } ;
//!
//! digit = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" ;
//! ```

use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_while1},
    character::complete::{digit1, multispace0, multispace1, one_of},
    combinator::{all_consuming, map_res, opt},
    multi::{fold_many1, separated_list0},
    sequence::{preceded, terminated},
    IResult, Parser,
};

const TIME_UNITS: &[(&str, TimeUnit)] = &[
    ("seconds", TimeUnit::Second),
    ("second", TimeUnit::Second),
    ("secs", TimeUnit::Second),
    ("sec", TimeUnit::Second),
    ("s", TimeUnit::Second),
    ("minutes", TimeUnit::Minute),
    ("minute", TimeUnit::Minute),
    ("mins", TimeUnit::Minute),
    ("min", TimeUnit::Minute),
    ("m", TimeUnit::Minute),
    ("hours", TimeUnit::Hour),
    ("hour", TimeUnit::Hour),
    ("h", TimeUnit::Hour),
    ("days", TimeUnit::Day),
    ("day", TimeUnit::Day),
    ("weeks", TimeUnit::Week),
    ("week", TimeUnit::Week),
    ("fortnights", TimeUnit::Fortnight),
    ("fortnight", TimeUnit::Fortnight),
    ("months", TimeUnit::Month),
    ("month", TimeUnit::Month),
    ("years", TimeUnit::Year),
    ("year", TimeUnit::Year),
];

const DAY_SHIFTS: &[(&str, RelativeTime)] = &[
    ("now", RelativeTime::Now),
    ("today", RelativeTime::Today),
    ("tomorrow", RelativeTime::Tomorrow),
    ("yesterday", RelativeTime::Yesterday),
];

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

/// The `TimeUnit` enum represents the different time units that can be used in
/// relative time parsing.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TimeUnit {
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Fortnight,
    Month,
    Year,
}

/// The `RelativeTime` enum represents the different types of relative time. It
/// can be a specific time unit with displacement (like "2 hours") or a day shift
/// (like "today").
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RelativeTime {
    Now,
    Today,
    Tomorrow,
    Yesterday,
    Displacement { value: i64, unit: TimeUnit },
}

fn find_in_pairs<T: Clone>(pairs: &[(&str, T)], key: &str) -> Option<T> {
    pairs.iter().find_map(|(k, v)| {
        if k.eq_ignore_ascii_case(key) {
            Some(v.clone())
        } else {
            None
        }
    })
}

pub(super) fn relative_times(input: &str) -> IResult<&str, Vec<RelativeTime>> {
    all_consuming(separated_list0(
        alt((
            preceded(multispace0, terminated(tag(","), multispace0)),
            preceded(multispace1, terminated(tag_no_case("and"), multispace1)),
            multispace0,
        )),
        relative_time,
    ))
    .parse(input)
}

fn relative_time(input: &str) -> IResult<&str, RelativeTime> {
    alt((day_shift, displacement)).parse(input)
}

fn displacement(input: &str) -> IResult<&str, RelativeTime> {
    let (rest, (value, unit, ago)) = (
        opt(terminated(alt((ordinal, integer)), multispace0)),
        unit,
        opt(preceded(multispace1, ago)),
    )
        .parse(input)?;

    let mut value = value.unwrap_or(1);
    if ago.unwrap_or(false) {
        value = -value;
    }

    Ok((rest, RelativeTime::Displacement { value, unit }))
}

fn ago(input: &str) -> IResult<&str, bool> {
    map_res(take_while1(|c: char| c.is_alphabetic()), |s: &str| {
        if s.eq_ignore_ascii_case("ago") {
            Ok(true)
        } else {
            Err("not ago")
        }
    })
    .parse(input)
}

fn unit(input: &str) -> IResult<&str, TimeUnit> {
    map_res(take_while1(|c: char| c.is_alphabetic()), |s: &str| {
        find_in_pairs(TIME_UNITS, s).ok_or("unknown time unit")
    })
    .parse(input)
}

fn day_shift(input: &str) -> IResult<&str, RelativeTime> {
    map_res(take_while1(|c: char| c.is_alphabetic()), |s: &str| {
        find_in_pairs(DAY_SHIFTS, s).ok_or("unknown day shift")
    })
    .parse(input)
}

fn ordinal(input: &str) -> IResult<&str, i64> {
    map_res(take_while1(|c: char| c.is_alphabetic()), |s: &str| {
        find_in_pairs(ORDINALS, s).ok_or("unknown ordinal")
    })
    .parse(input)
}

fn integer(input: &str) -> IResult<&str, i64> {
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
fn sign(input: &str) -> IResult<&str, char> {
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
    fn test_relative_times() {
        assert!(relative_times(" ").is_err());
        assert!(relative_times("invalid").is_err());

        assert_eq!(relative_times(""), Ok(("", vec![])));

        assert_eq!(
            relative_times("second"),
            Ok((
                "",
                vec![RelativeTime::Displacement {
                    value: 1,
                    unit: TimeUnit::Second
                }]
            ))
        );
        assert_eq!(
            relative_times("2 minutes"),
            Ok((
                "",
                vec![RelativeTime::Displacement {
                    value: 2,
                    unit: TimeUnit::Minute
                }]
            ))
        );
        assert_eq!(
            relative_times("3 hours ago"),
            Ok((
                "",
                vec![RelativeTime::Displacement {
                    value: -3,
                    unit: TimeUnit::Hour
                }]
            ))
        );

        // Space separator
        assert_eq!(
            relative_times("today tomorrow"),
            Ok(("", vec![RelativeTime::Today, RelativeTime::Tomorrow]))
        );

        // Comma separator
        assert_eq!(
            relative_times("today, tomorrow"),
            Ok(("", vec![RelativeTime::Today, RelativeTime::Tomorrow]))
        );
        assert_eq!(
            relative_times("today ,tomorrow"),
            Ok(("", vec![RelativeTime::Today, RelativeTime::Tomorrow]))
        );
        assert_eq!(
            relative_times("today , tomorrow"),
            Ok(("", vec![RelativeTime::Today, RelativeTime::Tomorrow]))
        );

        // "and" separator
        assert_eq!(
            relative_times("today and tomorrow"),
            Ok(("", vec![RelativeTime::Today, RelativeTime::Tomorrow]))
        );

        // Mixed separator
        assert_eq!(
            relative_times("yesterday, today and tomorrow"),
            Ok((
                "",
                vec![
                    RelativeTime::Yesterday,
                    RelativeTime::Today,
                    RelativeTime::Tomorrow
                ]
            ))
        );

        // Boundary
        assert_eq!(
            relative_times("1week2months-3years"),
            Ok((
                "",
                vec![
                    RelativeTime::Displacement {
                        value: 1,
                        unit: TimeUnit::Week
                    },
                    RelativeTime::Displacement {
                        value: 2,
                        unit: TimeUnit::Month
                    },
                    RelativeTime::Displacement {
                        value: -3,
                        unit: TimeUnit::Year
                    }
                ]
            ))
        );
        assert!(relative_times("1week2months-3years123").is_err());
        assert!(relative_times("1week2months-3yearsabc").is_err());
    }

    #[test]
    fn test_relative_time() {
        assert!(relative_time("").is_err());
        assert!(relative_time("invalid").is_err());

        assert_eq!(
            relative_time("second"),
            Ok((
                "",
                RelativeTime::Displacement {
                    value: 1,
                    unit: TimeUnit::Second
                }
            ))
        );
        assert_eq!(
            relative_time("2 minutes"),
            Ok((
                "",
                RelativeTime::Displacement {
                    value: 2,
                    unit: TimeUnit::Minute
                }
            ))
        );
        assert_eq!(
            relative_time("3 hours ago"),
            Ok((
                "",
                RelativeTime::Displacement {
                    value: -3,
                    unit: TimeUnit::Hour
                }
            ))
        );
        assert_eq!(
            relative_time("last day"),
            Ok((
                "",
                RelativeTime::Displacement {
                    value: -1,
                    unit: TimeUnit::Day
                }
            ))
        );
        assert_eq!(
            relative_time("twelfth week"),
            Ok((
                "",
                RelativeTime::Displacement {
                    value: 12,
                    unit: TimeUnit::Week
                }
            ))
        );
        assert_eq!(relative_time("now"), Ok(("", RelativeTime::Now)));
        assert_eq!(relative_time("today"), Ok(("", RelativeTime::Today)));
    }

    #[test]
    fn test_displacement() {
        assert!(displacement("").is_err());
        assert!(displacement("invalid").is_err());

        assert_eq!(
            displacement("second"),
            Ok((
                "",
                RelativeTime::Displacement {
                    value: 1,
                    unit: TimeUnit::Second
                }
            ))
        );
        assert_eq!(
            displacement("2 minutes"),
            Ok((
                "",
                RelativeTime::Displacement {
                    value: 2,
                    unit: TimeUnit::Minute
                }
            ))
        );
        assert_eq!(
            displacement("3 hours ago"),
            Ok((
                "",
                RelativeTime::Displacement {
                    value: -3,
                    unit: TimeUnit::Hour
                }
            ))
        );
        assert_eq!(
            displacement("last day"),
            Ok((
                "",
                RelativeTime::Displacement {
                    value: -1,
                    unit: TimeUnit::Day
                }
            ))
        );
        assert_eq!(
            displacement("twelfth week"),
            Ok((
                "",
                RelativeTime::Displacement {
                    value: 12,
                    unit: TimeUnit::Week
                }
            ))
        );

        // Boundary
        assert_eq!(
            displacement("3 hours123"),
            Ok((
                "123",
                RelativeTime::Displacement {
                    value: 3,
                    unit: TimeUnit::Hour
                }
            ))
        );
        assert!(displacement("3 hoursabc").is_err());
        assert_eq!(
            displacement("3 hours ago123"),
            Ok((
                "123",
                RelativeTime::Displacement {
                    value: -3,
                    unit: TimeUnit::Hour
                }
            ))
        );
        assert_eq!(
            displacement("3 hours ago abc"),
            Ok((
                " abc",
                RelativeTime::Displacement {
                    value: -3,
                    unit: TimeUnit::Hour
                }
            ))
        );
        assert_eq!(
            displacement("3 hours agoabc"),
            Ok((
                " agoabc",
                RelativeTime::Displacement {
                    value: 3,
                    unit: TimeUnit::Hour
                }
            ))
        );
    }

    #[test]
    fn test_ago() {
        assert!(ago("").is_err());
        assert!(ago("invalid").is_err());

        assert_eq!(ago("ago"), Ok(("", true)));

        // Boundary
        assert_eq!(ago("ago123"), Ok(("123", true)));
        assert_eq!(ago("ago abc"), Ok((" abc", true)));
        assert!(ago("agoabc").is_err());
    }

    #[test]
    fn test_unit() {
        assert!(day_shift("").is_err());
        assert!(unit("invalid").is_err());
        assert!(unit(" second").is_err());

        assert_eq!(unit("seconds"), Ok(("", TimeUnit::Second)));
        assert_eq!(unit("second"), Ok(("", TimeUnit::Second)));
        assert_eq!(unit("secs"), Ok(("", TimeUnit::Second)));
        assert_eq!(unit("sec"), Ok(("", TimeUnit::Second)));
        assert_eq!(unit("s"), Ok(("", TimeUnit::Second)));
        assert_eq!(unit("minutes"), Ok(("", TimeUnit::Minute)));
        assert_eq!(unit("minute"), Ok(("", TimeUnit::Minute)));
        assert_eq!(unit("mins"), Ok(("", TimeUnit::Minute)));
        assert_eq!(unit("min"), Ok(("", TimeUnit::Minute)));
        assert_eq!(unit("m"), Ok(("", TimeUnit::Minute)));
        assert_eq!(unit("hours"), Ok(("", TimeUnit::Hour)));
        assert_eq!(unit("hour"), Ok(("", TimeUnit::Hour)));
        assert_eq!(unit("days"), Ok(("", TimeUnit::Day)));
        assert_eq!(unit("day"), Ok(("", TimeUnit::Day)));
        assert_eq!(unit("weeks"), Ok(("", TimeUnit::Week)));
        assert_eq!(unit("week"), Ok(("", TimeUnit::Week)));
        assert_eq!(unit("fortnights"), Ok(("", TimeUnit::Fortnight)));
        assert_eq!(unit("fortnight"), Ok(("", TimeUnit::Fortnight)));
        assert_eq!(unit("months"), Ok(("", TimeUnit::Month)));
        assert_eq!(unit("month"), Ok(("", TimeUnit::Month)));
        assert_eq!(unit("years"), Ok(("", TimeUnit::Year)));
        assert_eq!(unit("year"), Ok(("", TimeUnit::Year)));

        // Boundary
        assert_eq!(unit("second123"), Ok(("123", TimeUnit::Second)));
        assert_eq!(unit("second abc"), Ok((" abc", TimeUnit::Second)));
        assert!(unit("secondabc").is_err());

        // Case insensitive
        assert_eq!(unit("SECOND"), Ok(("", TimeUnit::Second)));
        assert_eq!(unit("Second"), Ok(("", TimeUnit::Second)));
    }

    #[test]
    fn test_day_shift() {
        assert!(day_shift("").is_err());
        assert!(day_shift("invalid").is_err());
        assert!(day_shift(" now").is_err());

        assert_eq!(day_shift("now"), Ok(("", RelativeTime::Now)));
        assert_eq!(day_shift("today"), Ok(("", RelativeTime::Today)));
        assert_eq!(day_shift("tomorrow"), Ok(("", RelativeTime::Tomorrow)));
        assert_eq!(day_shift("yesterday"), Ok(("", RelativeTime::Yesterday)));

        // Boundary
        assert_eq!(day_shift("now123"), Ok(("123", RelativeTime::Now)));
        assert_eq!(day_shift("now abc"), Ok((" abc", RelativeTime::Now)));
        assert!(day_shift("nowabc").is_err());

        // Case insensitive
        assert_eq!(day_shift("NOW"), Ok(("", RelativeTime::Now)));
        assert_eq!(day_shift("Now"), Ok(("", RelativeTime::Now)));
    }

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

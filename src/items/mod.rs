// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore multispace0

//! From the GNU docs:
//!
//! > A date is a string, possibly empty, containing many items separated by
//! > whitespace. The whitespace may be omitted when no ambiguity arises. The
//! > empty string means the beginning of today (i.e., midnight). Order of the
//! > items is immaterial. A date string may contain many flavors of items:
//! >  - calendar date items
//! >  - time of day items
//! >  - time zone items
//! >  - combined date and time of day items
//! >  - day of the week items
//! >  - relative items
//! >  - pure numbers.
//!
//! We put all of those in separate modules:
//!  - [`date`]
//!  - [`time`]
//!  - [`time_zone`]
//!  - [`combined`]
//!  - [`weekday`]
//!  - [`relative`]
//!  - [`number]

#![allow(deprecated)]
mod combined;
mod date;
mod ordinal;
mod relative;
mod time;
mod weekday;
mod epoch {
    use winnow::{combinator::preceded, ModalResult, Parser};

    use super::{dec_int, s};
    pub fn parse(input: &mut &str) -> ModalResult<i32> {
        s(preceded("@", dec_int)).parse_next(input)
    }
}
mod timezone {
    use super::time;
    use winnow::ModalResult;

    pub(crate) fn parse(input: &mut &str) -> ModalResult<time::Offset> {
        time::timezone(input)
    }
}

use chrono::NaiveDate;
use chrono::{DateTime, Datelike, FixedOffset, TimeZone, Timelike};

use winnow::error::{StrContext, StrContextValue};
use winnow::{
    ascii::{digit1, multispace0},
    combinator::{alt, delimited, not, opt, peek, preceded, repeat, separated, trace},
    error::{ContextError, ErrMode, ParserError},
    stream::AsChar,
    token::{none_of, one_of, take_while},
    ModalResult, Parser,
};

use crate::ParseDateTimeError;

#[derive(PartialEq, Debug)]
pub enum Item {
    Timestamp(i32),
    Year(u32),
    DateTime(combined::DateTime),
    Date(date::Date),
    Time(time::Time),
    Weekday(weekday::Weekday),
    Relative(relative::Relative),
    TimeZone(time::Offset),
}

/// Allow spaces and comments before a parser
///
/// Every token parser should be wrapped in this to allow spaces and comments.
/// It is only preceding, because that allows us to check mandatory whitespace
/// after running the parser.
fn s<'a, O, E>(p: impl Parser<&'a str, O, E>) -> impl Parser<&'a str, O, E>
where
    E: ParserError<&'a str>,
{
    preceded(space, p)
}

/// Parse the space in-between tokens
///
/// You probably want to use the [`s`] combinator instead.
fn space<'a, E>(input: &mut &'a str) -> winnow::Result<(), E>
where
    E: ParserError<&'a str>,
{
    separated(0.., multispace0, alt((comment, ignored_hyphen_or_plus))).parse_next(input)
}

/// A hyphen or plus is ignored when it is not followed by a digit
///
/// This includes being followed by a comment! Compare these inputs:
/// ```txt
/// - 12 weeks
/// - (comment) 12 weeks
/// ```
/// The last comment should be ignored.
///
/// The plus is undocumented, but it seems to be ignored.
fn ignored_hyphen_or_plus<'a, E>(input: &mut &'a str) -> winnow::Result<(), E>
where
    E: ParserError<&'a str>,
{
    (
        alt(('-', '+')),
        multispace0,
        peek(not(take_while(1, AsChar::is_dec_digit))),
    )
        .void()
        .parse_next(input)
}

/// Parse a comment
///
/// A comment is given between parentheses, which must be balanced. Any other
/// tokens can be within the comment.
fn comment<'a, E>(input: &mut &'a str) -> winnow::Result<(), E>
where
    E: ParserError<&'a str>,
{
    delimited(
        '(',
        repeat(0.., alt((none_of(['(', ')']).void(), comment))),
        ')',
    )
    .parse_next(input)
}

/// Parse a signed decimal integer.
///
/// Rationale for not using `winnow::ascii::dec_int`: When upgrading winnow from
/// 0.5 to 0.7, we discovered that `winnow::ascii::dec_int` now accepts only the
/// following two forms:
///
/// - 0
/// - [+-][1-9][0-9]*
///
/// Inputs like [+-]0[0-9]* (e.g., `+012`) are therefore rejected. We provide a
/// custom implementation to support such zero-prefixed integers.
fn dec_int<'a, E>(input: &mut &'a str) -> winnow::Result<i32, E>
where
    E: ParserError<&'a str>,
{
    (opt(one_of(['+', '-'])), digit1)
        .void()
        .take()
        .verify_map(|s: &str| s.parse().ok())
        .parse_next(input)
}

/// Parse an unsigned decimal integer.
///
/// See the rationale for `dec_int` for why we don't use
/// `winnow::ascii::dec_uint`.
fn dec_uint<'a, E>(input: &mut &'a str) -> winnow::Result<u32, E>
where
    E: ParserError<&'a str>,
{
    digit1
        .void()
        .take()
        .verify_map(|s: &str| s.parse().ok())
        .parse_next(input)
}

// Parse an item
pub fn parse_one(input: &mut &str) -> ModalResult<Item> {
    trace(
        "parse_one",
        alt((
            combined::parse.map(Item::DateTime),
            date::parse.map(Item::Date),
            time::parse.map(Item::Time),
            relative::parse.map(Item::Relative),
            weekday::parse.map(Item::Weekday),
            epoch::parse.map(Item::Timestamp),
            timezone::parse.map(Item::TimeZone),
            date::year.map(Item::Year),
        )),
    )
    .parse_next(input)
}

pub fn parse(input: &mut &str) -> ModalResult<Vec<Item>> {
    let mut items = Vec::new();
    let mut date_seen = false;
    let mut time_seen = false;
    let mut year_seen = false;
    let mut tz_seen = false;

    loop {
        match parse_one.parse_next(input) {
            Ok(item) => {
                match item {
                    Item::DateTime(ref dt) => {
                        if date_seen || time_seen {
                            let mut ctx_err = ContextError::new();
                            ctx_err.push(StrContext::Expected(
                                winnow::error::StrContextValue::Description(
                                    "date or time cannot appear more than once",
                                ),
                            ));
                            return Err(ErrMode::Backtrack(ctx_err));
                        }

                        date_seen = true;
                        time_seen = true;
                        if dt.date.year.is_some() {
                            year_seen = true;
                        }
                    }
                    Item::Date(ref d) => {
                        if date_seen {
                            let mut ctx_err = ContextError::new();
                            ctx_err.push(StrContext::Expected(StrContextValue::Description(
                                "date cannot appear more than once",
                            )));
                            return Err(ErrMode::Backtrack(ctx_err));
                        }

                        date_seen = true;
                        if d.year.is_some() {
                            year_seen = true;
                        }
                    }
                    Item::Time(_) => {
                        if time_seen {
                            let mut ctx_err = ContextError::new();
                            ctx_err.push(StrContext::Expected(StrContextValue::Description(
                                "time cannot appear more than once",
                            )));
                            return Err(ErrMode::Backtrack(ctx_err));
                        }
                        time_seen = true;
                    }
                    Item::Year(_) => {
                        if year_seen {
                            let mut ctx_err = ContextError::new();
                            ctx_err.push(StrContext::Expected(StrContextValue::Description(
                                "year cannot appear more than once",
                            )));
                            return Err(ErrMode::Backtrack(ctx_err));
                        }
                        year_seen = true;
                    }
                    Item::TimeZone(_) => {
                        if tz_seen {
                            let mut ctx_err = ContextError::new();
                            ctx_err.push(StrContext::Expected(StrContextValue::Description(
                                "timezone cannot appear more than once",
                            )));
                            return Err(ErrMode::Backtrack(ctx_err));
                        }
                        tz_seen = true;
                    }
                    _ => {}
                }
                items.push(item);
            }
            Err(ErrMode::Backtrack(_)) => break,
            Err(e) => return Err(e),
        }
    }

    space.parse_next(input)?;
    if !input.is_empty() {
        return Err(ErrMode::Backtrack(ContextError::new()));
    }

    Ok(items)
}

fn new_date(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    offset: FixedOffset,
) -> Option<DateTime<FixedOffset>> {
    let newdate = NaiveDate::from_ymd_opt(year, month, day)
        .and_then(|naive| naive.and_hms_opt(hour, minute, second))?;

    Some(DateTime::<FixedOffset>::from_local(newdate, offset))
}

/// Restores year, month, day, etc after applying the timezone
/// returns None if timezone overflows the date
fn with_timezone_restore(
    offset: time::Offset,
    at: DateTime<FixedOffset>,
) -> Option<DateTime<FixedOffset>> {
    let offset: FixedOffset = chrono::FixedOffset::from(offset);
    let copy = at;
    let x = at
        .with_timezone(&offset)
        .with_day(copy.day())?
        .with_month(copy.month())?
        .with_year(copy.year())?
        .with_hour(copy.hour())?
        .with_minute(copy.minute())?
        .with_second(copy.second())?;
    Some(x)
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    NaiveDate::from_ymd_opt(year, month + 1, 1)
        .unwrap_or(NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap())
        .pred_opt()
        .unwrap()
        .day()
}

fn at_date_inner(date: Vec<Item>, mut d: DateTime<FixedOffset>) -> Option<DateTime<FixedOffset>> {
    d = d.with_hour(0).unwrap();
    d = d.with_minute(0).unwrap();
    d = d.with_second(0).unwrap();
    d = d.with_nanosecond(0).unwrap();

    for item in date {
        match item {
            Item::Timestamp(ts) => {
                d = chrono::Utc
                    .timestamp_opt(ts.into(), 0)
                    .unwrap()
                    .with_timezone(&d.timezone())
            }
            Item::Date(date::Date { day, month, year }) => {
                d = new_date(
                    year.map(|x| x as i32).unwrap_or(d.year()),
                    month,
                    day,
                    d.hour(),
                    d.minute(),
                    d.second(),
                    *d.offset(),
                )?;
            }
            Item::DateTime(combined::DateTime {
                date: date::Date { day, month, year },
                time:
                    time::Time {
                        hour,
                        minute,
                        second,
                        offset,
                    },
                ..
            }) => {
                let offset = offset.map(chrono::FixedOffset::from).unwrap_or(*d.offset());

                d = new_date(
                    year.map(|x| x as i32).unwrap_or(d.year()),
                    month,
                    day,
                    hour,
                    minute,
                    second as u32,
                    offset,
                )?;
            }
            Item::Year(year) => d = d.with_year(year as i32).unwrap_or(d),
            Item::Time(time::Time {
                hour,
                minute,
                second,
                offset,
            }) => {
                let offset = offset.map(chrono::FixedOffset::from).unwrap_or(*d.offset());
                d = new_date(
                    d.year(),
                    d.month(),
                    d.day(),
                    hour,
                    minute,
                    second as u32,
                    offset,
                )?;
            }
            Item::Weekday(weekday::Weekday {
                offset: _, // TODO: use the offset
                day,
            }) => {
                let mut beginning_of_day = d
                    .with_hour(0)
                    .unwrap()
                    .with_minute(0)
                    .unwrap()
                    .with_second(0)
                    .unwrap()
                    .with_nanosecond(0)
                    .unwrap();
                let day = day.into();

                while beginning_of_day.weekday() != day {
                    beginning_of_day += chrono::Duration::days(1);
                }

                d = beginning_of_day
            }
            Item::Relative(relative::Relative::Years(x)) => {
                d = d.with_year(d.year() + x)?;
            }
            Item::Relative(relative::Relative::Months(x)) => {
                // *NOTE* This is done in this way to conform to
                // GNU behavior.
                let days = last_day_of_month(d.year(), d.month());
                if x >= 0 {
                    d += d
                        .date_naive()
                        .checked_add_days(chrono::Days::new((days * x as u32) as u64))?
                        .signed_duration_since(d.date_naive());
                } else {
                    d += d
                        .date_naive()
                        .checked_sub_days(chrono::Days::new((days * -x as u32) as u64))?
                        .signed_duration_since(d.date_naive());
                }
            }
            Item::Relative(relative::Relative::Days(x)) => d += chrono::Duration::days(x.into()),
            Item::Relative(relative::Relative::Hours(x)) => d += chrono::Duration::hours(x.into()),
            Item::Relative(relative::Relative::Minutes(x)) => {
                d += chrono::Duration::minutes(x.into());
            }
            // Seconds are special because they can be given as a float
            Item::Relative(relative::Relative::Seconds(x)) => {
                d += chrono::Duration::seconds(x as i64);
            }
            Item::TimeZone(offset) => {
                d = with_timezone_restore(offset, d)?;
            }
        }
    }

    Some(d)
}

pub(crate) fn at_date(
    date: Vec<Item>,
    d: DateTime<FixedOffset>,
) -> Result<DateTime<FixedOffset>, ParseDateTimeError> {
    at_date_inner(date, d).ok_or(ParseDateTimeError::InvalidInput)
}

pub(crate) fn at_local(date: Vec<Item>) -> Result<DateTime<FixedOffset>, ParseDateTimeError> {
    at_date(date, chrono::Local::now().into())
}

#[cfg(test)]
mod tests {
    use super::{at_date, date::Date, parse, time::Time, Item};
    use chrono::{DateTime, FixedOffset};

    fn at_utc(date: Vec<Item>) -> DateTime<FixedOffset> {
        at_date(date, chrono::Utc::now().fixed_offset()).unwrap()
    }

    fn test_eq_fmt(fmt: &str, input: &str) -> String {
        let input = input.to_ascii_lowercase();
        parse(&mut input.as_str())
            .map(at_utc)
            .map_err(|e| eprintln!("TEST FAILED AT:\n{e}"))
            .expect("parsing failed during tests")
            .format(fmt)
            .to_string()
    }

    #[test]
    fn date_and_time() {
        assert_eq!(
            parse(&mut "   10:10   2022-12-12    "),
            Ok(vec![
                Item::Time(Time {
                    hour: 10,
                    minute: 10,
                    second: 0.0,
                    offset: None,
                }),
                Item::Date(Date {
                    day: 12,
                    month: 12,
                    year: Some(2022)
                })
            ])
        );

        //               format,  expected output, input
        assert_eq!("2024-01-02", test_eq_fmt("%Y-%m-%d", "2024-01-02"));

        // https://github.com/uutils/coreutils/issues/6662
        assert_eq!("2005-01-02", test_eq_fmt("%Y-%m-%d", "2005-01-01 +1 day"));

        // https://github.com/uutils/coreutils/issues/6644
        assert_eq!("Jul 16", test_eq_fmt("%b %d", "Jul 16"));
        assert_eq!("0718061449", test_eq_fmt("%m%d%H%M%S", "Jul 18 06:14:49"));
        assert_eq!(
            "07182024061449",
            test_eq_fmt("%m%d%Y%H%M%S", "Jul 18, 2024 06:14:49")
        );
        assert_eq!(
            "07182024061449",
            test_eq_fmt("%m%d%Y%H%M%S", "Jul 18 06:14:49 2024")
        );

        // https://github.com/uutils/coreutils/issues/5177
        assert_eq!(
            "2023-07-27T13:53:54+00:00",
            test_eq_fmt("%+", "@1690466034")
        );

        // https://github.com/uutils/coreutils/issues/6398
        // TODO: make this work
        // assert_eq!("1111 1111 00", test_eq_fmt("%m%d %H%M %S", "11111111"));

        assert_eq!(
            "2024-07-17 06:14:49 +00:00",
            test_eq_fmt("%Y-%m-%d %H:%M:%S %Z", "Jul 17 06:14:49 2024 GMT"),
        );

        assert_eq!(
            "2024-07-17 06:14:49 -03:00",
            test_eq_fmt("%Y-%m-%d %H:%M:%S %Z", "Jul 17 06:14:49 2024 BRT"),
        );
    }
}

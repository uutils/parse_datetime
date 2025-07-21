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
//!  - [`timezone`]
//!  - [`combined`]
//!  - [`weekday`]
//!  - [`relative`]

#![allow(deprecated)]

// date and time items
mod combined;
mod date;
mod epoch;
mod relative;
mod time;
mod timezone;
mod weekday;

// utility modules
mod builder;
mod ordinal;
mod primitive;

use builder::DateTimeBuilder;
use chrono::{DateTime, FixedOffset};
use primitive::space;
use winnow::{
    combinator::{alt, eof, terminated, trace},
    error::{AddContext, ContextError, ErrMode, StrContext, StrContextValue},
    stream::Stream,
    ModalResult, Parser,
};

use crate::ParseDateTimeError;

#[derive(PartialEq, Debug)]
pub(crate) enum Item {
    Timestamp(i32),
    Year(u32),
    DateTime(combined::DateTime),
    Date(date::Date),
    Time(time::Time),
    Weekday(weekday::Weekday),
    Relative(relative::Relative),
    TimeZone(time::Offset),
}

/// Build a `DateTime<FixedOffset>` from a `DateTimeBuilder` and a base date.
pub(crate) fn at_date(
    builder: DateTimeBuilder,
    base: DateTime<FixedOffset>,
) -> Result<DateTime<FixedOffset>, ParseDateTimeError> {
    builder
        .set_base(base)
        .build()
        .ok_or(ParseDateTimeError::InvalidInput)
}

/// Build a `DateTime<FixedOffset>` from a `DateTimeBuilder` and the current
/// time.
pub(crate) fn at_local(
    builder: DateTimeBuilder,
) -> Result<DateTime<FixedOffset>, ParseDateTimeError> {
    builder.build().ok_or(ParseDateTimeError::InvalidInput)
}

/// Parse a date and time string.
///
/// Grammar:
///
/// ```ebnf
/// spec = timestamp | items ;
///
/// timestamp = "@" , dec_int ;
///
/// items = item , { item } ;
/// item = datetime | date | time | relative | weekday | timezone | year ;
///
/// datetime = iso_date , [ "T" | "t" | whitespace ] , iso_time ;
///
/// iso_date = year , [ delim ] , month , [ delim ] , day ;
/// year = dec_int ;
/// month = dec_int ;
/// day = dec_int ;
/// delim = [ { whitespace } ] , "-" , [ { whitespace } ] ;
/// ```
pub(crate) fn parse(input: &mut &str) -> ModalResult<DateTimeBuilder> {
    trace("parse", alt((parse_timestamp, parse_items))).parse_next(input)
}

/// Parse a timestamp.
///
/// From the GNU docs:
///
/// > (Timestamp) Such a number cannot be combined with any other date item, as
/// > it specifies a complete timestamp.
fn parse_timestamp(input: &mut &str) -> ModalResult<DateTimeBuilder> {
    trace(
        "parse_timestamp",
        terminated(epoch::parse.map(Item::Timestamp), eof),
    )
    .verify_map(|ts: Item| {
        if let Item::Timestamp(ts) = ts {
            DateTimeBuilder::new().set_timestamp(ts).ok()
        } else {
            None
        }
    })
    .parse_next(input)
}

/// Parse a sequence of items.
fn parse_items(input: &mut &str) -> ModalResult<DateTimeBuilder> {
    let mut builder = DateTimeBuilder::new();

    loop {
        match parse_item.parse_next(input) {
            Ok(item) => match item {
                Item::Timestamp(ts) => {
                    builder = builder
                        .set_timestamp(ts)
                        .map_err(|e| expect_error(input, e))?;
                }
                Item::Year(year) => {
                    builder = builder.set_year(year).map_err(|e| expect_error(input, e))?;
                }
                Item::DateTime(dt) => {
                    builder = builder
                        .set_date(dt.date)
                        .map_err(|e| expect_error(input, e))?
                        .set_time(dt.time)
                        .map_err(|e| expect_error(input, e))?;
                }
                Item::Date(d) => {
                    builder = builder.set_date(d).map_err(|e| expect_error(input, e))?;
                }
                Item::Time(t) => {
                    builder = builder.set_time(t).map_err(|e| expect_error(input, e))?;
                }
                Item::Weekday(weekday) => {
                    builder = builder
                        .set_weekday(weekday)
                        .map_err(|e| expect_error(input, e))?;
                }
                Item::TimeZone(tz) => {
                    builder = builder
                        .set_timezone(tz)
                        .map_err(|e| expect_error(input, e))?;
                }
                Item::Relative(rel) => {
                    builder = builder.push_relative(rel);
                }
            },
            Err(ErrMode::Backtrack(_)) => break,
            Err(e) => return Err(e),
        }
    }

    space.parse_next(input)?;
    if !input.is_empty() {
        return Err(expect_error(input, "unexpected input"));
    }

    Ok(builder)
}

/// Parse an item.
fn parse_item(input: &mut &str) -> ModalResult<Item> {
    trace(
        "parse_item",
        alt((
            combined::parse.map(Item::DateTime),
            date::parse.map(Item::Date),
            time::parse.map(Item::Time),
            relative::parse.map(Item::Relative),
            weekday::parse.map(Item::Weekday),
            timezone::parse.map(Item::TimeZone),
            date::year.map(Item::Year),
        )),
    )
    .parse_next(input)
}

/// Create an error with context for unexpected input.
fn expect_error(input: &mut &str, reason: &'static str) -> ErrMode<ContextError> {
    ErrMode::Cut(ContextError::new()).add_context(
        input,
        &input.checkpoint(),
        StrContext::Expected(StrContextValue::Description(reason)),
    )
}

#[cfg(test)]
mod tests {
    use super::{at_date, parse, DateTimeBuilder};
    use chrono::{
        DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike, Utc,
    };

    fn at_utc(builder: DateTimeBuilder) -> DateTime<FixedOffset> {
        at_date(builder, Utc::now().fixed_offset()).unwrap()
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
            "2022-12-12",
            test_eq_fmt("%Y-%m-%d", "   10:10   2022-12-12    ")
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
            test_eq_fmt("%Y-%m-%dT%H:%M:%S%:z", "@1690466034")
        );

        // https://github.com/uutils/coreutils/issues/6398
        // TODO: make this work
        // assert_eq!("1111 1111 00", test_eq_fmt("%m%d %H%M %S", "11111111"));

        assert_eq!(
            "2024-07-17 06:14:49 +00:00",
            test_eq_fmt("%Y-%m-%d %H:%M:%S %:z", "Jul 17 06:14:49 2024 GMT"),
        );

        assert_eq!(
            "2024-07-17 06:14:49.567 +00:00",
            test_eq_fmt("%Y-%m-%d %H:%M:%S%.f %:z", "Jul 17 06:14:49.567 2024 GMT"),
        );

        assert_eq!(
            "2024-07-17 06:14:49.567 +00:00",
            test_eq_fmt("%Y-%m-%d %H:%M:%S%.f %:z", "Jul 17 06:14:49,567 2024 GMT"),
        );

        assert_eq!(
            "2024-07-17 06:14:49 -03:00",
            test_eq_fmt("%Y-%m-%d %H:%M:%S %:z", "Jul 17 06:14:49 2024 BRT"),
        );
    }

    #[test]
    fn invalid() {
        let result = parse(&mut "2025-05-19 2024-05-20 06:14:49");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("date cannot appear more than once"));

        let result = parse(&mut "2025-05-19 2024-05-20");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("date cannot appear more than once"));

        let result = parse(&mut "06:14:49 06:14:49");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("time cannot appear more than once"));

        let result = parse(&mut "2025-05-19 2024");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("year cannot appear more than once"));

        let result = parse(&mut "2025-05-19 +00:00 +01:00");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unexpected input"));

        let result = parse(&mut "m1y");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("timezone cannot appear more than once"));

        let result = parse(&mut "2025-05-19 abcdef");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unexpected input"));

        let result = parse(&mut "@1690466034 2025-05-19");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unexpected input"));

        let result = parse(&mut "2025-05-19 @1690466034");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unexpected input"));
    }

    #[test]
    fn relative_weekday() {
        // Jan 1 2025 is a Wed
        let now = Utc
            .from_utc_datetime(&NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            ))
            .fixed_offset();

        assert_eq!(
            at_date(parse(&mut "last wed").unwrap(), now).unwrap(),
            now - chrono::Duration::days(7)
        );
        assert_eq!(at_date(parse(&mut "this wed").unwrap(), now).unwrap(), now);
        assert_eq!(
            at_date(parse(&mut "next wed").unwrap(), now).unwrap(),
            now + chrono::Duration::days(7)
        );
        assert_eq!(
            at_date(parse(&mut "last thu").unwrap(), now).unwrap(),
            now - chrono::Duration::days(6)
        );
        assert_eq!(
            at_date(parse(&mut "this thu").unwrap(), now).unwrap(),
            now + chrono::Duration::days(1)
        );
        assert_eq!(
            at_date(parse(&mut "next thu").unwrap(), now).unwrap(),
            now + chrono::Duration::days(1)
        );
        assert_eq!(
            at_date(parse(&mut "1 wed").unwrap(), now).unwrap(),
            now + chrono::Duration::days(7)
        );
        assert_eq!(
            at_date(parse(&mut "1 thu").unwrap(), now).unwrap(),
            now + chrono::Duration::days(1)
        );
        assert_eq!(
            at_date(parse(&mut "2 wed").unwrap(), now).unwrap(),
            now + chrono::Duration::days(14)
        );
        assert_eq!(
            at_date(parse(&mut "2 thu").unwrap(), now).unwrap(),
            now + chrono::Duration::days(8)
        );
    }

    #[test]
    fn relative_date_time() {
        let now = Utc::now().fixed_offset();

        let result = at_date(parse(&mut "2 days ago").unwrap(), now).unwrap();
        assert_eq!(result, now - chrono::Duration::days(2));
        assert_eq!(result.hour(), now.hour());
        assert_eq!(result.minute(), now.minute());
        assert_eq!(result.second(), now.second());

        let result = at_date(parse(&mut "2025-01-01 2 days ago").unwrap(), now).unwrap();
        assert_eq!(result.hour(), 0);
        assert_eq!(result.minute(), 0);
        assert_eq!(result.second(), 0);

        let result = at_date(parse(&mut "3 weeks").unwrap(), now).unwrap();
        assert_eq!(result, now + chrono::Duration::days(21));
        assert_eq!(result.hour(), now.hour());
        assert_eq!(result.minute(), now.minute());
        assert_eq!(result.second(), now.second());

        let result = at_date(parse(&mut "2025-01-01 3 weeks").unwrap(), now).unwrap();
        assert_eq!(result.hour(), 0);
        assert_eq!(result.minute(), 0);
        assert_eq!(result.second(), 0);
    }
}

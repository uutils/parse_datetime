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
mod primitive;
mod relative;
mod time;
mod weekday;

mod epoch {
    use winnow::{combinator::preceded, ModalResult, Parser};

    use super::primitive::{dec_int, s};

    pub fn parse(input: &mut &str) -> ModalResult<i32> {
        s(preceded("@", dec_int)).parse_next(input)
    }
}

mod timezone {
    use winnow::ModalResult;

    use super::time;

    pub(crate) fn parse(input: &mut &str) -> ModalResult<time::Offset> {
        time::timezone(input)
    }
}

use chrono::NaiveDate;
use chrono::{DateTime, Datelike, FixedOffset, TimeZone, Timelike};

use primitive::space;
use winnow::{
    combinator::{alt, trace},
    error::{AddContext, ContextError, ErrMode, StrContext, StrContextValue},
    stream::Stream,
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

fn expect_error(input: &mut &str, reason: &'static str) -> ErrMode<ContextError> {
    ErrMode::Cut(ContextError::new()).add_context(
        input,
        &input.checkpoint(),
        StrContext::Expected(StrContextValue::Description(reason)),
    )
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
                            return Err(expect_error(
                                input,
                                "date or time cannot appear more than once",
                            ));
                        }

                        date_seen = true;
                        time_seen = true;
                        if dt.date.year.is_some() {
                            year_seen = true;
                        }
                    }
                    Item::Date(ref d) => {
                        if date_seen {
                            return Err(expect_error(input, "date cannot appear more than once"));
                        }

                        date_seen = true;
                        if d.year.is_some() {
                            year_seen = true;
                        }
                    }
                    Item::Time(ref t) => {
                        if time_seen {
                            return Err(expect_error(input, "time cannot appear more than once"));
                        }

                        if t.offset.is_some() {
                            if tz_seen {
                                return Err(expect_error(
                                    input,
                                    "timezone cannot appear more than once",
                                ));
                            }
                            tz_seen = true;
                        }

                        time_seen = true;
                    }
                    Item::Year(_) => {
                        if year_seen {
                            return Err(expect_error(input, "year cannot appear more than once"));
                        }
                        year_seen = true;
                    }
                    Item::TimeZone(_) => {
                        if tz_seen {
                            return Err(expect_error(
                                input,
                                "timezone cannot appear more than once",
                            ));
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
        return Err(expect_error(input, "unexpected input"));
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
    let offset: FixedOffset = chrono::FixedOffset::try_from(offset).ok()?;
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

fn at_date_inner(date: Vec<Item>, at: DateTime<FixedOffset>) -> Option<DateTime<FixedOffset>> {
    let mut d = at
        .with_hour(0)
        .unwrap()
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();

    // This flag is used by relative items to determine which date/time to use.
    // If any date/time item is set, it will use that; otherwise, it will use
    // the `at` value.
    let date_time_set = date.iter().any(|item| {
        matches!(
            item,
            Item::Timestamp(_)
                | Item::Date(_)
                | Item::DateTime(_)
                | Item::Year(_)
                | Item::Time(_)
                | Item::Weekday(_)
        )
    });

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
                let offset = offset
                    .and_then(|o| chrono::FixedOffset::try_from(o).ok())
                    .unwrap_or(*d.offset());

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
                let offset = offset
                    .and_then(|o| chrono::FixedOffset::try_from(o).ok())
                    .unwrap_or(*d.offset());

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
            Item::Weekday(weekday::Weekday { offset: x, day }) => {
                let mut x = x;
                let day = day.into();

                // If the current day is not the target day, we need to adjust
                // the x value to ensure we find the correct day.
                //
                // Consider this:
                // Assuming today is Monday, next Friday is actually THIS Friday;
                // but next Monday is indeed NEXT Monday.
                if d.weekday() != day && x > 0 {
                    x -= 1;
                }

                // Calculate the delta to the target day.
                //
                // Assuming today is Thursday, here are some examples:
                //
                // Example 1: last Thursday (x = -1, day = Thursday)
                //            delta = (3 - 3) % 7 + (-1) * 7 = -7
                //
                // Example 2: last Monday (x = -1, day = Monday)
                //            delta = (0 - 3) % 7 + (-1) * 7 = -3
                //
                // Example 3: next Monday (x = 1, day = Monday)
                //            delta = (0 - 3) % 7 + (0) * 7 = 4
                // (Note that we have adjusted the x value above)
                //
                // Example 4: next Thursday (x = 1, day = Thursday)
                //            delta = (3 - 3) % 7 + (1) * 7 = 7
                let delta = (day.num_days_from_monday() as i32
                    - d.weekday().num_days_from_monday() as i32)
                    .rem_euclid(7)
                    + x * 7;

                d = if delta < 0 {
                    d.checked_sub_days(chrono::Days::new((-delta) as u64))?
                } else {
                    d.checked_add_days(chrono::Days::new(delta as u64))?
                }
            }
            Item::Relative(rel) => {
                // If date and/or time is set, use the set value; otherwise, use
                // the reference value.
                if !date_time_set {
                    d = at;
                }

                match rel {
                    relative::Relative::Years(x) => {
                        d = d.with_year(d.year() + x)?;
                    }
                    relative::Relative::Months(x) => {
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
                    relative::Relative::Days(x) => d += chrono::Duration::days(x.into()),
                    relative::Relative::Hours(x) => d += chrono::Duration::hours(x.into()),
                    relative::Relative::Minutes(x) => {
                        d += chrono::Duration::minutes(x.into());
                    }
                    // Seconds are special because they can be given as a float
                    relative::Relative::Seconds(x) => {
                        d += chrono::Duration::seconds(x as i64);
                    }
                }
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
    at: DateTime<FixedOffset>,
) -> Result<DateTime<FixedOffset>, ParseDateTimeError> {
    at_date_inner(date, at).ok_or(ParseDateTimeError::InvalidInput)
}

pub(crate) fn at_local(date: Vec<Item>) -> Result<DateTime<FixedOffset>, ParseDateTimeError> {
    at_date(date, chrono::Local::now().into())
}

#[cfg(test)]
mod tests {
    use super::{at_date, date::Date, parse, time::Time, Item};
    use chrono::{
        DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike, Utc,
    };

    fn at_utc(date: Vec<Item>) -> DateTime<FixedOffset> {
        at_date(date, Utc::now().fixed_offset()).unwrap()
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
            .contains("date or time cannot appear more than once"));

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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("timezone cannot appear more than once"));

        let result = parse(&mut "m1y");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("timezone cannot appear more than once"));

        let result = parse(&mut "2025-05-19 abcdef");
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

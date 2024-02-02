//! A Rust crate for parsing human-readable relative time strings and human-readable datetime strings and converting them to a `DateTime`.
//! The function supports the following formats for time:
//!
//! * ISO formats
//! * timezone offsets, e.g., "UTC-0100"
//! * unix timestamps, e.g., "@12"
//! * relative time to now, e.g. "+1 hour"
//!
use regex::{Error as RegexError, Regex};
use std::error::Error;
use std::fmt::{self, Display};

use chrono::{
    DateTime, Datelike, Duration, FixedOffset, Local, LocalResult, NaiveDate, NaiveDateTime,
    TimeZone, Timelike,
};

use crate::parse_relative_time::dt_from_relative;
use parse_timestamp::parse_timestamp;

// Expose parse_datetime
mod parse_relative_time;
mod parse_timestamp;

mod parse_weekday;

#[derive(Debug, PartialEq)]
pub enum ParseDateTimeError {
    InvalidRegex(RegexError),
    InvalidInput,
}

impl Display for ParseDateTimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRegex(err) => {
                write!(f, "Invalid regex for time pattern: {err}")
            }
            Self::InvalidInput => {
                write!(
                    f,
                    "Invalid input string: cannot be parsed as a relative time"
                )
            }
        }
    }
}

impl Error for ParseDateTimeError {}

impl From<RegexError> for ParseDateTimeError {
    fn from(err: RegexError) -> Self {
        Self::InvalidRegex(err)
    }
}

/// Formats that parse input can take.
/// Taken from `touch` coreutils
mod format {
    pub const ISO_8601: &str = "%Y-%m-%d";
    pub const ISO_8601_NO_SEP: &str = "%Y%m%d";
    pub const POSIX_LOCALE: &str = "%a %b %e %H:%M:%S %Y";
    pub const YYYYMMDDHHMM_DOT_SS: &str = "%Y%m%d%H%M.%S";
    pub const YYYYMMDDHHMMSS: &str = "%Y-%m-%d %H:%M:%S.%f";
    pub const YYYYMMDDHHMMS: &str = "%Y-%m-%d %H:%M:%S";
    pub const YYYY_MM_DD_HH_MM: &str = "%Y-%m-%d %H:%M";
    pub const YYYYMMDDHHMM: &str = "%Y%m%d%H%M";
    pub const YYYYMMDDHHMM_OFFSET: &str = "%Y%m%d%H%M %z";
    pub const YYYYMMDDHHMM_UTC_OFFSET: &str = "%Y%m%d%H%MUTC%z";
    pub const YYYYMMDDHHMM_ZULU_OFFSET: &str = "%Y%m%d%H%MZ%z";
    pub const YYYYMMDDHHMM_HYPHENATED_OFFSET: &str = "%Y-%m-%d %H:%M %z";
    pub const YYYYMMDDHHMMS_T_SEP: &str = "%Y-%m-%dT%H:%M:%S";
    pub const UTC_OFFSET: &str = "UTC%#z";
    pub const ZULU_OFFSET: &str = "Z%#z";
}

/// Parses a time string with optional modifiers and returns a `DateTime<FixedOffset>` representing the
/// absolute time of the string.
///
/// # Arguments
///
/// * `s` - A string slice representing the time.
///
/// # Examples
///
/// ```
/// use chrono::{DateTime, Utc, TimeZone, Local, FixedOffset};
/// let date = parse_datetime::parse_datetime("2023-06-03 12:00:01Z +16 days");
/// assert_eq!(date.unwrap(), Utc.with_ymd_and_hms(2023, 06, 19, 12, 00, 01).unwrap());
///
/// let time = parse_datetime::parse_datetime("2023-06-03 00:00:00Z tomorrow 1230");
/// assert_eq!(time.unwrap(), Utc.with_ymd_and_hms(2023, 06, 04, 12, 30, 00).unwrap());
/// ```
///
/// # Formats
///
///    %Y-%m-%d
///
///    %Y%m%d
///
///   %a %b %e %H:%M:%S %Y
///
///    %Y%m%d%H%M.%S
///
///    %Y-%m-%d %H:%M:%S.%f
///
///    %Y-%m-%d %H:%M:%S
///
///    %Y-%m-%d %H:%M
///
///    %Y%m%d%H%M
///
///    %Y%m%d%H%M %z
///
///    %Y%m%d%H%MUTC%z
///
///    %Y%m%d%H%MZ%z
///
///    %Y-%m-%d %H:%M %z
///
///    %Y-%m-%dT%H:%M:%S
///
///    UTC%#z
///
///    Z%#z
///
///
/// # Modifiers
///
/// Years
///
/// Months
///
/// Fortnights
///
/// Weeks
///
/// Days
///
/// Hours
///
/// Minutes
///
/// Seconds
///
/// Tomorrow
///
/// Yesterday
///
/// Now
///
/// Today
///
/// Time (represented as an isolated 4 digit number)
///
/// # Returns
///
/// * `Ok(DateTime<FixedOffset>)` - If the input string can be parsed as a time
/// * `Err(ParseDateTimeError)` - If the input string cannot be parsed as a relative time
///
/// # Errors
///
/// This function will return `Err(ParseDateTimeError::InvalidInput)` if the input string
/// cannot be parsed.
pub fn parse_datetime<S: AsRef<str> + Clone>(
    s: S,
) -> Result<DateTime<FixedOffset>, ParseDateTimeError> {
    if let Ok(parsed) = try_parse(s.as_ref()) {
        return Ok(parsed);
    }

    for fmt in [
        format::YYYYMMDDHHMM_OFFSET,
        format::YYYYMMDDHHMM_HYPHENATED_OFFSET,
        format::YYYYMMDDHHMM_UTC_OFFSET,
        format::YYYYMMDDHHMM_ZULU_OFFSET,
    ] {
        if let Ok((parsed, modifier)) = DateTime::parse_and_remainder(s.as_ref(), fmt) {
            if modifier.is_empty() {
                return Ok(parsed);
            }
            if let Ok(dt) = dt_from_relative(modifier, parsed) {
                return Ok(dt);
            }
        }
    }

    // Parse formats with no offset, assume local time
    for fmt in [
        format::YYYYMMDDHHMMS_T_SEP,
        format::YYYYMMDDHHMM,
        format::YYYYMMDDHHMMSS,
        format::YYYYMMDDHHMMS,
        format::YYYY_MM_DD_HH_MM,
        format::YYYYMMDDHHMM_DOT_SS,
        format::POSIX_LOCALE,
    ] {
        if let Ok((parsed, modifier)) = NaiveDateTime::parse_and_remainder(s.as_ref(), fmt) {
            if let Ok(dt) = naive_dt_to_fixed_offset(Local::now(), parsed) {
                if modifier.is_empty() {
                    return Ok(dt);
                } else if let Ok(dt) = dt_from_relative(modifier, dt) {
                    return Ok(dt);
                }
            };
        }
    }

    // parse weekday
    if let Ok(date) = parse_weekday(Local::now().into(), s.as_ref()) {
        return Ok(date);
    }

    // Parse epoch seconds
    if let Ok(timestamp) = parse_timestamp(s.as_ref()) {
        if let Some(timestamp_date) = NaiveDateTime::from_timestamp_opt(timestamp, 0) {
            if let Ok(dt) = naive_dt_to_fixed_offset(Local::now(), timestamp_date) {
                return Ok(dt);
            }
        }
    }

    // Parse date only formats - assume midnight local timezone
    for fmt in [format::ISO_8601, format::ISO_8601_NO_SEP] {
        if let Ok((date, remainder)) = NaiveDate::parse_and_remainder(s.as_ref(), fmt) {
            let ndt = date.and_hms_opt(0, 0, 0).unwrap();
            if let Ok(dt) = naive_dt_to_fixed_offset(Local::now(), ndt) {
                if let Ok(dt) = dt_from_relative(remainder, dt) {
                    return Ok(dt);
                }
            }
        }
    }

    // Parse offsets. chrono doesn't provide any functionality to parse
    // offsets, so instead we replicate parse_date behaviour by getting
    // the current date with local, and create a date time string at midnight,
    // before trying offset suffixes
    let ts = format!("{}", Local::now().format("%Y%m%d")) + "0000" + s.as_ref();
    for fmt in [format::UTC_OFFSET, format::ZULU_OFFSET] {
        let f = format::YYYYMMDDHHMM.to_owned() + fmt;
        if let Ok((parsed, modifier)) = DateTime::parse_and_remainder(&ts, &f) {
            if modifier.trim().is_empty() {
                return Ok(parsed);
                // if the is a non empty remainder we check to see if the
                // first letter is a space. If it is not we reject the input
                // because it could be left over form an invalid offset.
            } else if !modifier.as_bytes()[0].is_ascii_whitespace() {
                return Err(ParseDateTimeError::InvalidInput);
            }
            if let Ok(dt) = dt_from_relative(modifier, parsed) {
                return Ok(dt);
            }
        }
    }

    parse_datetime_at_date(Local::now(), s.as_ref())
}

fn try_parse<S: AsRef<str>>(s: S) -> Result<DateTime<FixedOffset>, ParseDateTimeError> {
    let re = Regex::new(r"(?ix)
                                (?:[+-]?\s*\d+\s*)?
                                (\s*(?:years?|months?|fortnights?|weeks?|days?|hours?|h|minutes?|mins?|m|seconds?|secs?|s)|
                                (\s*(?:next|last)\s*)|
                                (\s*(?:yesterday|tomorrow|now|today)\s*)|
                                (\s*(?:and|,)\s*))").unwrap();

    match re.find(s.as_ref()) {
        None => s
            .as_ref()
            .parse::<DateTime<FixedOffset>>()
            .map_err(|_| ParseDateTimeError::InvalidInput),
        Some(matcher) => {
            let pivot = matcher.start();
            let date = &s.as_ref()[..pivot];
            let modifier = &s.as_ref()[pivot..];
            if let Ok(dt) = date.parse::<DateTime<FixedOffset>>() {
                dt_from_relative(modifier, dt)
            } else if let Ok(dt) = date.parse::<NaiveDate>() {
                let ndt = dt.and_hms_opt(0, 0, 0).unwrap();
                dt_from_relative(
                    modifier,
                    naive_dt_to_fixed_offset(Local::now(), ndt).unwrap(),
                )
            } else {
                Err(ParseDateTimeError::InvalidInput)
            }
        }
    }
}

fn parse_weekday<S: AsRef<str>>(
    date: DateTime<FixedOffset>,
    s: S,
) -> Result<DateTime<FixedOffset>, ParseDateTimeError> {
    if let Some(weekday) = parse_weekday::parse_weekday(s.as_ref()) {
        let mut beginning_of_day = date
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap();

        while beginning_of_day.weekday() != weekday {
            beginning_of_day += Duration::days(1);
        }
        return Ok(beginning_of_day);
    }
    Err(ParseDateTimeError::InvalidInput)
}

/// Parses a time string at a specific date and returns a `DateTime` representing the
/// absolute time of the string.
///
/// # Arguments
///
/// * date - The date represented in local time
/// * `s` - A string slice representing the time.
///
/// # Examples
///
/// ```
/// use chrono::{Duration, Local};
/// use parse_datetime::parse_datetime_at_date;
///
///  let now = Local::now();
///  let after = parse_datetime_at_date(now, "+3 days");
///
///  assert_eq!(
///    (now + Duration::days(3)).naive_utc(),
///    after.unwrap().naive_utc()
///  );
/// ```
///
/// # Returns
///
/// * `Ok(DateTime<FixedOffset>)` - If the input string can be parsed as a time
/// * `Err(ParseDateTimeError)` - If the input string cannot be parsed as a relative time
///
/// # Errors
///
/// This function will return `Err(ParseDateTimeError::InvalidInput)` if the input string
/// cannot be parsed as a relative time.
pub fn parse_datetime_at_date<S: AsRef<str> + Clone>(
    date: DateTime<Local>,
    s: S,
) -> Result<DateTime<FixedOffset>, ParseDateTimeError> {
    if let Ok(dt) = parse_weekday(date.into(), s.as_ref()) {
        return Ok(dt);
    }
    // Parse relative time.
    dt_from_relative(s.as_ref(), date.fixed_offset())
}

// Convert NaiveDateTime to DateTime<FixedOffset> by assuming the offset
// is local time
fn naive_dt_to_fixed_offset(
    local: DateTime<Local>,
    dt: NaiveDateTime,
) -> Result<DateTime<FixedOffset>, ()> {
    match local.offset().from_local_datetime(&dt) {
        LocalResult::Single(dt) => Ok(dt),
        _ => Err(()),
    }
}

#[cfg(test)]
mod tests {
    static TEST_TIME: i64 = 1613371067;

    #[cfg(test)]
    mod iso_8601 {
        use std::env;

        use crate::ParseDateTimeError;
        use crate::{parse_datetime, tests::TEST_TIME};

        #[test]
        fn test_t_sep() {
            env::set_var("TZ", "UTC");
            let dt = "2021-02-15T06:37:47";
            let actual = parse_datetime(dt);
            assert_eq!(actual.unwrap().timestamp(), TEST_TIME);
        }

        #[test]
        fn test_space_sep() {
            env::set_var("TZ", "UTC");
            let dt = "2021-02-15 06:37:47";
            let actual = parse_datetime(dt);
            assert_eq!(actual.unwrap().timestamp(), TEST_TIME);
        }

        #[test]
        fn test_space_sep_offset() {
            env::set_var("TZ", "UTC");
            let dt = "2021-02-14 22:37:47 -0800";
            let actual = parse_datetime(dt);
            assert_eq!(actual.unwrap().timestamp(), TEST_TIME);
        }

        #[test]
        fn test_t_sep_offset() {
            env::set_var("TZ", "UTC");
            let dt = "2021-02-14T22:37:47 -0800";
            let actual = parse_datetime(dt);
            assert_eq!(actual.unwrap().timestamp(), TEST_TIME);
        }

        #[test]
        fn invalid_formats() {
            let invalid_dts = vec!["NotADate", "202104", "202104-12T22:37:47"];
            for dt in invalid_dts {
                assert_eq!(parse_datetime(dt), Err(ParseDateTimeError::InvalidInput));
            }
        }

        #[test]
        fn test_epoch_seconds() {
            env::set_var("TZ", "UTC");
            let dt = "@1613371067";
            let actual = parse_datetime(dt);
            assert_eq!(actual.unwrap().timestamp(), TEST_TIME);
        }
    }

    #[cfg(test)]
    mod offsets {
        use chrono::Local;

        use crate::parse_datetime;
        use crate::ParseDateTimeError;

        #[test]
        fn test_positive_offsets() {
            let offsets = vec![
                "UTC+07:00",
                "UTC+0700",
                "UTC+07",
                "Z+07:00",
                "Z+0700",
                "Z+07",
            ];

            let expected = format!("{}{}", Local::now().format("%Y%m%d"), "0000+0700");
            for offset in offsets {
                let actual = parse_datetime(offset).unwrap();
                assert_eq!(expected, format!("{}", actual.format("%Y%m%d%H%M%z")));
            }
        }

        #[test]
        fn test_partial_offset() {
            let offsets = vec!["UTC+00:15", "UTC+0015", "Z+00:15", "Z+0015"];
            let expected = format!("{}{}", Local::now().format("%Y%m%d"), "0000+0015");
            for offset in offsets {
                let actual = parse_datetime(offset).unwrap();
                assert_eq!(expected, format!("{}", actual.format("%Y%m%d%H%M%z")));
            }
        }

        #[test]
        fn invalid_offset_format() {
            let invalid_offsets = vec!["+0700", "UTC+2", "Z-1", "UTC+01005"];
            for offset in invalid_offsets {
                assert_eq!(
                    parse_datetime(offset),
                    Err(ParseDateTimeError::InvalidInput)
                );
            }
        }
    }

    #[cfg(test)]
    mod relative_time {
        use crate::parse_datetime;
        use chrono::DateTime;

        #[test]
        fn test_positive_offsets() {
            let relative_times = vec![
                "today",
                "yesterday",
                "1 minute",
                "3 hours",
                "1 year 3 months",
            ];

            for relative_time in relative_times {
                assert_eq!(parse_datetime(relative_time).is_ok(), true);
            }
        }

        #[test]
        fn test_date_with_modifiers() {
            let format = "%Y %b %d %H:%M:%S.%f %z";
            let input = [
                (
                    parse_datetime("2022-08-31 00:00:00 +0000 1 month 1230").unwrap(),
                    DateTime::parse_from_str("2022 Oct 1 12:30:00.0 +0000", format).unwrap(),
                ),
                (
                    parse_datetime("2022-08-31 00:00:00.0 +0000 2 month 1230").unwrap(),
                    DateTime::parse_from_str("2022 Oct 31 12:30:00.0 +0000", format).unwrap(),
                ),
                (
                    parse_datetime("2020-02-29 00:00:00.0 +0000 1 year 1230").unwrap(),
                    DateTime::parse_from_str("2021 Mar 1 12:30:00.0 +0000", format).unwrap(),
                ),
                (
                    parse_datetime("2020-02-29 00:00:00.0 +0500 100 year 1230").unwrap(),
                    DateTime::parse_from_str("2120 Feb 29 12:30:00.0 +0500", format).unwrap(),
                ),
                (
                    parse_datetime("2020-02-29 00:00:00.0 -0500 101 year 1230").unwrap(),
                    DateTime::parse_from_str("2121 Mar 1 12:30:00.0 -0500", format).unwrap(),
                ),
                (
                    parse_datetime("2020-02-29 00:00:00.0 +1000 1 month yesterday").unwrap(),
                    DateTime::parse_from_str("2020 Mar 28 00:00:00.0 +1000", format).unwrap(),
                ),
                (
                    parse_datetime("2022-08-31 00:00:00.0 +0000 1 month 1230").unwrap(),
                    DateTime::parse_from_str("2022 Oct 1 12:30:00.0 +0000", format).unwrap(),
                ),
                (
                    parse_datetime("2022-08-31 00:00:00.0 +0000 +12 seconds").unwrap(),
                    DateTime::parse_from_str("2022 Aug 31 00:00:12.0 +0000", format).unwrap(),
                ),
                (
                    parse_datetime("2022-08-31").unwrap(),
                    DateTime::parse_from_str("2022 Aug 31 00:00:00.0 +0000", format).unwrap(),
                ),
            ];
            for (parsed, expected) in input {
                assert_eq!(parsed, expected);
            }
        }
    }

    #[cfg(test)]
    mod weekday {
        use chrono::{DateTime, Local, TimeZone};

        use crate::parse_datetime_at_date;

        fn get_formatted_date(date: DateTime<Local>, weekday: &str) -> String {
            let result = parse_datetime_at_date(date, weekday).unwrap();

            return result.format("%F %T %f").to_string();
        }
        #[test]
        fn test_weekday() {
            // add some constant hours and minutes and seconds to check its reset
            let date = Local.with_ymd_and_hms(2023, 02, 28, 10, 12, 3).unwrap();

            // 2023-2-28 is tuesday
            assert_eq!(
                get_formatted_date(date, "tuesday"),
                "2023-02-28 00:00:00 000000000"
            );

            // 2023-3-01 is wednesday
            assert_eq!(
                get_formatted_date(date, "wed"),
                "2023-03-01 00:00:00 000000000"
            );

            assert_eq!(
                get_formatted_date(date, "thu"),
                "2023-03-02 00:00:00 000000000"
            );

            assert_eq!(
                get_formatted_date(date, "fri"),
                "2023-03-03 00:00:00 000000000"
            );

            assert_eq!(
                get_formatted_date(date, "sat"),
                "2023-03-04 00:00:00 000000000"
            );

            assert_eq!(
                get_formatted_date(date, "sun"),
                "2023-03-05 00:00:00 000000000"
            );
        }
    }

    #[cfg(test)]
    mod timestamp {
        use chrono::{TimeZone, Utc};

        use crate::parse_datetime;

        #[test]
        fn test_positive_and_negative_offsets() {
            let offsets: Vec<i64> = vec![
                0, 1, 2, 10, 100, 150, 2000, 1234400000, 1334400000, 1692582913, 2092582910,
            ];

            for offset in offsets {
                // positive offset
                let time = Utc.timestamp_opt(offset, 0).unwrap();
                let dt = parse_datetime(format!("@{}", offset));
                assert_eq!(dt.unwrap(), time);

                // negative offset
                let time = Utc.timestamp_opt(-offset, 0).unwrap();
                let dt = parse_datetime(format!("@-{}", offset));
                assert_eq!(dt.unwrap(), time);
            }
        }
    }

    /// Used to test example code presented in the README.
    mod readme_test {
        use crate::parse_datetime;
        use chrono::{Local, TimeZone};

        #[test]
        fn test_readme_code() {
            let dt = parse_datetime("2021-02-14 06:37:47");
            assert_eq!(
                dt.unwrap(),
                Local.with_ymd_and_hms(2021, 2, 14, 6, 37, 47).unwrap()
            );
        }
    }

    mod invalid_test {
        use crate::parse_datetime;
        use crate::ParseDateTimeError;

        #[test]
        fn test_invalid_input() {
            let result = parse_datetime("foobar");
            assert_eq!(result, Err(ParseDateTimeError::InvalidInput));

            let result = parse_datetime("invalid 1");
            assert_eq!(result, Err(ParseDateTimeError::InvalidInput));
        }
    }
}

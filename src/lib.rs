// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! A Rust crate for parsing human-readable relative time strings and human-readable datetime strings and converting them to a `DateTime`.
//! The function supports the following formats for time:
//!
//! * ISO formats
//! * timezone offsets, e.g., "UTC-0100"
//! * unix timestamps, e.g., "@12"
//! * relative time to now, e.g. "+1 hour"
//!
use regex::Error as RegexError;
use std::error::Error;
use std::fmt::{self, Display};

// Expose parse_datetime
mod parse_relative_time;
mod parse_timestamp;

mod parse_time_only_str;
mod parse_weekday;

use chrono::{
    DateTime, Datelike, Duration, FixedOffset, Local, LocalResult, NaiveDateTime, TimeZone,
    Timelike,
};

use parse_relative_time::parse_relative_time;
use parse_timestamp::parse_timestamp;

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

/// Parses a time string and returns a `DateTime` representing the
/// absolute time of the string.
///
/// # Arguments
///
/// * `s` - A string slice representing the time.
///
/// # Examples
///
/// ```
/// use chrono::{DateTime, Utc, TimeZone};
/// let time = parse_datetime::parse_datetime("2023-06-03 12:00:01Z");
/// assert_eq!(time.unwrap(), Utc.with_ymd_and_hms(2023, 06, 03, 12, 00, 01).unwrap());
/// ```
///
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
pub fn parse_datetime<S: AsRef<str> + Clone>(
    s: S,
) -> Result<DateTime<FixedOffset>, ParseDateTimeError> {
    parse_datetime_at_date(Local::now(), s)
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
    // TODO: Replace with a proper customiseable parsing solution using `nom`, `grmtools`, or
    // similar

    // Formats with offsets don't require NaiveDateTime workaround
    for fmt in [
        format::YYYYMMDDHHMM_OFFSET,
        format::YYYYMMDDHHMM_HYPHENATED_OFFSET,
        format::YYYYMMDDHHMM_UTC_OFFSET,
        format::YYYYMMDDHHMM_ZULU_OFFSET,
    ] {
        if let Ok(parsed) = DateTime::parse_from_str(s.as_ref(), fmt) {
            return Ok(parsed);
        }
    }

    // Parse formats with no offset, assume local time
    for fmt in [
        format::YYYYMMDDHHMMS_T_SEP,
        format::YYYYMMDDHHMM,
        format::YYYYMMDDHHMMS,
        format::YYYYMMDDHHMMSS,
        format::YYYY_MM_DD_HH_MM,
        format::YYYYMMDDHHMM_DOT_SS,
        format::POSIX_LOCALE,
    ] {
        if let Ok(parsed) = NaiveDateTime::parse_from_str(s.as_ref(), fmt) {
            if let Ok(dt) = naive_dt_to_fixed_offset(date, parsed) {
                return Ok(dt);
            }
        }
    }

    // parse weekday
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

        let dt = DateTime::<FixedOffset>::from(beginning_of_day);

        return Ok(dt);
    }

    // Parse epoch seconds
    if let Ok(timestamp) = parse_timestamp(s.as_ref()) {
        if let Some(timestamp_date) = NaiveDateTime::from_timestamp_opt(timestamp, 0) {
            return Ok(date.offset().from_utc_datetime(&timestamp_date));
        }
    }

    let ts = s.as_ref().to_owned() + " 0000";
    // Parse date only formats - assume midnight local timezone
    for fmt in [format::ISO_8601, format::ISO_8601_NO_SEP] {
        let f = fmt.to_owned() + " %H%M";
        if let Ok(parsed) = NaiveDateTime::parse_from_str(&ts, &f) {
            if let Ok(dt) = naive_dt_to_fixed_offset(date, parsed) {
                return Ok(dt);
            }
        }
    }

    // Parse offsets. chrono doesn't provide any functionality to parse
    // offsets, so instead we replicate parse_date behaviour by getting
    // the current date with local, and create a date time string at midnight,
    // before trying offset suffixes
    let ts = format!("{}", date.format("%Y%m%d")) + "0000" + s.as_ref();
    for fmt in [format::UTC_OFFSET, format::ZULU_OFFSET] {
        let f = format::YYYYMMDDHHMM.to_owned() + fmt;
        if let Ok(parsed) = DateTime::parse_from_str(&ts, &f) {
            return Ok(parsed);
        }
    }

    // Parse relative time.
    if let Ok(relative_time) = parse_relative_time(s.as_ref()) {
        let current_time = DateTime::<FixedOffset>::from(date);

        if let Some(date_time) = current_time.checked_add_signed(relative_time) {
            return Ok(date_time);
        }
    }

    // parse time only dates
    if let Some(date_time) = parse_time_only_str::parse_time_only(date, s.as_ref()) {
        return Ok(date_time);
    }

    // Default parse and failure
    s.as_ref()
        .parse()
        .map_err(|_| (ParseDateTimeError::InvalidInput))
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

        #[test]
        fn test_epoch_seconds_non_utc() {
            env::set_var("TZ", "EST");
            let dt = "@1613371067";
            let actual = parse_datetime(dt);
            assert_eq!(actual.unwrap().timestamp(), TEST_TIME);
        }
    }

    #[cfg(test)]
    mod formats {
        use crate::parse_datetime;
        use chrono::{DateTime, Local, TimeZone};

        #[test]
        fn single_digit_month_day() {
            let x = Local.with_ymd_and_hms(1987, 5, 7, 0, 0, 0).unwrap();
            let expected = DateTime::fixed_offset(&x);

            assert_eq!(Ok(expected), parse_datetime("1987-05-07"));
            assert_eq!(Ok(expected), parse_datetime("1987-5-07"));
            assert_eq!(Ok(expected), parse_datetime("1987-05-7"));
            assert_eq!(Ok(expected), parse_datetime("1987-5-7"));
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
                assert!(parse_datetime(relative_time).is_ok());
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
            let date = Local.with_ymd_and_hms(2023, 2, 28, 10, 12, 3).unwrap();

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
        use crate::parse_datetime;
        use chrono::{TimeZone, Utc};

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

    #[cfg(test)]
    mod timeonly {
        use crate::parse_datetime_at_date;
        use chrono::{Local, TimeZone};
        use std::env;
        #[test]
        fn test_time_only() {
            env::set_var("TZ", "UTC");
            let test_date = Local.with_ymd_and_hms(2024, 3, 3, 0, 0, 0).unwrap();
            let parsed_time = parse_datetime_at_date(test_date, "9:04:30 PM +0530")
                .unwrap()
                .timestamp();
            assert_eq!(parsed_time, 1709480070)
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
            println!("{result:?}");
            assert_eq!(result, Err(ParseDateTimeError::InvalidInput));

            let result = parse_datetime("invalid 1");
            assert_eq!(result, Err(ParseDateTimeError::InvalidInput));
        }
    }
}

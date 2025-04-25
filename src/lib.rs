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
use regex::Regex;
use std::error::Error;
use std::fmt::{self, Display};

pub(crate) mod parse;

// Expose parse_datetime
mod parse_relative_time;
mod parse_timestamp;

mod parse_time_only_str;
mod parse_weekday;

use chrono::{
    DateTime, FixedOffset, Local, LocalResult, MappedLocalTime, NaiveDate, NaiveDateTime, TimeZone,
};

use parse_relative_time::parse_relative_time_at_date;
use parse_timestamp::parse_timestamp;
use parse_weekday::parse_weekday_at_date;

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
    // US format for calendar date items:
    // https://www.gnu.org/software/coreutils/manual/html_node/Calendar-date-items.html
    pub const MMDDYYYY_SLASH: &str = "%m/%d/%Y";
    pub const MMDDYY_SLASH: &str = "%m/%d/%y";
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
    pub const YYYYMMDDHHMMSS_HYPHENATED_OFFSET: &str = "%Y-%m-%d %H:%M:%S %#z";
    pub const YYYYMMDDHHMMSS_HYPHENATED_ZULU: &str = "%Y-%m-%d %H:%M:%SZ";
    pub const YYYYMMDDHHMMSS_T_SEP_HYPHENATED_OFFSET: &str = "%Y-%m-%dT%H:%M:%S%#z";
    pub const YYYYMMDDHHMMSS_T_SEP_HYPHENATED_ZULU: &str = "%Y-%m-%dT%H:%M:%SZ";
    pub const YYYYMMDDHHMMSS_T_SEP_HYPHENATED_SPACE_OFFSET: &str = "%Y-%m-%dT%H:%M:%S %#z";
    pub const YYYYMMDDHHMMS_T_SEP: &str = "%Y-%m-%dT%H:%M:%S";
    pub const UTC_OFFSET: &str = "UTC%#z";
    pub const ZULU_OFFSET: &str = "Z%#z";
    pub const NAKED_OFFSET: &str = "%#z";

    /// Whether the pattern ends in the character `Z`.
    pub(crate) fn is_zulu(pattern: &str) -> bool {
        pattern.ends_with('Z')
    }

    /// Patterns for datetimes with timezones.
    ///
    /// These are in decreasing order of length. The same pattern may
    /// appear multiple times with different lengths if the pattern
    /// accepts input strings of different lengths. For example, the
    /// specifier `%#z` accepts two-digit time zone offsets (`+00`)
    /// and four-digit time zone offsets (`+0000`).
    pub(crate) const PATTERNS_TZ: [(&str, usize); 9] = [
        (YYYYMMDDHHMMSS_HYPHENATED_OFFSET, 25),
        (YYYYMMDDHHMMSS_T_SEP_HYPHENATED_SPACE_OFFSET, 25),
        (YYYYMMDDHHMMSS_T_SEP_HYPHENATED_OFFSET, 24),
        (YYYYMMDDHHMMSS_HYPHENATED_OFFSET, 23),
        (YYYYMMDDHHMMSS_T_SEP_HYPHENATED_OFFSET, 22),
        (YYYYMMDDHHMM_HYPHENATED_OFFSET, 22),
        (YYYYMMDDHHMM_UTC_OFFSET, 20),
        (YYYYMMDDHHMM_OFFSET, 18),
        (YYYYMMDDHHMM_ZULU_OFFSET, 18),
    ];

    /// Patterns for datetimes without timezones.
    ///
    /// These are in decreasing order of length.
    pub(crate) const PATTERNS_NO_TZ: [(&str, usize); 9] = [
        (YYYYMMDDHHMMSS, 29),
        (POSIX_LOCALE, 24),
        (YYYYMMDDHHMMSS_HYPHENATED_ZULU, 20),
        (YYYYMMDDHHMMSS_T_SEP_HYPHENATED_ZULU, 20),
        (YYYYMMDDHHMMS_T_SEP, 19),
        (YYYYMMDDHHMMS, 19),
        (YYYY_MM_DD_HH_MM, 16),
        (YYYYMMDDHHMM_DOT_SS, 15),
        (YYYYMMDDHHMM, 12),
    ];

    /// Patterns for dates with neither times nor timezones.
    ///
    /// These are in decreasing order of length. The same pattern may
    /// appear multiple times with different lengths if the pattern
    /// accepts input strings of different lengths. For example, the
    /// specifier `%m` accepts one-digit month numbers (like `2`) and
    /// two-digit month numbers (like `02` or `12`).
    pub(crate) const PATTERNS_DATE_NO_TZ: [(&str, usize); 8] = [
        (ISO_8601, 10),
        (MMDDYYYY_SLASH, 10),
        (ISO_8601, 9),
        (MMDDYYYY_SLASH, 9),
        (ISO_8601, 8),
        (MMDDYY_SLASH, 8),
        (MMDDYYYY_SLASH, 8),
        (ISO_8601_NO_SEP, 8),
    ];

    /// Patterns for lone timezone offsets.
    ///
    /// These are in decreasing order of length. The same pattern may
    /// appear multiple times with different lengths if the pattern
    /// accepts input strings of different lengths. For example, the
    /// specifier `%#z` accepts two-digit time zone offsets (`+00`)
    /// and four-digit time zone offsets (`+0000`).
    pub(crate) const PATTERNS_OFFSET: [(&str, usize); 9] = [
        (UTC_OFFSET, 9),
        (UTC_OFFSET, 8),
        (ZULU_OFFSET, 7),
        (UTC_OFFSET, 6),
        (ZULU_OFFSET, 6),
        (NAKED_OFFSET, 6),
        (NAKED_OFFSET, 5),
        (ZULU_OFFSET, 4),
        (NAKED_OFFSET, 3),
    ];
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

    // Try to parse a reference date first. Try parsing from longest
    // pattern to shortest pattern. If a reference date can be parsed,
    // then try to parse a time delta from the remaining slice. If no
    // reference date could be parsed, then try to parse the entire
    // string as a time delta. If no time delta could be parsed,
    // return an error.
    let (ref_date, n) = if let Some((ref_date, n)) = parse_reference_date(date, s.as_ref()) {
        (ref_date, n)
    } else {
        let tz = TimeZone::from_offset(date.offset());
        match date.naive_local().and_local_timezone(tz) {
            MappedLocalTime::Single(ref_date) => (ref_date, 0),
            _ => return Err(ParseDateTimeError::InvalidInput),
        }
    };
    parse_relative_time_at_date(ref_date, &s.as_ref()[n..])
}

/// Parse an absolute datetime from a prefix of s, if possible.
///
/// Try to parse the longest possible absolute datetime at the beginning
/// of string `s`. Return the parsed datetime and the index in `s` at
/// which the datetime ended.
fn parse_reference_date<S>(date: DateTime<Local>, s: S) -> Option<(DateTime<FixedOffset>, usize)>
where
    S: AsRef<str>,
{
    // HACK: if the string ends with a single digit preceded by a + or -
    // sign, then insert a 0 between the sign and the digit to make it
    // possible for `chrono` to parse it.
    let pattern = Regex::new(r"([\+-])(\d)$").unwrap();
    let tmp_s = pattern.replace(s.as_ref(), "${1}0${2}");
    for (fmt, n) in format::PATTERNS_TZ {
        if tmp_s.len() >= n {
            if let Ok(parsed) = DateTime::parse_from_str(&tmp_s[0..n], fmt) {
                if tmp_s == s.as_ref() {
                    return Some((parsed, n));
                }
                return Some((parsed, n - 1));
            }
        }
    }

    // Parse formats with no offset, assume local time
    for (fmt, n) in format::PATTERNS_NO_TZ {
        if s.as_ref().len() >= n {
            if let Ok(parsed) = NaiveDateTime::parse_from_str(&s.as_ref()[0..n], fmt) {
                // Special case: `chrono` can only parse a datetime like
                // `2000-01-01 01:23:45Z` as a naive datetime, so we
                // manually force it to be in UTC.
                if format::is_zulu(fmt) {
                    match FixedOffset::east_opt(0)
                        .unwrap()
                        .from_local_datetime(&parsed)
                    {
                        MappedLocalTime::Single(datetime) => return Some((datetime, n)),
                        _ => return None,
                    }
                } else if let Ok(dt) = naive_dt_to_fixed_offset(date, parsed) {
                    return Some((dt, n));
                }
            }
        }
    }

    // parse weekday
    if let Ok(dt) = parse_weekday_at_date(date, s.as_ref()) {
        let dt = DateTime::<FixedOffset>::from(dt);
        return Some((dt, s.as_ref().len()));
    }

    // Parse epoch seconds
    if let Ok(timestamp) = parse_timestamp(s.as_ref()) {
        if let Some(timestamp_date) = DateTime::from_timestamp(timestamp, 0) {
            return Some((timestamp_date.into(), s.as_ref().len()));
        }
    }

    // Parse date only formats - assume midnight local timezone
    for (fmt, n) in format::PATTERNS_DATE_NO_TZ {
        if s.as_ref().len() >= n {
            if let Ok(parsed) = NaiveDate::parse_from_str(&s.as_ref()[0..n], fmt) {
                let datetime = parsed.and_hms_opt(0, 0, 0).unwrap();
                if let Ok(dt) = naive_dt_to_fixed_offset(date, datetime) {
                    return Some((dt, n));
                }
            }
        }
    }

    // Parse offsets. chrono doesn't provide any functionality to parse
    // offsets, so instead we replicate parse_date behaviour by getting
    // the current date with local, and create a date time string at midnight,
    // before trying offset suffixes
    let ts = format!("{}0000{}", date.format("%Y%m%d"), tmp_s.as_ref());
    for (fmt, n) in format::PATTERNS_OFFSET {
        if ts.len() == n + 12 {
            let f = format::YYYYMMDDHHMM.to_owned() + fmt;
            if let Ok(parsed) = DateTime::parse_from_str(&ts, &f) {
                if tmp_s == s.as_ref() {
                    return Some((parsed, n));
                }
                return Some((parsed, n - 1));
            }
        }
    }

    // parse time only dates
    if let Some(date_time) = parse_time_only_str::parse_time_only(date, s.as_ref()) {
        return Some((date_time, s.as_ref().len()));
    }

    None
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
        fn test_t_sep_single_digit_offset_no_space() {
            env::set_var("TZ", "UTC");
            let dt = "2021-02-14T22:37:47-8";
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
    mod calendar_date_items {
        use crate::parse_datetime;
        use chrono::{DateTime, Local, TimeZone};

        #[test]
        fn single_digit_month_day() {
            std::env::set_var("TZ", "UTC");
            let x = Local.with_ymd_and_hms(1987, 5, 7, 0, 0, 0).unwrap();
            let expected = DateTime::fixed_offset(&x);

            assert_eq!(Ok(expected), parse_datetime("1987-05-07"));
            assert_eq!(Ok(expected), parse_datetime("1987-5-07"));
            assert_eq!(Ok(expected), parse_datetime("1987-05-7"));
            assert_eq!(Ok(expected), parse_datetime("1987-5-7"));
            assert_eq!(Ok(expected), parse_datetime("5/7/1987"));
            assert_eq!(Ok(expected), parse_datetime("5/07/1987"));
            assert_eq!(Ok(expected), parse_datetime("05/7/1987"));
            assert_eq!(Ok(expected), parse_datetime("05/07/1987"));
        }
    }

    #[cfg(test)]
    mod offsets {
        use chrono::{Local, NaiveDate};

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
                "+07",
                "+7",
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
            let offset = "UTC+01005";
            assert_eq!(
                parse_datetime(offset),
                Err(ParseDateTimeError::InvalidInput)
            );
        }

        #[test]
        fn test_datetime_with_offset() {
            let actual = parse_datetime("1997-01-19 08:17:48 +0").unwrap();
            let expected = NaiveDate::from_ymd_opt(1997, 1, 19)
                .unwrap()
                .and_hms_opt(8, 17, 48)
                .unwrap()
                .and_utc();
            assert_eq!(actual, expected);
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

            result.format("%F %T %f").to_string()
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
                let dt = parse_datetime(format!("@{offset}"));
                assert_eq!(dt.unwrap(), time);

                // negative offset
                let time = Utc.timestamp_opt(-offset, 0).unwrap();
                let dt = parse_datetime(format!("@-{offset}"));
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
            assert_eq!(parsed_time, 1709480070);
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

    #[test]
    fn test_datetime_ending_in_z() {
        use crate::parse_datetime;
        use chrono::{TimeZone, Utc};

        let actual = parse_datetime("2023-06-03 12:00:01Z").unwrap();
        let expected = Utc.with_ymd_and_hms(2023, 6, 3, 12, 0, 1).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_invalid_datetime() {
        assert!(crate::parse_datetime("bogus +1 day").is_err());
    }

    #[test]
    fn test_parse_invalid_delta() {
        assert!(crate::parse_datetime("1997-01-01 bogus").is_err());
    }

    #[test]
    fn test_parse_datetime_tz_nodelta() {
        std::env::set_var("TZ", "UTC0");

        // 1997-01-01 00:00:00 +0000
        let expected = chrono::NaiveDate::from_ymd_opt(1997, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .fixed_offset();

        for s in [
            "1997-01-01 00:00:00 +0000",
            "1997-01-01 00:00:00 +00",
            "199701010000 +0000",
            "199701010000UTC+0000",
            "199701010000Z+0000",
            "1997-01-01 00:00 +0000",
            "1997-01-01 00:00:00 +0000",
            "1997-01-01T00:00:00+0000",
            "1997-01-01T00:00:00+00",
            "1997-01-01T00:00:00Z",
            "@852076800",
        ] {
            let actual = crate::parse_datetime(s).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_parse_datetime_notz_nodelta() {
        std::env::set_var("TZ", "UTC0");
        let expected = chrono::NaiveDate::from_ymd_opt(1997, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .fixed_offset();

        for s in [
            "1997-01-01 00:00:00.000000000",
            "Wed Jan  1 00:00:00 1997",
            "1997-01-01T00:00:00",
            "1997-01-01 00:00:00",
            "1997-01-01 00:00",
            "199701010000.00",
            "199701010000",
        ] {
            let actual = crate::parse_datetime(s).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_parse_date_notz_nodelta() {
        std::env::set_var("TZ", "UTC0");
        let expected = chrono::NaiveDate::from_ymd_opt(1997, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .fixed_offset();

        for s in ["1997-01-01", "19970101", "01/01/1997", "01/01/97"] {
            let actual = crate::parse_datetime(s).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_parse_datetime_tz_delta() {
        std::env::set_var("TZ", "UTC0");

        // 1998-01-01
        let expected = chrono::NaiveDate::from_ymd_opt(1998, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .fixed_offset();

        for s in [
            "1997-01-01 00:00:00 +0000 +1 year",
            "1997-01-01 00:00:00 +00 +1 year",
            "199701010000 +0000 +1 year",
            "199701010000UTC+0000 +1 year",
            "199701010000Z+0000 +1 year",
            "1997-01-01T00:00:00Z +1 year",
            "1997-01-01 00:00 +0000 +1 year",
            "1997-01-01 00:00:00 +0000 +1 year",
            "1997-01-01T00:00:00+0000 +1 year",
            "1997-01-01T00:00:00+00 +1 year",
        ] {
            let actual = crate::parse_datetime(s).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_parse_datetime_notz_delta() {
        std::env::set_var("TZ", "UTC0");
        let expected = chrono::NaiveDate::from_ymd_opt(1998, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .fixed_offset();

        for s in [
            "1997-01-01 00:00:00.000000000 +1 year",
            "Wed Jan  1 00:00:00 1997 +1 year",
            "1997-01-01T00:00:00 +1 year",
            "1997-01-01 00:00:00 +1 year",
            "1997-01-01 00:00 +1 year",
            "199701010000.00 +1 year",
            "199701010000 +1 year",
        ] {
            let actual = crate::parse_datetime(s).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_parse_date_notz_delta() {
        std::env::set_var("TZ", "UTC0");
        let expected = chrono::NaiveDate::from_ymd_opt(1998, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .fixed_offset();

        for s in [
            "1997-01-01 +1 year",
            "19970101 +1 year",
            "01/01/1997 +1 year",
            "01/01/97 +1 year",
        ] {
            let actual = crate::parse_datetime(s).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_time_only() {
        use chrono::{FixedOffset, Local};
        std::env::set_var("TZ", "UTC");

        let offset = FixedOffset::east_opt(5 * 60 * 60 + 1800).unwrap();
        let expected = Local::now()
            .date_naive()
            .and_hms_opt(21, 4, 30)
            .unwrap()
            .and_local_timezone(offset)
            .unwrap();
        let actual = crate::parse_datetime("9:04:30 PM +0530").unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_weekday_only() {
        use chrono::{Datelike, Days, Local, MappedLocalTime, NaiveTime, Weekday};
        std::env::set_var("TZ", "UTC0");
        let now = Local::now();
        let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        let today = now.weekday();
        let midnight_today = if let MappedLocalTime::Single(t) = now.with_time(midnight) {
            t
        } else {
            panic!()
        };

        for (s, day) in [
            ("sunday", Weekday::Sun),
            ("monday", Weekday::Mon),
            ("tuesday", Weekday::Tue),
            ("wednesday", Weekday::Wed),
            ("thursday", Weekday::Thu),
            ("friday", Weekday::Fri),
            ("saturday", Weekday::Sat),
        ] {
            let actual = crate::parse_datetime(s).unwrap();
            let delta = Days::new(u64::from(day.days_since(today)));
            let expected = midnight_today.checked_add_days(delta).unwrap();
            assert_eq!(actual, expected);
        }
    }
}

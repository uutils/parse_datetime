// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use chrono::{DateTime, FixedOffset, Local, LocalResult, NaiveDateTime, TimeZone};

use crate::ParseDurationError;

/// Formats that parse input can take.
/// Taken from `touch` coreutils
mod format {
    pub(crate) const ISO_8601: &str = "%Y-%m-%d";
    pub(crate) const ISO_8601_NO_SEP: &str = "%Y%m%d";
    pub(crate) const POSIX_LOCALE: &str = "%a %b %e %H:%M:%S %Y";
    pub(crate) const YYYYMMDDHHMM_DOT_SS: &str = "%Y%m%d%H%M.%S";
    pub(crate) const YYYYMMDDHHMMSS: &str = "%Y-%m-%d %H:%M:%S.%f";
    pub(crate) const YYYYMMDDHHMMS: &str = "%Y-%m-%d %H:%M:%S";
    pub(crate) const YYYY_MM_DD_HH_MM: &str = "%Y-%m-%d %H:%M";
    pub(crate) const YYYYMMDDHHMM: &str = "%Y%m%d%H%M";
    pub(crate) const YYYYMMDDHHMM_OFFSET: &str = "%Y%m%d%H%M %z";
    pub(crate) const YYYYMMDDHHMM_UTC_OFFSET: &str = "%Y%m%d%H%MUTC%z";
    pub(crate) const YYYYMMDDHHMM_ZULU_OFFSET: &str = "%Y%m%d%H%MZ%z";
    pub(crate) const YYYYMMDDHHMM_HYPHENATED_OFFSET: &str = "%Y-%m-%d %H:%M %z";
    pub(crate) const YYYYMMDDHHMMS_T_SEP: &str = "%Y-%m-%dT%H:%M:%S";
    pub(crate) const UTC_OFFSET: &str = "UTC%#z";
    pub(crate) const ZULU_OFFSET: &str = "Z%#z";
}

/// Loosely parses a time string and returns a `DateTime` representing the
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
/// let time = parse_datetime::parse_datetime::from_str("2023-06-03 12:00:01Z");
/// assert_eq!(time.unwrap(), Utc.with_ymd_and_hms(2023, 06, 03, 12, 00, 01).unwrap());
/// ```
///
/// # Supported formats
///
/// The function supports the following formats for time:
///
/// * ISO formats
/// * timezone offsets, e.g., "UTC-0100"
///
/// # Returns
///
/// * `Ok(DateTime<FixedOffset>)` - If the input string can be parsed as a time
/// * `Err(ParseDurationError)` - If the input string cannot be parsed as a relative time
///
/// # Errors
///
/// This function will return `Err(ParseDurationError::InvalidInput)` if the input string
/// cannot be parsed as a relative time.
///
pub fn from_str<S: AsRef<str> + Clone>(s: S) -> Result<DateTime<FixedOffset>, ParseDurationError> {
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
            if let Ok(dt) = naive_dt_to_fixed_offset(parsed) {
                return Ok(dt);
            }
        }
    }

    // Parse epoch seconds
    if s.as_ref().bytes().next() == Some(b'@') {
        if let Ok(parsed) = NaiveDateTime::parse_from_str(&s.as_ref()[1..], "%s") {
            if let Ok(dt) = naive_dt_to_fixed_offset(parsed) {
                return Ok(dt);
            }
        }
    }

    let ts = s.as_ref().to_owned() + "0000";
    // Parse date only formats - assume midnight local timezone
    for fmt in [format::ISO_8601, format::ISO_8601_NO_SEP] {
        let f = fmt.to_owned() + "%H%M";
        if let Ok(parsed) = NaiveDateTime::parse_from_str(&ts, &f) {
            if let Ok(dt) = naive_dt_to_fixed_offset(parsed) {
                return Ok(dt);
            }
        }
    }

    // Parse offsets. chrono doesn't provide any functionality to parse
    // offsets, so instead we replicate parse_date behaviour by getting
    // the current date with local, and create a date time string at midnight,
    // before trying offset suffixes
    let local = Local::now();
    let ts = format!("{}", local.format("%Y%m%d")) + "0000" + s.as_ref();
    for fmt in [format::UTC_OFFSET, format::ZULU_OFFSET] {
        let f = format::YYYYMMDDHHMM.to_owned() + fmt;
        if let Ok(parsed) = DateTime::parse_from_str(&ts, &f) {
            return Ok(parsed);
        }
    }

    // Default parse and failure
    s.as_ref()
        .parse()
        .map_err(|_| (ParseDurationError::InvalidInput))
}

// Convert NaiveDateTime to DateTime<FixedOffset> by assuming the offset
// is local time
fn naive_dt_to_fixed_offset(dt: NaiveDateTime) -> Result<DateTime<FixedOffset>, ()> {
    let now = Local::now();
    match now.offset().from_local_datetime(&dt) {
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

        use crate::{
            parse_datetime::from_str, parse_datetime::tests::TEST_TIME, ParseDurationError,
        };

        #[test]
        fn test_t_sep() {
            env::set_var("TZ", "UTC");
            let dt = "2021-02-15T06:37:47";
            let actual = from_str(dt);
            assert_eq!(actual.unwrap().timestamp(), TEST_TIME);
        }

        #[test]
        fn test_space_sep() {
            env::set_var("TZ", "UTC");
            let dt = "2021-02-15 06:37:47";
            let actual = from_str(dt);
            assert_eq!(actual.unwrap().timestamp(), TEST_TIME);
        }

        #[test]
        fn test_space_sep_offset() {
            env::set_var("TZ", "UTC");
            let dt = "2021-02-14 22:37:47 -0800";
            let actual = from_str(dt);
            assert_eq!(actual.unwrap().timestamp(), TEST_TIME);
        }

        #[test]
        fn test_t_sep_offset() {
            env::set_var("TZ", "UTC");
            let dt = "2021-02-14T22:37:47 -0800";
            let actual = from_str(dt);
            assert_eq!(actual.unwrap().timestamp(), TEST_TIME);
        }

        #[test]
        fn invalid_formats() {
            let invalid_dts = vec!["NotADate", "202104", "202104-12T22:37:47"];
            for dt in invalid_dts {
                assert_eq!(from_str(dt), Err(ParseDurationError::InvalidInput));
            }
        }
    }

    #[cfg(test)]
    mod offsets {
        use chrono::Local;

        use crate::{parse_datetime::from_str, ParseDurationError};

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
                let actual = from_str(offset).unwrap();
                assert_eq!(expected, format!("{}", actual.format("%Y%m%d%H%M%z")));
            }
        }

        #[test]
        fn test_partial_offset() {
            let offsets = vec!["UTC+00:15", "UTC+0015", "Z+00:15", "Z+0015"];
            let expected = format!("{}{}", Local::now().format("%Y%m%d"), "0000+0015");
            for offset in offsets {
                let actual = from_str(offset).unwrap();
                assert_eq!(expected, format!("{}", actual.format("%Y%m%d%H%M%z")));
            }
        }

        #[test]
        fn invalid_offset_format() {
            let invalid_offsets = vec!["+0700", "UTC+2", "Z-1", "UTC+01005"];
            for offset in invalid_offsets {
                assert_eq!(from_str(offset), Err(ParseDurationError::InvalidInput));
            }
        }
    }

    /// Used to test example code presented in the README.
    mod readme_test {
        use crate::parse_datetime::from_str;
        use chrono::{Local, TimeZone};

        #[test]
        fn test_readme_code() {
            let dt = from_str("2021-02-14 06:37:47");
            assert_eq!(
                dt.unwrap(),
                Local.with_ymd_and_hms(2021, 2, 14, 6, 37, 47).unwrap()
            );
        }
    }
}

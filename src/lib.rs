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
use std::error::Error;
use std::fmt::{self, Display};

use items::Item;

mod items;

#[derive(Debug, PartialEq)]
pub enum ParseDateTimeError {
    InvalidInput,
}

impl Display for ParseDateTimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseDateTimeError::InvalidInput => {
                write!(
                    f,
                    "Invalid input string: cannot be parsed as a relative time"
                )
            }
        }
    }
}

impl Error for ParseDateTimeError {}

pub fn parse_datetime(input: &str) -> Result<Vec<Item>, ParseDateTimeError> {
    let input = input.to_ascii_lowercase();
    match items::parse(&mut input.as_ref()) {
        Some(x) => Ok(x),
        None => Err(ParseDateTimeError::InvalidInput),
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

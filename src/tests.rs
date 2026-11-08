// For the full copyright and license information, view the LICENSE file.

use jiff::{
    civil::{date, time, Time, Weekday},
    ToSpan, Zoned,
};

use crate::parse_datetime;

#[cfg(test)]
mod iso_8601 {
    use crate::parse_datetime;

    static TEST_TIME: i64 = 1613371067;

    #[test]
    fn test_t_sep() {
        let dt = "2021-02-15T06:37:47 +0000";
        let actual = parse_datetime(dt).unwrap().expect_in_range();
        assert_eq!(actual.timestamp().as_second(), TEST_TIME);
    }

    #[test]
    fn test_space_sep() {
        let dt = "2021-02-15 06:37:47 +0000";
        let actual = parse_datetime(dt).unwrap().expect_in_range();
        assert_eq!(actual.timestamp().as_second(), TEST_TIME);
    }

    #[test]
    fn test_space_sep_offset() {
        let dt = "2021-02-14 22:37:47 -0800";
        let actual = parse_datetime(dt).unwrap().expect_in_range();
        assert_eq!(actual.timestamp().as_second(), TEST_TIME);
    }

    #[test]
    fn test_t_sep_offset() {
        let dt = "2021-02-14T22:37:47 -0800";
        let actual = parse_datetime(dt).unwrap().expect_in_range();
        assert_eq!(actual.timestamp().as_second(), TEST_TIME);
    }

    #[test]
    fn test_t_sep_single_digit_offset_no_space() {
        let dt = "2021-02-14T22:37:47-8";
        let actual = parse_datetime(dt).unwrap().expect_in_range();
        assert_eq!(actual.timestamp().as_second(), TEST_TIME);
    }

    #[test]
    fn invalid_formats() {
        let invalid_dts = vec![
            "NotADate",
            "202104",
            "202104-12T22:37:47",
            "a774e26sec", // 774e26 is not a valid seconds value (we don't accept E-notation)
            "12.",        // Invalid floating point number
        ];
        for dt in invalid_dts {
            assert!(
                parse_datetime(dt).is_err(),
                "Expected error for input: {}",
                dt
            );
        }
    }

    #[test]
    fn test_epoch_seconds() {
        let dt = "@1613371067";
        let actual = parse_datetime(dt).unwrap().expect_in_range();
        assert_eq!(actual.timestamp().as_second(), TEST_TIME);
    }

    // #[test]
    // fn test_epoch_seconds_non_utc() {
    //     env::set_var("TZ", "EST");
    //     let dt = "@1613371067";
    //     let actual = parse_datetime(dt).unwrap();
    //     assert_eq!(actual.timestamp().as_second(), TEST_TIME);
    // }
}

#[cfg(test)]
mod calendar_date_items {
    use jiff::{
        civil::{date, time},
        Zoned,
    };

    use crate::parse_datetime;

    #[test]
    fn single_digit_month_day() {
        let expected = Zoned::now()
            .with()
            .date(date(1987, 5, 7))
            .time(time(0, 0, 0, 0))
            .build()
            .unwrap();

        assert_eq!(parse_datetime("1987-05-07").unwrap(), expected);
        assert_eq!(parse_datetime("1987-5-07").unwrap(), expected);
        assert_eq!(parse_datetime("1987-05-7").unwrap(), expected);
        assert_eq!(parse_datetime("1987-5-7").unwrap(), expected);
        assert_eq!(parse_datetime("5/7/1987").unwrap(), expected);
        assert_eq!(parse_datetime("5/07/1987").unwrap(), expected);
        assert_eq!(parse_datetime("05/7/1987").unwrap(), expected);
        assert_eq!(parse_datetime("05/07/1987").unwrap(), expected);
    }
}

#[cfg(test)]
mod offsets {
    use jiff::{civil::DateTime, tz, Zoned};

    use crate::parse_datetime;

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

        let expected = format!("{}{}", Zoned::now().strftime("%Y%m%d"), "0000+0700");
        for offset in offsets {
            let actual = parse_datetime(offset).unwrap().expect_in_range();
            assert_eq!(expected, actual.strftime("%Y%m%d%H%M%z").to_string());
        }
    }

    #[test]
    fn test_partial_offset() {
        let offsets = vec!["UTC+00:15", "UTC+0015", "Z+00:15", "Z+0015"];
        let expected = format!("{}{}", Zoned::now().strftime("%Y%m%d"), "0000+0015");
        for offset in offsets {
            let actual = parse_datetime(offset).unwrap().expect_in_range();
            assert_eq!(expected, actual.strftime("%Y%m%d%H%M%z").to_string());
        }
    }

    #[test]
    fn test_datetime_with_offset() {
        let actual = parse_datetime("1997-01-19 08:17:48 +2").unwrap();
        let expected = "1997-01-19 08:17:48"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(tz::TimeZone::fixed(tz::offset(2)))
            .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_datetime_with_timezone() {
        let actual = parse_datetime("1997-01-19 08:17:48 BRT").unwrap();
        let expected = "1997-01-19 08:17:48"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(tz::TimeZone::fixed(tz::offset(-3)))
            .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn offset_overflow() {
        assert!(parse_datetime("m+25").is_err());
        assert!(parse_datetime("24:00").is_err());
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

    #[test]
    fn integer_seconds_offset() {
        let dt = "0 + 0 seconds";
        assert!(parse_datetime(dt).is_ok());
    }

    #[test]
    fn integer_seconds_offset_spaceless() {
        let dt = "0+0 seconds";
        assert!(parse_datetime(dt).is_ok());
    }

    #[test]
    fn floating_seconds_offset() {
        let dt = "0 + 0.0 seconds";
        assert!(parse_datetime(dt).is_ok());
    }

    #[test]
    fn floating_seconds_offset_spaceless() {
        let dt = "0+0.0 seconds";
        assert!(parse_datetime(dt).is_ok());
    }

    #[test]
    fn floating_seconds_offset_from_now() {
        let dt = "now  +  1.5 seconds";
        assert!(parse_datetime(dt).is_ok());
    }

    #[test]
    fn floating_seconds_offset_from_tomorrow_spaceless() {
        let dt = "tomorrow+1.5 seconds";
        assert!(parse_datetime(dt).is_ok());
    }
}

#[cfg(test)]
mod weekday {
    use jiff::{civil::DateTime, tz::TimeZone, Zoned};

    use crate::parse_datetime_at_date;

    fn get_formatted_date(date: &Zoned, weekday: &str) -> String {
        let result = parse_datetime_at_date(date.clone(), weekday)
            .unwrap()
            .expect_in_range();

        result.strftime("%F %T %9f").to_string()
    }

    #[test]
    fn test_weekday() {
        // add some constant hours and minutes and seconds to check its reset
        let date = "2023-02-28 10:12:03"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::system())
            .unwrap();

        // 2023-2-28 is tuesday
        assert_eq!(
            get_formatted_date(&date, "tuesday"),
            "2023-02-28 00:00:00 000000000"
        );

        // 2023-3-01 is wednesday
        assert_eq!(
            get_formatted_date(&date, "wed"),
            "2023-03-01 00:00:00 000000000"
        );

        assert_eq!(
            get_formatted_date(&date, "thu"),
            "2023-03-02 00:00:00 000000000"
        );

        assert_eq!(
            get_formatted_date(&date, "fri"),
            "2023-03-03 00:00:00 000000000"
        );

        assert_eq!(
            get_formatted_date(&date, "sat"),
            "2023-03-04 00:00:00 000000000"
        );

        assert_eq!(
            get_formatted_date(&date, "sun"),
            "2023-03-05 00:00:00 000000000"
        );
    }

    #[test]
    fn test_weekday_with_text_ordinal() {
        let date = "2023-02-28 10:12:03"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::system())
            .unwrap();

        assert_eq!(
            get_formatted_date(&date, "eighth monday"),
            "2023-04-24 00:00:00 000000000"
        );
    }

    #[test]
    fn test_weekday_abbreviation_with_period() {
        let date = "2023-02-28 10:12:03"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::system())
            .unwrap();

        for (weekday, expected) in [
            ("mon.", "2023-03-06 00:00:00 000000000"),
            ("tue.", "2023-02-28 00:00:00 000000000"),
            ("wed.", "2023-03-01 00:00:00 000000000"),
            ("thu.", "2023-03-02 00:00:00 000000000"),
            ("fri.", "2023-03-03 00:00:00 000000000"),
            ("sat.", "2023-03-04 00:00:00 000000000"),
            ("sun.", "2023-03-05 00:00:00 000000000"),
        ] {
            assert_eq!(get_formatted_date(&date, weekday), expected, "{weekday}");
        }

        for (input, expected) in [
            ("wed., 2024-01-03", "2024-01-03 00:00:00"),
            ("mon., 2024-01-03", "2024-01-03 00:00:00"),
        ] {
            let actual = crate::parse_datetime(input).unwrap().expect_in_range();
            assert_eq!(actual.strftime("%F %T").to_string(), expected, "{input}");
        }
    }
}

#[cfg(test)]
mod timestamp {
    use jiff::Timestamp;

    use crate::parse_datetime;

    #[test]
    fn test_positive_and_negative_offsets() {
        let offsets: Vec<i64> = vec![
            0, 1, 2, 10, 100, 150, 2000, 1234400000, 1334400000, 1692582913, 2092582910,
        ];

        for offset in offsets {
            // positive offset
            let time = Timestamp::from_second(offset).unwrap();
            let dt = parse_datetime(format!("@{offset}"))
                .unwrap()
                .expect_in_range();
            assert_eq!(dt.timestamp(), time);

            // negative offset
            let time = Timestamp::from_second(-offset).unwrap();
            let dt = parse_datetime(format!("@-{offset}"))
                .unwrap()
                .expect_in_range();
            assert_eq!(dt.timestamp(), time);
        }
    }
}

/// Used to test example code presented in the README.
mod readme_test {
    use jiff::{civil::DateTime, tz::TimeZone};

    use crate::parse_datetime;

    #[test]
    fn test_readme_code() {
        let dt = parse_datetime("2021-02-14 06:37:47").unwrap();
        let expected = "2021-02-14 06:37:47"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::system())
            .unwrap();

        assert_eq!(dt, expected);
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

        assert_eq!(
            ParseDateTimeError::InvalidInput.to_string(),
            "Invalid input string"
        );
    }
}

mod trace {
    use jiff::{civil::DateTime, tz::TimeZone, Zoned};

    use crate::{parse_datetime_at_date_with_trace, ParsedDateTime, ParsedItem, ParsedYear};

    fn base() -> Zoned {
        "2024-01-15 12:00:00"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::UTC)
            .unwrap()
    }

    #[test]
    fn records_items_and_adjustment_stages() {
        let parsed = parse_datetime_at_date_with_trace(
            base(),
            "Tue, 2024-01-30 03:04:05 UTC +1 month +2 days +3 hours",
        )
        .unwrap();

        assert!(matches!(parsed.datetime, ParsedDateTime::InRange(_)));
        assert!(parsed
            .trace
            .items
            .iter()
            .any(|item| matches!(item, ParsedItem::Date(_))));
        assert!(parsed
            .trace
            .items
            .iter()
            .any(|item| matches!(item, ParsedItem::Time(_))));
        assert!(parsed
            .trace
            .items
            .iter()
            .any(|item| matches!(item, ParsedItem::Weekday { .. })));
        assert!(parsed
            .trace
            .items
            .iter()
            .any(|item| matches!(item, ParsedItem::NamedZone { .. })));
        assert!(parsed
            .trace
            .items
            .iter()
            .any(|item| matches!(item, ParsedItem::Relative(_))));
        assert!(parsed.trace.weekday_ignored);
        assert!(parsed.trace.starting.is_some());
        assert!(parsed.trace.after_date_adjustment.is_some());
        assert!(parsed.trace.after_time_adjustment.is_some());
        assert_eq!(parsed.trace.date_adjustment.months, 1);
        assert_eq!(parsed.trace.date_adjustment.days, 2);
        assert_eq!(parsed.trace.time_adjustment.hours, 3);
        assert!(parsed.trace.month_year_adjustment.is_some());
    }

    #[test]
    fn records_standalone_items_and_timestamp() {
        let time = parse_datetime_at_date_with_trace(base(), "03:04:05").unwrap();
        assert!(time
            .trace
            .items
            .iter()
            .any(|item| matches!(item, ParsedItem::Time(_))));

        let local_zone = parse_datetime_at_date_with_trace(base(), "j").unwrap();
        assert!(local_zone
            .trace
            .items
            .iter()
            .any(|item| matches!(item, ParsedItem::LocalZone { .. })));

        let pure_time = parse_datetime_at_date_with_trace(base(), "1234").unwrap();
        assert!(pure_time
            .trace
            .items
            .iter()
            .any(|item| matches!(item, ParsedItem::NumberTime(_))));

        let timezone_rule =
            parse_datetime_at_date_with_trace(base(), "TZ=\"UTC\" 2024-01-01").unwrap();
        assert_eq!(
            timezone_rule.trace.timezone_rule.as_deref(),
            Some("TZ=\"UTC\"")
        );

        let timestamp = parse_datetime_at_date_with_trace(base(), "@0").unwrap();
        assert!(matches!(timestamp.datetime, ParsedDateTime::InRange(_)));
        assert_eq!(
            timestamp.trace.items,
            vec![ParsedItem::Timestamp {
                seconds: 0,
                nanoseconds: 0,
            }]
        );
        assert!(timestamp.trace.after_time_adjustment.is_some());
    }

    #[test]
    fn records_trace_edge_cases() {
        let yearless = parse_datetime_at_date_with_trace(base(), "11/14").unwrap();
        let inherited_year = yearless.trace.items.iter().any(|item| {
            matches!(
                item,
                ParsedItem::Date(crate::ParsedDate {
                    year: Some(ParsedYear {
                        value: 2024,
                        digits: 0
                    }),
                    ..
                })
            )
        });
        assert!(inherited_year);

        let extended = parse_datetime_at_date_with_trace(base(), "10000-01-01").unwrap();
        assert!(matches!(extended.datetime, ParsedDateTime::Extended(_)));

        let boundary = parse_datetime_at_date_with_trace(base(), "9999-12-31 +1 day").unwrap();
        assert!(matches!(boundary.datetime, ParsedDateTime::Extended(_)));

        let pure_year = parse_datetime_at_date_with_trace(base(), "jul 18 12:30 2025").unwrap();
        assert!(pure_year
            .trace
            .items
            .iter()
            .any(|item| matches!(item, ParsedItem::NumberYear(ParsedYear { value: 2025, .. }))));
        let pure_time = parse_datetime_at_date_with_trace(base(), "1234").unwrap();
        assert!(pure_time
            .trace
            .items
            .iter()
            .any(|item| matches!(item, ParsedItem::NumberTime(_))));

        let paris = "2024-01-15 12:00:00"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::get("Europe/Paris").unwrap())
            .unwrap();
        let named_date =
            parse_datetime_at_date_with_trace(paris.clone(), "2024-01-15 12:00 CET +1 day")
                .unwrap();
        assert!(named_date
            .trace
            .items
            .iter()
            .any(|item| matches!(item, ParsedItem::LocalZone { isdst: 0 })));
        assert!(named_date.trace.after_date_adjustment.is_some());

        let named_time =
            parse_datetime_at_date_with_trace(paris, "2024-01-15 12:00 CET +1 hour").unwrap();
        assert!(named_time.trace.after_time_adjustment.is_some());

        let new_york = "2024-01-15 12:00:00"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::get("America/New_York").unwrap())
            .unwrap();
        let gap = parse_datetime_at_date_with_trace(new_york, "2024-03-10 02:30").unwrap();
        assert!(gap.trace.nonexistent_local_time.is_some());

        let upper_base = "9999-12-30 12:34:56.123456789"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::UTC)
            .unwrap();
        let rollover = parse_datetime_at_date_with_trace(upper_base, "+2 days").unwrap();
        assert!(matches!(rollover.datetime, ParsedDateTime::Extended(_)));
    }
}

#[test]
fn test_datetime_ending_in_z() {
    let actual = parse_datetime("2023-06-03 12:00:01Z").unwrap();
    let expected = "2023-06-03 12:00:01[UTC]".parse::<Zoned>().unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn parsed_datetime_accessors_cover_both_variants() {
    let in_range = parse_datetime("2023-06-03 12:00:01Z").unwrap();
    assert!(in_range.as_zoned().is_some());
    assert!(in_range.into_zoned().is_some());

    let extended = parse_datetime("10000-01-01").unwrap();
    assert!(extended.as_zoned().is_none());
    assert!(extended.into_zoned().is_none());
}

#[test]
fn parsed_datetime_extended_never_equals_zoned() {
    let extended = parse_datetime("10000-01-01").unwrap();
    let zoned = Zoned::now();
    assert_ne!(extended, zoned);
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
    // 1997-01-01 00:00:00 +0000
    let expected = "1997-01-01 00:00:00[UTC]".parse::<Zoned>().unwrap();

    for s in [
        "1997-01-01 00:00:00 +0000",
        "1997-01-01 00:00:00 +00",
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
    let expected = Zoned::now()
        .with()
        .date(date(1997, 1, 1))
        .time(time(0, 0, 0, 0))
        .build()
        .unwrap();

    for s in [
        "1997-01-01 00:00:00.000000000",
        "Wed Jan  1 00:00:00 1997",
        "1997-01-01T00:00:00",
        "1997-01-01 00:00:00",
        "1997-01-01 00:00",
    ] {
        let actual = crate::parse_datetime(s).unwrap();
        assert_eq!(actual, expected);
    }
}

#[test]
fn test_parse_date_notz_nodelta() {
    let expected = Zoned::now()
        .with()
        .date(date(1997, 1, 1))
        .time(time(0, 0, 0, 0))
        .build()
        .unwrap();

    for s in ["1997-01-01", "19970101", "01/01/1997", "01/01/97"] {
        let actual = crate::parse_datetime(s).unwrap();
        assert_eq!(actual, expected);
    }
}

#[test]
fn test_parse_datetime_tz_delta() {
    // 1998-01-01
    let expected = "1998-01-01 00:00:00[UTC]".parse::<Zoned>().unwrap();

    for s in [
        "1997-01-01 00:00:00 +0000 +1 year",
        "1997-01-01 00:00:00 +00 +1 year",
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
    let expected = Zoned::now()
        .with()
        .date(date(1998, 1, 1))
        .time(time(0, 0, 0, 0))
        .build()
        .unwrap();

    for s in [
        "1997-01-01 00:00:00.000000000 1 year",
        "Wed Jan  1 00:00:00 1997 1 year",
        "1997-01-01T00:00:00 1 year",
        "1997-01-01 00:00:00 1 year",
        "1997-01-01 00:00 1 year",
    ] {
        let actual = crate::parse_datetime(s).unwrap();
        assert_eq!(actual, expected);
    }
}

#[test]
fn test_parse_invalid_datetime_notz_delta() {
    // GNU date does not accept the following formats.
    for s in ["199701010000.00 +1 year", "199701010000 +1 year"] {
        assert!(crate::parse_datetime(s).is_err());
    }
}

#[test]
fn test_parse_date_notz_delta() {
    let expected = Zoned::now()
        .with()
        .date(date(1998, 1, 1))
        .time(time(0, 0, 0, 0))
        .build()
        .unwrap();

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

/// GNU `date` accepts bare `ut` / `UT` as a synonym for UTC (issue #280).
/// `gmt` and `GMT` already worked; this test covers all four forms.
#[test]
fn test_bare_utc_timezone_abbreviations() {
    // All four should parse without error and produce a result in UTC (+00:00).
    for abbr in ["ut", "UT", "gmt", "GMT"] {
        let result = parse_datetime(abbr).expect("bare timezone should be accepted");
        let offset_secs = result.expect_in_range().offset().seconds();
        assert_eq!(offset_secs, 0, "'{abbr}' should resolve to UTC (+0)");
    }
}

#[test]
fn test_weekday_only() {
    let now = Zoned::now();
    let midnight = Time::new(0, 0, 0, 0).unwrap();
    let today = now.weekday();
    let midnight_today = now.with().time(midnight).build().unwrap();

    for (s, day) in [
        ("sunday", Weekday::Sunday),
        ("monday", Weekday::Monday),
        ("tuesday", Weekday::Tuesday),
        ("wednesday", Weekday::Wednesday),
        ("thursday", Weekday::Thursday),
        ("friday", Weekday::Friday),
        ("saturday", Weekday::Saturday),
    ] {
        let actual = parse_datetime(s).unwrap();
        let delta = day.since(today);
        let expected = midnight_today.checked_add(delta.days()).unwrap();
        assert_eq!(actual, expected);
    }
}

mod test_relative {
    use crate::parse_datetime;

    #[test]
    fn test_hence_suffix() {
        let now: jiff::Zoned = "2026-01-15T12:00:00Z[UTC]".parse().unwrap();
        for relative in [
            "2 months",
            "year",
            "-2 days",
            "next week",
            "0 hours",
            "1.5 seconds",
        ] {
            let expected = crate::parse_datetime_at_date(now.clone(), relative).unwrap();
            for suffix in ["hence", "HENCE"] {
                assert_eq!(
                    crate::parse_datetime_at_date(now.clone(), format!("{relative} {suffix}"))
                        .unwrap(),
                    expected,
                );
            }
        }
        assert!(crate::parse_datetime_at_date(now.clone(), "hence").is_err());
        assert!(crate::parse_datetime_at_date(now, "2 days henceforth").is_err());
    }

    #[test]
    fn test_month() {
        assert_eq!(
            parse_datetime("28 feb + 1 month")
                .expect("parse_datetime")
                .expect_in_range()
                .strftime("%m%d")
                .to_string(),
            "0328"
        );

        // 29 feb 2025 is invalid
        assert!(parse_datetime("29 feb + 1 year").is_err());

        // 29 feb 2025 is an invalid date
        assert!(parse_datetime("29 feb 2025").is_err());

        // because 29 feb 2025 is invalid, 29 feb 2025 + 1 day is invalid
        // arithmetic does not operate on invalid dates
        assert!(parse_datetime("29 feb 2025 + 1 day").is_err());

        // 28 feb 2023 + 1 day = 1 mar
        assert_eq!(
            parse_datetime("28 feb 2023 + 1 day")
                .unwrap()
                .expect_in_range()
                .strftime("%m%d")
                .to_string(),
            "0301"
        );
    }

    #[test]
    fn month_overflow() {
        assert_eq!(
            parse_datetime("2024-01-31 + 1 month")
                .unwrap()
                .expect_in_range()
                .strftime("%Y-%m-%dT%H:%M:%S")
                .to_string(),
            "2024-03-02T00:00:00",
        );

        assert_eq!(
            parse_datetime("2024-02-29 + 1 month")
                .unwrap()
                .expect_in_range()
                .strftime("%Y-%m-%dT%H:%M:%S")
                .to_string(),
            "2024-03-29T00:00:00",
        );
    }
}

mod test_gnu {
    use crate::parse_datetime;

    #[test]
    fn gnu_compat() {
        const FMT: &str = "%Y-%m-%d %H:%M:%S";
        let input = "0000-03-02 00:00:00";
        assert_eq!(
            input,
            parse_datetime(input)
                .unwrap()
                .expect_in_range()
                .strftime(FMT)
                .to_string()
        );

        let input = "2621-03-10 00:00:00";
        assert_eq!(
            input,
            parse_datetime(input)
                .unwrap()
                .expect_in_range()
                .strftime(FMT)
                .to_string()
        );

        let input = "1038-03-10 00:00:00";
        assert_eq!(
            input,
            parse_datetime(input)
                .unwrap()
                .expect_in_range()
                .strftime(FMT)
                .to_string()
        );
    }
}

#[cfg(test)]
mod display {
    use jiff::{civil::DateTime, tz::TimeZone};

    use crate::{parse_datetime_at_date, ExtendedDateTime, ParsedDateTime};

    #[test]
    fn in_range_without_nanoseconds() {
        let base = "2024-06-15 08:30:00"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::UTC)
            .unwrap();
        let parsed = parse_datetime_at_date(base, "2024-06-15 08:30:00").unwrap();
        assert_eq!(parsed.to_string(), "2024-06-15 08:30:00+00:00");
    }

    #[test]
    fn in_range_with_nanoseconds() {
        let base = "2024-06-15 08:30:00.123456789"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::UTC)
            .unwrap();
        // Parsing "now" at a base with subsecond precision preserves it.
        let parsed = parse_datetime_at_date(base, "now").unwrap();
        assert_eq!(parsed.to_string(), "2024-06-15 08:30:00.123456789+00:00");
    }

    #[test]
    fn extended_without_nanoseconds() {
        let dt = ExtendedDateTime::new(
            crate::DateParts {
                year: 12000,
                month: 3,
                day: 15,
            },
            crate::TimeParts {
                hour: 10,
                minute: 30,
                second: 45,
                nanosecond: 0,
            },
            3600,
        )
        .unwrap();
        assert_eq!(
            ParsedDateTime::Extended(dt).to_string(),
            "12000-03-15 10:30:45+01:00"
        );
    }

    #[test]
    fn extended_with_nanoseconds() {
        let dt = ExtendedDateTime::new(
            crate::DateParts {
                year: 12000,
                month: 3,
                day: 15,
            },
            crate::TimeParts {
                hour: 10,
                minute: 30,
                second: 45,
                nanosecond: 123456789,
            },
            -18000,
        )
        .unwrap();
        assert_eq!(
            ParsedDateTime::Extended(dt).to_string(),
            "12000-03-15 10:30:45.123456789-05:00"
        );
    }
}

mod unix_epoch_second {
    use jiff::{civil::DateTime, tz::TimeZone};

    use crate::{parse_datetime, parse_datetime_at_date, DateParts, ParsedDateTime, TimeParts};

    fn epoch_second(input: &str) -> (i64, i32) {
        let base = "2024-01-01 00:00:00"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::UTC)
            .unwrap();
        let parsed = parse_datetime_at_date(base, input).unwrap();
        (parsed.unix_epoch_second(), parsed.subsec_nanosecond())
    }

    #[test]
    fn positive_integer_epoch() {
        let (sec, nsec) = epoch_second("@1690466034");
        assert_eq!(sec, 1690466034);
        assert_eq!(nsec, 0);
    }

    #[test]
    fn positive_fractional_epoch() {
        let (sec, nsec) = epoch_second("@1690466034.5");
        assert_eq!(sec, 1690466034);
        assert_eq!(nsec, 500_000_000);
    }

    #[test]
    fn negative_integer_epoch() {
        let (sec, nsec) = epoch_second("@-1");
        assert_eq!(sec, -1);
        assert_eq!(nsec, 0);
    }

    #[test]
    fn negative_fractional_epoch_floors() {
        // Issue #283: @-1.5 must floor to -2, not truncate to -1.
        let (sec, nsec) = epoch_second("@-1.5");
        assert_eq!(sec, -2, "floor(-1.5) should be -2, not -1");
        assert_eq!(nsec, 500_000_000);
    }

    #[test]
    fn negative_fractional_epoch_large() {
        // From issue #283: @-893375784.554767216
        let (sec, nsec) = epoch_second("@-893375784.554767216");
        assert_eq!(sec, -893375785);
        assert_eq!(nsec, 445_232_784);
    }

    #[test]
    fn zero_epoch() {
        let (sec, nsec) = epoch_second("@0");
        assert_eq!(sec, 0);
        assert_eq!(nsec, 0);
    }

    #[test]
    fn negative_zero_fractional() {
        // @-0.5 should floor to -1
        let (sec, nsec) = epoch_second("@-0.5");
        assert_eq!(sec, -1);
        assert_eq!(nsec, 500_000_000);
    }

    #[test]
    fn extended_values_use_extended_epoch_path() {
        let parsed = parse_datetime("10000-01-01").unwrap();
        // Year 10000 is beyond jiff::Zoned's representable range, so
        // parse_datetime must route through the ExtendedDateTime branch
        // of both accessors.
        assert!(matches!(parsed, ParsedDateTime::Extended(_)));
        assert_eq!(parsed.subsec_nanosecond(), 0);

        // Year 10000 UTC in seconds since 1970 is well past 2.5e11.
        assert!(parsed.unix_epoch_second() > 250_000_000_000);
    }

    #[test]
    fn manually_constructed_extended_values_use_extended_epoch_path() {
        let ext = crate::ExtendedDateTime::new(
            DateParts {
                year: 10_000,
                month: 1,
                day: 1,
            },
            TimeParts {
                hour: 12,
                minute: 34,
                second: 56,
                nanosecond: 789_000_000,
            },
            0,
        )
        .unwrap();
        let parsed = ParsedDateTime::Extended(ext.clone());

        assert_eq!(parsed.unix_epoch_second(), ext.unix_seconds());
        assert_eq!(parsed.subsec_nanosecond(), 0);
    }
}

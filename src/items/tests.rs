// For the full copyright and license information, view the LICENSE file.

use crate::ParsedDateTime;
use jiff::{civil::DateTime, tz::TimeZone, ToSpan, Zoned};

use super::builder::DateTimeBuilder;
use super::parser::parse;
use super::{parse_at_date, parse_at_local};

fn at_date(builder: DateTimeBuilder, base: Zoned) -> Zoned {
    builder.set_base(base).build().unwrap().expect_in_range()
}

fn at_utc(builder: DateTimeBuilder) -> Zoned {
    at_date(builder, Zoned::now().with_time_zone(TimeZone::UTC))
}

fn test_eq_fmt(fmt: &str, input: &str) -> String {
    let input = input.to_ascii_lowercase();
    parse(&mut input.as_str())
        .map(at_utc)
        .map_err(|e| eprintln!("TEST FAILED AT:\n{e}"))
        .expect("parsing failed during tests")
        .strftime(fmt)
        .to_string()
}

fn assert_extended_datetime(input: &str, base: Zoned, expected: &str) {
    let parsed = parse_at_date(base, input).unwrap();
    assert!(
        matches!(parsed, ParsedDateTime::Extended(_)),
        "expected extended datetime, got in-range for: {input}"
    );
    assert_eq!(parsed.to_string(), expected, "{input}");
}

fn expect_extended_datetime(parsed: ParsedDateTime) -> crate::ExtendedDateTime {
    match parsed {
        ParsedDateTime::Extended(dt) => dt,
        ParsedDateTime::InRange(z) => panic!("expected extended datetime, got in-range: {z}"),
    }
}

#[test]
#[should_panic(expected = "parsing failed during tests")]
fn test_eq_fmt_reports_parse_errors() {
    let _ = test_eq_fmt("%Y", "not-a-date");
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

    assert_eq!(
        "2023-07-27T13:53:54+00:00",
        test_eq_fmt("%Y-%m-%dT%H:%M:%S%:z", " @1690466034 ")
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
fn date_and_time_ampm() {
    // https://github.com/uutils/parse_datetime/issues/282
    assert_eq!("12:00", test_eq_fmt("%H:%M", "2024-06-15 12:00 PM"));
    assert_eq!("11:30", test_eq_fmt("%H:%M", "2024-06-15 11:30am"));
    assert_eq!("15:00", test_eq_fmt("%H:%M", "2024-06-15 3:00 PM"));
}

#[test]
fn empty() {
    let result = parse(&mut "");
    assert!(result.is_ok());
}

#[test]
fn whitespace_only() {
    let result = parse(&mut "\x0B \x0C");
    assert!(result.is_ok());
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

    let result = parse(&mut "2025-05-19 +00:00 +01:00");
    assert!(result.is_err());

    let result = parse(&mut "m1y");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("time offset cannot appear more than once"));

    let result = parse(&mut "2025-05-19 abcdef");
    assert!(result.is_err());

    let result = parse(&mut "@1690466034 2025-05-19");
    assert!(result.is_err());

    let result = parse(&mut "2025-05-19 @1690466034");
    assert!(result.is_err());

    // Pure number as year (large years are accepted).
    let result = parse(&mut "jul 18 12:30 10000");
    assert!(result.is_ok());
    let built = result.unwrap().build().unwrap();
    let dt = expect_extended_datetime(built);
    assert_eq!(dt.year, 10000);

    // Pure number as time (too long).
    let result = parse(&mut "01:02 12345");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("pure number must be 1-4 digits when interpreted as time"));

    // Pure number as time (repeated time).
    let result = parse(&mut "01:02 1234");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("time cannot appear more than once"));

    // Pure number as time (invalid hour).
    let result = parse(&mut "jul 18 2025 2400");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("invalid hour in pure number"));

    // Pure number as time (invalid minute).
    let result = parse(&mut "jul 18 2025 2360");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("invalid minute in pure number"));
}

#[test]
fn negative_base_year_with_yearless_date_errors() {
    let base = DateTime::new(-1, 1, 1, 0, 0, 0, 0)
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    let result = parse_at_date(base, "11/14");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("base year must be non-negative"));
}

#[test]
fn boundary_rollover_from_9999_falls_back_to_extended() {
    let base = "2000-01-01 00:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    assert_extended_datetime("9999-12-31 +1 day", base, "10000-01-01 00:00:00+00:00");
}

#[test]
fn boundary_year_9999_absolute_date_parses() {
    let base = "2000-01-01 00:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    assert_extended_datetime("9999-12-31", base, "9999-12-31 00:00:00+00:00");
}

#[test]
fn boundary_year_9999_with_time_parses() {
    let base = "2000-01-01 00:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    assert_extended_datetime("9999-12-31 12:00", base, "9999-12-31 12:00:00+00:00");
}

#[test]
fn boundary_year_9999_with_utc_timezone_parses() {
    let base = "2000-01-01 00:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    assert_extended_datetime("9999-12-31 12:00 UTC", base, "9999-12-31 12:00:00+00:00");
}

#[test]
fn boundary_year_9999_with_explicit_offset_parses() {
    let base = "2000-01-01 00:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    assert_extended_datetime("9999-12-31 12:00 +01:00", base, "9999-12-31 12:00:00+01:00");
}

#[test]
fn large_year_relative_parity_months_and_years() {
    let base = "2000-01-01 00:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();

    assert_extended_datetime(
        "10000-01-31 +2 months",
        base.clone(),
        "10000-03-31 00:00:00+00:00",
    );
    assert_extended_datetime(
        "10000-01-31 +3 months",
        base.clone(),
        "10000-05-01 00:00:00+00:00",
    );
    assert_extended_datetime(
        "10000-08-31 +6 months",
        base.clone(),
        "10001-03-03 00:00:00+00:00",
    );
    assert_extended_datetime(
        "10000-05-31 3 months ago",
        base.clone(),
        "10000-03-02 00:00:00+00:00",
    );
    assert_extended_datetime("10000-02-29 +1 year", base, "10001-03-01 00:00:00+00:00");
}

#[test]
fn rejects_year_above_gnu_max() {
    assert!(parse_at_local("2147485548-01-01").is_err());
}

#[test]
fn large_year_can_return_in_range_after_relative() {
    let base = "2000-01-01 00:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    let result = parse_at_date(base, "10000-01-01 -1000 years").unwrap();
    let z = result.expect_in_range();
    assert_eq!(
        z.strftime("%Y-%m-%d %H:%M:%S%:z").to_string(),
        "9000-01-01 00:00:00+00:00"
    );
}

#[test]
fn large_year_can_return_in_range_after_relative_with_named_timezone_rule() {
    let base = "2000-01-01 00:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    let result = parse_at_date(base, "TZ=\"Europe/Paris\" 10000-01-01 -1000 years").unwrap();
    let z = result.expect_in_range();
    assert_eq!(
        z.strftime("%Y-%m-%d %H:%M:%S%:z").to_string(),
        "9000-01-01 00:00:00+01:00"
    );
}

#[test]
fn large_year_time_with_explicit_offset_is_extended() {
    let base = "2000-01-01 00:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    let dt = expect_extended_datetime(parse_at_date(base, "10000-01-01 12:34:56+02:00").unwrap());
    assert_eq!((dt.year, dt.month, dt.day), (10000, 1, 1));
    assert_eq!((dt.hour, dt.minute, dt.second), (12, 34, 56));
    assert_eq!(dt.offset_seconds, 2 * 3600);
}

#[test]
fn parse_at_date_returns_error_for_invalid_input() {
    let base = "2000-01-01 00:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    let result = parse_at_date(base, "not-a-date");
    assert!(result.is_err());
}

#[test]
#[should_panic(expected = "expected extended datetime")]
fn assert_extended_datetime_panics_for_in_range_input() {
    let base = "2000-01-01 00:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    assert_extended_datetime("2001-01-01", base, "2001-01-01 00:00:00+00:00");
}

#[test]
#[should_panic(expected = "expected extended datetime")]
fn expect_extended_datetime_panics_for_in_range_input() {
    let base = "2000-01-01 00:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    let parsed = parse_at_date(base, "2001-01-01").unwrap();
    let _ = expect_extended_datetime(parsed);
}

#[test]
#[should_panic(expected = "ParsedDateTime is not representable as jiff::Zoned")]
fn expect_in_range_panics_for_extended_input() {
    let base = "2000-01-01 00:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    let parsed = parse_at_date(base, "10000-01-01").unwrap();
    let _ = parsed.expect_in_range();
}

#[test]
fn relative_weekday() {
    // Jan 1 2025 is a Wed
    let now = "2025-01-01 00:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();

    assert_eq!(
        at_date(parse(&mut "last wed").unwrap(), now.clone()),
        now.checked_sub(7.days()).unwrap()
    );
    assert_eq!(at_date(parse(&mut "this wed").unwrap(), now.clone()), now);
    assert_eq!(
        at_date(parse(&mut "next wed").unwrap(), now.clone()),
        now.checked_add(7.days()).unwrap()
    );
    assert_eq!(
        at_date(parse(&mut "last thu").unwrap(), now.clone()),
        now.checked_sub(6.days()).unwrap()
    );
    assert_eq!(
        at_date(parse(&mut "this thu").unwrap(), now.clone()),
        now.checked_add(1.days()).unwrap()
    );
    assert_eq!(
        at_date(parse(&mut "next thu").unwrap(), now.clone()),
        now.checked_add(1.days()).unwrap()
    );
    assert_eq!(
        at_date(parse(&mut "1 wed").unwrap(), now.clone()),
        now.checked_add(7.days()).unwrap()
    );
    assert_eq!(
        at_date(parse(&mut "1 thu").unwrap(), now.clone()),
        now.checked_add(1.days()).unwrap()
    );
    assert_eq!(
        at_date(parse(&mut "2 wed").unwrap(), now.clone()),
        now.checked_add(14.days()).unwrap()
    );
    assert_eq!(
        at_date(parse(&mut "2 thu").unwrap(), now.clone()),
        now.checked_add(8.days()).unwrap()
    );
}

#[test]
fn relative_date_time() {
    let now = Zoned::now().with_time_zone(TimeZone::UTC);

    let result = at_date(parse(&mut "2 days ago").unwrap(), now.clone());
    assert_eq!(result, now.checked_sub(2.days()).unwrap());
    assert_eq!(result.hour(), now.hour());
    assert_eq!(result.minute(), now.minute());
    assert_eq!(result.second(), now.second());

    let result = at_date(parse(&mut "2 days 3 days ago").unwrap(), now.clone());
    assert_eq!(result, now.checked_sub(1.days()).unwrap());
    assert_eq!(result.hour(), now.hour());
    assert_eq!(result.minute(), now.minute());
    assert_eq!(result.second(), now.second());

    let result = at_date(parse(&mut "2025-01-01 2 days ago").unwrap(), now.clone());
    assert_eq!(result.hour(), 0);
    assert_eq!(result.minute(), 0);
    assert_eq!(result.second(), 0);

    let result = at_date(parse(&mut "3 weeks").unwrap(), now.clone());
    assert_eq!(result, now.checked_add(21.days()).unwrap());
    assert_eq!(result.hour(), now.hour());
    assert_eq!(result.minute(), now.minute());
    assert_eq!(result.second(), now.second());

    let result = at_date(parse(&mut "2025-01-01 3 weeks").unwrap(), now);
    assert_eq!(result.hour(), 0);
    assert_eq!(result.minute(), 0);
    assert_eq!(result.second(), 0);
}

#[test]
fn offset_followed_by_relative() {
    let base = "2000-01-01 12:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();

    // Both read "+8 years" as a relative item, not as an offset of +8.
    for input in ["UTC +8 years", "+8 years"] {
        let result = parse_at_date(base.clone(), input).unwrap();
        assert_eq!(result.to_string(), "2008-01-01 12:00:00+00:00", "{input}");
    }
}

#[test]
fn comments_and_ignored_signs() {
    let base = "2000-06-15 12:34:56"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();

    for (input, expected) in [
        ("1997-01-01 (foo) 10:00", "1997-01-01 10:00:00+00:00"),
        ("1997-01-01 (a (b)) 10:00", "1997-01-01 10:00:00+00:00"),
        ("(foo) 1997-01-01", "1997-01-01 00:00:00+00:00"),
        ("+ (c) 1997-01-01", "1997-01-01 00:00:00+00:00"),
        ("- (c) 1997-01-01", "1997-01-01 00:00:00+00:00"),
        ("1997-01-01 (x) + 1 day", "1997-01-02 00:00:00+00:00"),
    ] {
        let result = parse_at_date(base.clone(), input).unwrap();
        assert_eq!(result.to_string(), expected, "{input}");
    }

    // Like GNU, a hyphen followed by a digit is a sign, not ignored.
    assert!(parse_at_date(base.clone(), "- 1997-01-01").is_err());
}

#[test]
fn pure() {
    let now = Zoned::now().with_time_zone(TimeZone::UTC);

    // Pure number as year.
    let result = at_date(parse(&mut "jul 18 12:30 2025").unwrap(), now.clone());
    assert_eq!(result.year(), 2025);

    // Pure number as time.
    let result = at_date(parse(&mut "1230").unwrap(), now.clone());
    assert_eq!(result.hour(), 12);
    assert_eq!(result.minute(), 30);

    let result = at_date(parse(&mut "123").unwrap(), now.clone());
    assert_eq!(result.hour(), 1);
    assert_eq!(result.minute(), 23);

    let result = at_date(parse(&mut "12").unwrap(), now.clone());
    assert_eq!(result.hour(), 12);
    assert_eq!(result.minute(), 0);

    let result = at_date(parse(&mut "1").unwrap(), now.clone());
    assert_eq!(result.hour(), 1);
    assert_eq!(result.minute(), 0);
}

#[test]
fn timezone_rule() {
    let parse_build = |mut s| parse(&mut s).unwrap().build().unwrap();

    for (input, expected) in [
        (
            r#"TZ="Europe/Paris" 2025-01-02"#,
            "2025-01-02 00:00:00[Europe/Paris]"
                .parse::<Zoned>()
                .unwrap(),
        ),
        (
            r#"TZ="Europe/Paris" 2025-01-02 03:04:05"#,
            "2025-01-02 03:04:05[Europe/Paris]"
                .parse::<Zoned>()
                .unwrap(),
        ),
    ] {
        assert_eq!(parse_build(input), expected, "{input}");
    }
}
/// `j` is a time zone item, so it participates in the same duplicate check
/// as a numeric offset. GNU rejects all three of these as a repeated zone.
#[test]
fn military_j_is_a_timezone_item() {
    // Parses on its own, attached or spaced, in either case.
    for input in ["j", "8j", "8 j", "8J", "j 8"] {
        assert!(parse(&mut &*input).is_ok(), "`{input}` should parse");
    }

    for input in ["8j utc", "8 utc j", "8 j j"] {
        let result = parse(&mut &*input);
        assert!(result.is_err(), "`{input}` should be rejected");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("time offset cannot appear more than once"),
            "`{input}` should report a repeated time offset"
        );
    }

    // A bare numeric offset is not an item on its own (the grammar requires
    // a named zone), so this is rejected by the parser rather than by the
    // duplicate check. GNU rejects it too.
    assert!(parse(&mut "j +05:00").is_err());
}

// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Coverage map:
//! - General syntax: <https://www.gnu.org/software/coreutils/manual/html_node/General-date-syntax.html>
//! - Calendar items: <https://www.gnu.org/software/coreutils/manual/html_node/Calendar-date-items.html>
//! - Time of day: <https://www.gnu.org/software/coreutils/manual/html_node/Time-of-day-items.html>
//! - Time zones: <https://www.gnu.org/software/coreutils/manual/html_node/Time-zone-items.html>
//! - Combined date and time: <https://www.gnu.org/software/coreutils/manual/html_node/Combined-date-and-time-of-day-items.html>
//! - Weekdays: <https://www.gnu.org/software/coreutils/manual/html_node/Day-of-week-items.html>
//! - Relative items: <https://www.gnu.org/software/coreutils/manual/html_node/Relative-items-in-date-strings.html>
//! - Pure numbers: <https://www.gnu.org/software/coreutils/manual/html_node/Pure-numbers-in-date-strings.html>
//! - Epoch seconds: <https://www.gnu.org/software/coreutils/manual/html_node/Seconds-since-the-Epoch.html>
//! - Time-zone rules: <https://www.gnu.org/software/coreutils/manual/html_node/Specifying-time-zone-rules.html>

use jiff::{civil::DateTime, tz::TimeZone};
use parse_datetime::{parse_datetime, parse_datetime_at_date, ParsedDateTime};
use rstest::rstest;

mod common;
use common::{check_absolute, check_relative};

const CANONICAL_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.9f %:z";

fn canonical(parsed: ParsedDateTime) -> String {
    parsed
        .expect_in_range()
        .with_time_zone(TimeZone::UTC)
        .strftime(CANONICAL_FORMAT)
        .to_string()
}

fn check_canonical(input: &str, expected: &str) {
    std::env::set_var("TZ", "UTC0");
    let actual = parse_datetime(input)
        .map(canonical)
        .unwrap_or_else(|error| panic!("Failed to parse '{input}': {error}"));
    assert_eq!(actual, expected, "Input value: {input}");
}

fn check_canonical_at_date(base: &str, input: &str, expected: &str) {
    std::env::set_var("TZ", "UTC0");
    let base = base
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    let actual = parse_datetime_at_date(base, input)
        .map(canonical)
        .unwrap_or_else(|error| panic!("Failed to parse '{input}': {error}"));
    assert_eq!(actual, expected, "Input value: {input}");
}

fn check_invalid(input: &str) {
    assert!(
        parse_datetime(input).is_err(),
        "Input value '{input}' should be rejected"
    );
}

#[rstest]
#[case::iso8601("2022-11-14", "2022-11-14 00:00:00+00:00")]
#[case::short_year_22("22-11-14", "2022-11-14 00:00:00+00:00")]
#[case::short_year_68("68-11-14", "2068-11-14 00:00:00+00:00")]
#[case::short_year_00("00-11-14", "2000-11-14 00:00:00+00:00")]
#[case::short_year_69("69-11-14", "1969-11-14 00:00:00+00:00")]
#[case::short_year_99("99-11-14", "1999-11-14 00:00:00+00:00")]
#[case::us_style("11/14/2022", "2022-11-14 00:00:00+00:00")]
#[case::us_style_short_year("11/14/22", "2022-11-14 00:00:00+00:00")]
#[case::year_zero("0000-01-01", "0000-01-01 00:00:00+00:00")]
#[case::year_001("001-11-14", "0001-11-14 00:00:00+00:00")]
#[case::year_100("100-11-14", "0100-11-14 00:00:00+00:00")]
#[case::year_999("999-11-14", "0999-11-14 00:00:00+00:00")]
#[case::year_9999("9999-11-14", "9999-11-14 00:00:00+00:00")]
#[case::dotted_date("14.11.2022", "2022-11-14 00:00:00+00:00")]
#[case::dotted_date_spaced("14 . 11 . 2022", "2022-11-14 00:00:00+00:00")]
#[case::dotted_date_short_year("14.11.22", "2022-11-14 00:00:00+00:00")]
#[case::year_10000("10000-12-31", "10000-12-31 00:00:00+00:00")]
#[case::year_100000("100000-12-31", "100000-12-31 00:00:00+00:00")]
#[case::year_1000000("1000000-12-31", "1000000-12-31 00:00:00+00:00")]
#[case::year_10000000("10000000-12-31", "10000000-12-31 00:00:00+00:00")]
#[case::max_date("2147485547-12-31", "2147485547-12-31 00:00:00+00:00")]
#[case::long_month_in_the_middle("14 November 2022", "2022-11-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_lowercase("14 november 2022", "2022-11-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_uppercase("14 NOVEMBER 2022", "2022-11-14 00:00:00+00:00")]
#[case::short_month_in_the_middle("14 nov 2022", "2022-11-14 00:00:00+00:00")]
#[case::short_month_in_the_uppercase("14 NOV 2022", "2022-11-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_hyphened("14-november-2022", "2022-11-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_nospace("14november2022", "2022-11-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_hyphened("14-nov-2022", "2022-11-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_nospace("14nov2022", "2022-11-14 00:00:00+00:00")]
#[case::long_month_at_start("November 14 2022", "2022-11-14 00:00:00+00:00")]
#[case::long_month_at_start_with_comma("November 14, 2022", "2022-11-14 00:00:00+00:00")]
#[case::short_month_at_start("nov 14 2022", "2022-11-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_jan("14 January 2022", "2022-01-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_feb("14 February 2022", "2022-02-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_mar("14 March 2022", "2022-03-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_apr("14 April 2022", "2022-04-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_may("14 May 2022", "2022-05-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_jun("14 June 2022", "2022-06-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_jul("14 July 2022", "2022-07-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_aug("14 August 2022", "2022-08-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_sep("14 September 2022", "2022-09-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_oct("14 October 2022", "2022-10-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_dec("14 December 2022", "2022-12-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_jan("14 jan 2022", "2022-01-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_feb("14 feb 2022", "2022-02-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_mar("14 mar 2022", "2022-03-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_apr("14 apr 2022", "2022-04-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_may("14 may 2022", "2022-05-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_jun("14 jun 2022", "2022-06-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_jul("14 jul 2022", "2022-07-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_aug("14 aug 2022", "2022-08-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_sep("14 sep 2022", "2022-09-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_sept("14 sept 2022", "2022-09-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_oct("14 oct 2022", "2022-10-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_dec("14 dec 2022", "2022-12-14 00:00:00+00:00")]
fn test_absolute_date_numeric(#[case] input: &str, #[case] expected: &str) {
    check_absolute(input, expected);
}

// ASCII whitespace may separate items.
#[rstest]
#[case::vertical_tab_between_date_and_year("Jan 23\x0B2026 1:00AM", "2026-01-23 01:00:00+00:00")]
#[case::form_feed_between_date_and_year("Jan 23\x0C2026 1:00AM", "2026-01-23 01:00:00+00:00")]
#[case::vertical_tab_after_comma("Jan 23,\x0B2026", "2026-01-23 00:00:00+00:00")]
#[case::vertical_tab_before_time("14 nov 2026\x0B1:00AM", "2026-11-14 01:00:00+00:00")]
#[case::vertical_tab_as_iso_separator("2026-01-23\x0B01:00:00", "2026-01-23 01:00:00+00:00")]
fn test_whitespace_between_items(#[case] input: &str, #[case] expected: &str) {
    check_absolute(input, expected);
}

// GNU treats an unmatched opening parenthesis as a comment through the end
// of the input.
#[rstest]
#[case::unterminated_after_date(
    "2024-02-29 12:34:56 (unterminated",
    "2024-02-29 12:34:56.000000000 +00:00"
)]
#[case::comment_before_date("(ignored) 1997-01-01", "1997-01-01 00:00:00.000000000 +00:00")]
fn test_comments(#[case] input: &str, #[case] expected: &str) {
    check_canonical_at_date("2000-06-15 12:34:56", input, expected);
}

#[rstest]
#[case::us_style("11/14", 2022, "2022-11-14 00:00:00+00:00")]
#[case::alphabetical_full_month_in_front("november 14", 2022, "2022-11-14 00:00:00+00:00")]
#[case::alphabetical_full_month_at_back("14 november", 2022, "2022-11-14 00:00:00+00:00")]
#[case::alphabetical_short_month_in_front("nov 14", 2022, "2022-11-14 00:00:00+00:00")]
#[case::alphabetical_short_month_at_back("14 nov", 2022, "2022-11-14 00:00:00+00:00")]
#[case::alphabetical_full_month_in_front("november 14", 2022, "2022-11-14 00:00:00+00:00")]
#[case::alphabetical_full_month_at_back("14 november", 2022, "2022-11-14 00:00:00+00:00")]
#[case::alphabetical_short_month_in_front("nov 14", 2022, "2022-11-14 00:00:00+00:00")]
#[case::alphabetical_short_month_at_back("14 nov", 2022, "2022-11-14 00:00:00+00:00")]
#[case::alphabetical_long_month_at_back_hyphen("14-november", 2022, "2022-11-14 00:00:00+00:00")]
#[case::alphabetical_short_month_at_back_hyphen("14-nov", 2022, "2022-11-14 00:00:00+00:00")]
fn test_date_omitting_year(#[case] input: &str, #[case] year: u32, #[case] expected: &str) {
    let now = format!("{year}-06-01 00:00:00")
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    check_relative(now, input, expected);
}

#[rstest]
#[case::tz_prefix_est5("TZ=\"EST5\" 1970-01-01 00:00", "1970-01-01 00:00:00-05:00")]
#[case::tz_prefix_pst8("TZ=\"PST8\" 1970-01-01 00:00", "1970-01-01 00:00:00-08:00")]
#[case::tz_prefix_utc("TZ=\"UTC\" 1970-01-01 12:00", "1970-01-01 12:00:00+00:00")]
#[case::tz_prefix_europe_paris(
    r#"TZ="Europe/Paris" 2025-01-02 03:04:05"#,
    "2025-01-02 03:04:05+01:00"
)]
// Unknown IANA names use the UTC fallback.
#[case::tz_prefix_unknown_iana(r#"TZ="Not/A/Zone" 1970-01-01 00:00"#, "1970-01-01 00:00:00+00:00")]
fn test_tz_prefix_with_base_date(#[case] input: &str, #[case] expected: &str) {
    let base = "2020-06-15 12:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    check_relative(base, input, expected);
}

#[rstest]
#[case::utc_rule(
    r#"TZ="UTC0" 2024-01-15 12:00:00"#,
    "2024-01-15 12:00:00.000000000 +00:00"
)]
#[case::new_york_winter(
    r#"TZ="America/New_York" 2024-01-15 12:00:00"#,
    "2024-01-15 17:00:00.000000000 +00:00"
)]
#[case::new_york_summer(
    r#"TZ="America/New_York" 2024-07-15 12:00:00"#,
    "2024-07-15 16:00:00.000000000 +00:00"
)]
#[case::paris_winter(
    r#"TZ="Europe/Paris" 2024-01-15 12:00:00"#,
    "2024-01-15 11:00:00.000000000 +00:00"
)]
#[case::paris_summer(
    r#"TZ="Europe/Paris" 2024-07-15 12:00:00"#,
    "2024-07-15 10:00:00.000000000 +00:00"
)]
#[case::dst_fall_overlap(
    r#"TZ="America/New_York" 2024-11-03 01:30:00"#,
    "2024-11-03 05:30:00.000000000 +00:00"
)]
fn test_timezone_rules(#[case] input: &str, #[case] expected: &str) {
    check_canonical(input, expected);
}

// Leap-day year overflow.
#[rstest]
#[case::feb29_1996_plus_1year("1996-02-29 00:00:00", "1 year", "1997-03-01 00:00:00+00:00")]
#[case::feb29_2020_plus_1year("2020-02-29 00:00:00", "1 year", "2021-03-01 00:00:00+00:00")]
#[case::feb29_2000_plus_1year("2000-02-29 00:00:00", "1 year", "2001-03-01 00:00:00+00:00")]
// Zero years preserves the date.
#[case::zero_years("2024-01-15 12:30:45", "0 years", "2024-01-15 12:30:45+00:00")]
#[case::zero_years_feb29("2020-02-29 00:00:00", "0 years", "2020-02-29 00:00:00+00:00")]
fn test_leap_year_overflow(#[case] base: &str, #[case] input: &str, #[case] expected: &str) {
    let now = base
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    check_relative(now, input, expected);
}

// Month arithmetic with day overflow.
#[rstest]
#[case::jan31_plus_1month_leap("2024-01-31 00:00:00", "1 month", "2024-03-02 00:00:00+00:00")]
#[case::jan31_plus_1month_nonleap("2023-01-31 00:00:00", "1 month", "2023-03-03 00:00:00+00:00")]
#[case::mar31_plus_1month("2024-03-31 00:00:00", "1 month", "2024-05-01 00:00:00+00:00")]
#[case::may31_plus_1month("2024-05-31 00:00:00", "1 month", "2024-07-01 00:00:00+00:00")]
#[case::rel_2b("1997-01-19 08:17:48", "7 months ago", "1996-06-19 08:17:48+00:00")]
// Zero months preserves the date.
#[case::zero_months("2024-01-31 12:30:45", "0 months", "2024-01-31 12:30:45+00:00")]
#[case::zero_months_feb29("2020-02-29 00:00:00", "0 months", "2020-02-29 00:00:00+00:00")]
fn test_month_overflow(#[case] base: &str, #[case] input: &str, #[case] expected: &str) {
    let now = base
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    check_relative(now, input, expected);
}

// Negative year operations with leap days.
#[rstest]
#[case::feb29_minus_1year("2020-02-29 00:00:00", "1 year ago", "2019-03-01 00:00:00+00:00")]
#[case::feb29_minus_4years("2020-02-29 00:00:00", "4 years ago", "2016-02-29 00:00:00+00:00")]
#[case::march1_minus_1year("2021-03-01 00:00:00", "1 year ago", "2020-03-01 00:00:00+00:00")]
fn test_negative_year_operations(#[case] base: &str, #[case] input: &str, #[case] expected: &str) {
    let now = base
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    check_relative(now, input, expected);
}

// Negative month operations with day overflow.
#[rstest]
#[case::march31_minus_1month("2024-03-31 00:00:00", "1 month ago", "2024-03-02 00:00:00+00:00")]
#[case::march31_minus_1month_nonleap(
    "2023-03-31 00:00:00",
    "1 month ago",
    "2023-03-03 00:00:00+00:00"
)]
#[case::may31_minus_1month("2024-05-31 00:00:00", "1 month ago", "2024-05-01 00:00:00+00:00")]
#[case::jan31_minus_1month("2024-01-31 00:00:00", "1 month ago", "2023-12-31 00:00:00+00:00")]
fn test_negative_month_operations(#[case] base: &str, #[case] input: &str, #[case] expected: &str) {
    let now = base
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    check_relative(now, input, expected);
}

// Chained relative operations.
#[rstest]
#[case::feb29_plus_year_plus_month(
    "2020-02-29 00:00:00",
    "1 year 1 month",
    "2021-04-01 00:00:00+00:00"
)]
#[case::jan31_plus_month_plus_year(
    "2024-01-31 00:00:00",
    "1 month 1 year",
    "2025-03-02 00:00:00+00:00"
)]
#[case::jan31_plus_2months_1day(
    "2024-01-31 00:00:00",
    "2 months 1 day",
    "2024-04-01 00:00:00+00:00"
)]
#[case::feb29_minus_year_plus_month(
    "2020-02-29 00:00:00",
    "1 year ago 1 month",
    "2019-04-01 00:00:00+00:00"
)]
#[case::complex_chain(
    "2024-01-31 12:30:45",
    "1 year 2 months 3 days 4 hours",
    "2025-04-03 16:30:45+00:00"
)]
fn test_chained_operations(#[case] base: &str, #[case] input: &str, #[case] expected: &str) {
    let now = base
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    check_relative(now, input, expected);
}

// Multiple month additions.
#[rstest]
#[case::jan31_plus_2months("2024-01-31 00:00:00", "2 months", "2024-03-31 00:00:00+00:00")]
#[case::jan31_plus_3months("2024-01-31 00:00:00", "3 months", "2024-05-01 00:00:00+00:00")]
#[case::jan31_plus_6months("2024-01-31 00:00:00", "6 months", "2024-07-31 00:00:00+00:00")]
#[case::jan31_plus_7months("2024-01-31 00:00:00", "7 months", "2024-08-31 00:00:00+00:00")]
#[case::aug31_plus_6months("2024-08-31 00:00:00", "6 months", "2025-03-03 00:00:00+00:00")]
#[case::may31_minus_3months_leap(
    "2024-05-31 00:00:00",
    "3 months ago",
    "2024-03-02 00:00:00+00:00"
)]
#[case::oct31_minus_8months_leap(
    "2024-10-31 00:00:00",
    "8 months ago",
    "2024-03-02 00:00:00+00:00"
)]
fn test_multiple_month_skip(#[case] base: &str, #[case] input: &str, #[case] expected: &str) {
    let now = base
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    check_relative(now, input, expected);
}

// Test embedded timezone handling (cross-TZ-mishandled)
// When TZ="..." is specified in input with a base date, apply the timezone to the base
// https://bugs.debian.org/851934#10
//
// NOTE: These tests were added without implementation changes.
// The timezone handling was already working correctly from previous commits.
// These tests document and verify the expected behavior for this edge case.
#[rstest]
#[case::utc_explicit(r#"TZ="UTC0" 1970-01-01 00:00"#, "1970-01-01 00:00:00+00:00")]
#[case::with_time(r#"TZ="EST5" 1970-01-01 12:30:45"#, "1970-01-01 12:30:45-05:00")]
#[case::iana_timezone(
    r#"TZ="America/New_York" 1970-01-01 00:00"#,
    "1970-01-01 00:00:00-05:00"
)]
// Bug #851934: timezone conversion case
// Parse date in Australia/Perth (AWST, UTC+8) and output should reflect that timezone
// Input: 2016-08-15 07:00 in Australia/Perth -> expected: 2016-08-15 07:00:00+08:00
#[case::perth_to_london(
    r#"TZ="Australia/Perth" 2016-08-15 07:00"#,
    "2016-08-15 07:00:00+08:00"
)]
fn test_embedded_timezone(#[case] input: &str, #[case] expected: &str) {
    check_absolute(input, expected);
}

// Zone abbreviations use their parser-defined offsets.
#[rstest]
#[case::bst("2026-06-15 12:00 BST", "2026-06-15 12:00:00+01:00")]
#[case::gst("2026-06-15 12:00 GST", "2026-06-15 12:00:00+10:00")]
#[case::ast("2026-06-15 12:00 AST", "2026-06-15 12:00:00-04:00")]
#[case::adt("2026-06-15 12:00 ADT", "2026-06-15 12:00:00-03:00")]
#[case::sst("2026-06-15 12:00 SST", "2026-06-15 12:00:00-12:00")]
fn test_zone_abbreviation_offsets(#[case] input: &str, #[case] expected: &str) {
    check_absolute(input, expected);
}

// Regression test for uutils/coreutils#12555.
// A fixed offset (e.g. the "UTC" keyword) must anchor the instant *before*
// relative adjustments are applied. Otherwise, when the base zone observes DST,
// adding a large relative offset drifts the wall-clock across a DST boundary
// and re-anchoring at the fixed offset bakes in the extra hour.
// "1970-01-01 UTC + N seconds" must equal exactly N seconds after the epoch,
// regardless of the base zone's DST rules.
#[test]
fn test_utc_keyword_plus_relative_seconds_across_dst() {
    // Base in a DST-observing zone (Europe/Berlin); the value only matters as
    // the default zone for interpreting the date, not the result.
    let base = "2020-06-15 12:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::get("Europe/Berlin").unwrap())
        .unwrap();

    // A timestamp landing in summer (CEST, UTC+2) while the epoch base is in
    // winter (CET, UTC+1), so the DST offset differs between base and result.
    let seconds = 1_780_318_971_i64;
    let input = format!("1970-01-01 UTC {seconds} seconds");

    let parsed = parse_datetime::parse_datetime_at_date(base, input)
        .unwrap()
        .expect_in_range();
    assert_eq!(parsed.timestamp().as_second(), seconds);
}

// `j` follows the base zone; other military letters use fixed offsets.
#[rstest]
#[case::est("2026-01-15 12:00:00", -5 * 3600)]
#[case::edt("2026-07-15 12:00:00", -4 * 3600)]
fn test_military_j_is_local_time(#[case] base: &str, #[case] expected_offset: i32) {
    let base = base
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::get("America/New_York").unwrap())
        .unwrap();

    let parsed = parse_datetime::parse_datetime_at_date(base.clone(), "8j")
        .unwrap()
        .expect_in_range();

    assert_eq!(parsed.hour(), 8, "`8j` should be 08:00 local");
    assert_eq!(
        parsed.offset().seconds(),
        expected_offset,
        "`8j` should take the base zone's offset"
    );

    // `z` is UTC.
    let utc = parse_datetime::parse_datetime_at_date(base, "8z")
        .unwrap()
        .expect_in_range();
    assert_eq!(utc.offset().seconds(), 0, "`8z` should be UTC");
}

#[rstest]
#[case::bare("j")]
#[case::attached("8j")]
#[case::uppercase("8J")]
#[case::spaced("8 j")]
#[case::leading("j 8")]
fn test_military_j_accepted(#[case] input: &str) {
    assert!(
        parse_datetime::parse_datetime(input).is_ok(),
        "`{input}` should parse"
    );
}

#[rstest]
// A second zone item is invalid.
#[case::j_then_utc("8j utc")]
#[case::utc_then_j("8 utc j")]
#[case::j_twice("8 j j")]
#[case::j_then_numeric("j +05:00")]
// The whole alphabetic word is matched, so `j` never steals a prefix.
#[case::doubled_letter("jj")]
#[case::doubled_letter_after_number("8 jj")]
fn test_military_j_rejected(#[case] input: &str) {
    assert!(
        parse_datetime::parse_datetime(input).is_err(),
        "`{input}` should be rejected"
    );
}

// A relative item preserves the base time of day with a zone item.
#[rstest]
#[case::local_zone_and_relative("j 1 day", "2026-08-30 11:30:45")]
#[case::relative_then_local_zone("5 minutes j", "2026-08-29 11:35:45")]
#[case::offset_and_relative("utc 1 day", "2026-08-30 11:30:45")]
#[case::relative_then_offset("5 minutes utc", "2026-08-29 11:35:45")]
#[case::timezone_rule_and_relative("TZ=\"Europe/Paris\" 1 day", "2026-08-30 17:30:45")]
// Without a relative item, a zone item still truncates to midnight.
#[case::local_zone_alone("j", "2026-08-29 00:00:00")]
#[case::local_zone_and_weekday("j monday", "2026-08-31 00:00:00")]
#[case::offset_alone("utc", "2026-08-29 00:00:00")]
fn test_zone_item_keeps_time_of_day_with_relative(#[case] input: &str, #[case] expected: &str) {
    let base = "2026-08-29 11:30:45"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::get("America/New_York").unwrap())
        .unwrap();

    let parsed = parse_datetime::parse_datetime_at_date(base, input)
        .unwrap()
        .expect_in_range();

    assert_eq!(
        parsed.datetime().to_string(),
        expected.parse::<DateTime>().unwrap().to_string(),
        "`{input}` should resolve to {expected} in its own zone"
    );
}

// An explicit date takes precedence over a weekday.
#[rstest]
#[case::mismatching_named_month("Wednesday August 17 2026", "2026-08-17 00:00:00")]
#[case::matching_named_month("Monday August 17 2026", "2026-08-17 00:00:00")]
#[case::abbreviated_with_comma("Wed, Aug 17 2026", "2026-08-17 00:00:00")]
#[case::day_month_with_comma("Wednesday, 17 August 2026", "2026-08-17 00:00:00")]
#[case::named_month_without_year("Wed Aug 17", "2026-08-17 00:00:00")]
#[case::iso_date("Wed 2026-08-17", "2026-08-17 00:00:00")]
#[case::iso_date_then_weekday("2026-08-17 Wed", "2026-08-17 00:00:00")]
#[case::us_date("Wed 08/17/2026", "2026-08-17 00:00:00")]
#[case::date_and_time("Wed 2026-08-17 10:30", "2026-08-17 10:30:00")]
#[case::time_before_date("Wed 10:30 2026-08-17", "2026-08-17 10:30:00")]
#[case::iso_datetime("2026-08-17T10:30:00 Wed", "2026-08-17 10:30:00")]
#[case::next_weekday("next Friday Sep 25 2026", "2026-09-25 00:00:00")]
#[case::next_other_weekday("next Sunday Sep 25 2026", "2026-09-25 00:00:00")]
#[case::last_weekday("last Wed 2026-08-17", "2026-08-17 00:00:00")]
#[case::ordinal_weekday("third Wed 2026-08-17", "2026-08-17 00:00:00")]
#[case::numeric_ordinal_weekday("2 Wed 2026-08-17", "2026-08-17 00:00:00")]
#[case::date_output_format("Sun Sep 25 12:00:00 AM UTC 2026", "2026-09-25 00:00:00")]
// Relative items still apply on top of the given date.
#[case::relative_day("Wed 2026-08-17 +1 day", "2026-08-18 00:00:00")]
#[case::relative_week("Wed 2026-08-17 next week", "2026-08-24 00:00:00")]
// Without a date, the weekday is still honoured.
#[case::weekday_alone("Wed", "2026-10-07 00:00:00")]
#[case::weekday_and_time("Wed 10:30", "2026-10-07 10:30:00")]
#[case::weekday_and_relative("Wed +1 day", "2026-10-08 00:00:00")]
fn test_weekday_ignored_with_explicit_date(#[case] input: &str, #[case] expected: &str) {
    let base = "2026-10-01 12:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();

    let parsed = parse_datetime::parse_datetime_at_date(base, input)
        .unwrap()
        .expect_in_range();

    assert_eq!(
        parsed.datetime().to_string(),
        expected.parse::<DateTime>().unwrap().to_string(),
        "`{input}` should resolve to {expected}"
    );
}

// Calendar items.
#[rstest]
#[case::short_year("01-11-14", "2001-11-14 00:00:00.000000000 +00:00")]
#[case::dotted_date("14.11.2022", "2022-11-14 00:00:00.000000000 +00:00")]
#[case::mixed_case("14 nOvEmBeR 2022", "2022-11-14 00:00:00.000000000 +00:00")]
#[case::spaced_iso("2022 - 11 - 14", "2022-11-14 00:00:00.000000000 +00:00")]
#[case::comments(
    "2022-(ignored)-11-(nested (comment))-14",
    "2022-11-14 00:00:00.000000000 +00:00"
)]
#[case::date_before_time("2022-11-14 06:07:08", "2022-11-14 06:07:08.000000000 +00:00")]
#[case::time_before_date("06:07:08 2022-11-14", "2022-11-14 06:07:08.000000000 +00:00")]
#[case::utc("2022-11-14 UTC", "2022-11-14 00:00:00.000000000 +00:00")]
fn test_additional_calendar_forms(#[case] input: &str, #[case] expected: &str) {
    check_canonical(input, expected);
}

#[rstest]
#[case::january("January", "2022-01-14 00:00:00.000000000 +00:00")]
#[case::february("February", "2022-02-14 00:00:00.000000000 +00:00")]
#[case::march("March", "2022-03-14 00:00:00.000000000 +00:00")]
#[case::april("April", "2022-04-14 00:00:00.000000000 +00:00")]
#[case::may("May", "2022-05-14 00:00:00.000000000 +00:00")]
#[case::june("June", "2022-06-14 00:00:00.000000000 +00:00")]
#[case::july("July", "2022-07-14 00:00:00.000000000 +00:00")]
#[case::august("August", "2022-08-14 00:00:00.000000000 +00:00")]
#[case::september("September", "2022-09-14 00:00:00.000000000 +00:00")]
#[case::october("October", "2022-10-14 00:00:00.000000000 +00:00")]
#[case::november("November", "2022-11-14 00:00:00.000000000 +00:00")]
#[case::december("December", "2022-12-14 00:00:00.000000000 +00:00")]
#[case::jan_abbreviation("Jan", "2022-01-14 00:00:00.000000000 +00:00")]
#[case::feb_abbreviation("Feb", "2022-02-14 00:00:00.000000000 +00:00")]
#[case::mar_abbreviation("Mar", "2022-03-14 00:00:00.000000000 +00:00")]
#[case::apr_abbreviation("Apr", "2022-04-14 00:00:00.000000000 +00:00")]
#[case::may_abbreviation("May", "2022-05-14 00:00:00.000000000 +00:00")]
#[case::jun_abbreviation("Jun", "2022-06-14 00:00:00.000000000 +00:00")]
#[case::jul_abbreviation("Jul", "2022-07-14 00:00:00.000000000 +00:00")]
#[case::aug_abbreviation("Aug", "2022-08-14 00:00:00.000000000 +00:00")]
#[case::sep_abbreviation("Sep", "2022-09-14 00:00:00.000000000 +00:00")]
#[case::sept_abbreviation("Sept", "2022-09-14 00:00:00.000000000 +00:00")]
#[case::oct_abbreviation("Oct", "2022-10-14 00:00:00.000000000 +00:00")]
#[case::nov_abbreviation("Nov", "2022-11-14 00:00:00.000000000 +00:00")]
#[case::dec_abbreviation("Dec", "2022-12-14 00:00:00.000000000 +00:00")]
fn test_literal_month_punctuation(#[case] month: &str, #[case] expected: &str) {
    check_canonical(&format!("{month} 14, 2022"), expected);
    check_canonical(&format!("14-{month}-2022"), expected);
}

#[rstest]
#[case::january("Jan", "2022-01-14 00:00:00.000000000 +00:00")]
#[case::february("Feb", "2022-02-14 00:00:00.000000000 +00:00")]
#[case::march("Mar", "2022-03-14 00:00:00.000000000 +00:00")]
#[case::april("Apr", "2022-04-14 00:00:00.000000000 +00:00")]
#[case::may("May", "2022-05-14 00:00:00.000000000 +00:00")]
#[case::june("Jun", "2022-06-14 00:00:00.000000000 +00:00")]
#[case::july("Jul", "2022-07-14 00:00:00.000000000 +00:00")]
#[case::august("Aug", "2022-08-14 00:00:00.000000000 +00:00")]
#[case::september("Sep", "2022-09-14 00:00:00.000000000 +00:00")]
#[case::october("Oct", "2022-10-14 00:00:00.000000000 +00:00")]
#[case::november("Nov", "2022-11-14 00:00:00.000000000 +00:00")]
#[case::december("Dec", "2022-12-14 00:00:00.000000000 +00:00")]
fn test_literal_month_period(#[case] month: &str, #[case] expected: &str) {
    check_canonical(&format!("14 {month}. 2022"), expected);
}

#[rstest]
#[case("2022-02-29")]
#[case("2023-02-29")]
#[case("2022-04-31")]
#[case("2022-00-01")]
#[case("2022-13-01")]
#[case("2022-01-00")]
#[case("2022-01-32")]
#[case("13/01/2022")]
#[case("04/31/2022")]
#[case("31.04.2022")]
fn test_invalid_calendar_forms(#[case] input: &str) {
    check_invalid(input);
}

#[rstest]
#[case::dotted("14.11.", "2024-11-14 00:00:00.000000000 +00:00")]
fn test_yearless_forms(#[case] input: &str, #[case] expected: &str) {
    check_canonical_at_date("2024-06-15 00:00:00", input, expected);
}

// Time of day and combined date/time items.
#[rstest]
#[case::hours_minutes_seconds("2024-02-29 20:02:00", "2024-02-29 20:02:00.000000000 +00:00")]
#[case::hours_minutes("2024-02-29 20:02", "2024-02-29 20:02:00.000000000 +00:00")]
#[case::one_digit_hour("2024-02-29 8:02", "2024-02-29 08:02:00.000000000 +00:00")]
#[case::one_digit_minute("2024-02-29 20:2", "2024-02-29 20:02:00.000000000 +00:00")]
#[case::fraction_dot("2024-02-29 20:02:00.1", "2024-02-29 20:02:00.100000000 +00:00")]
#[case::fraction_three_digits("2024-02-29 20:02:00.123", "2024-02-29 20:02:00.123000000 +00:00")]
#[case::fraction_nine_digits(
    "2024-02-29 20:02:00.123456789",
    "2024-02-29 20:02:00.123456789 +00:00"
)]
#[case::fraction_truncated(
    "2024-02-29 20:02:00.123456789987",
    "2024-02-29 20:02:00.123456789 +00:00"
)]
#[case::fraction_comma("2024-02-29 20:02:00,123456", "2024-02-29 20:02:00.123456000 +00:00")]
#[case::iso_t_separator("2024-02-29T20:02:00", "2024-02-29 20:02:00.000000000 +00:00")]
#[case::spaces_around_colons("2024-02-29 20 : 02 : 00", "2024-02-29 20:02:00.000000000 +00:00")]
#[case::comments_in_time(
    "2024-02-29 20:(comment)02:(nested (comment))00",
    "2024-02-29 20:02:00.000000000 +00:00"
)]
#[case::midnight("2024-02-29 00:00:00", "2024-02-29 00:00:00.000000000 +00:00")]
#[case::last_second("2024-02-29 23:59:59", "2024-02-29 23:59:59.000000000 +00:00")]
#[case::am("2024-02-29 12:34:56am", "2024-02-29 00:34:56.000000000 +00:00")]
#[case::pm("2024-02-29 12:34:56pm", "2024-02-29 12:34:56.000000000 +00:00")]
#[case::a_m_dotted("2024-02-29 12:34:56a.m.", "2024-02-29 00:34:56.000000000 +00:00")]
#[case::p_m_dotted("2024-02-29 12:34:56p.m.", "2024-02-29 12:34:56.000000000 +00:00")]
#[case::am_hours_only("2024-02-29 12am", "2024-02-29 00:00:00.000000000 +00:00")]
#[case::pm_hours_only("2024-02-29 12pm", "2024-02-29 12:00:00.000000000 +00:00")]
#[case::meridiem_with_spaces("2024-02-29 8 : 02 pm", "2024-02-29 20:02:00.000000000 +00:00")]
#[case::offset("2024-02-29 20:02-0500", "2024-03-01 01:02:00.000000000 +00:00")]
#[case::offset_colon("2024-02-29 20:02-05:00", "2024-03-01 01:02:00.000000000 +00:00")]
#[case::offset_hours("2024-02-29 20:02+5", "2024-02-29 15:02:00.000000000 +00:00")]
#[case::offset_hours_padded("2024-02-29 20:02+05", "2024-02-29 15:02:00.000000000 +00:00")]
#[case::offset_minutes("2024-02-29 20:02+0530", "2024-02-29 14:32:00.000000000 +00:00")]
#[case::offset_minutes_colon("2024-02-29 20:02+05:30", "2024-02-29 14:32:00.000000000 +00:00")]
#[case::offset_maximum("2024-02-29 20:02+24:00", "2024-02-28 20:02:00.000000000 +00:00")]
#[case::date_before_time("20:02:00 2024-02-29", "2024-02-29 20:02:00.000000000 +00:00")]
fn test_combined_date_time_forms(#[case] input: &str, #[case] expected: &str) {
    check_canonical(input, expected);
}

#[rstest]
#[case("2024-02-29 24:00")]
#[case("2024-02-29 23:60")]
#[case("2024-02-29 23:59:60")]
#[case("2024-02-29 13:00pm")]
#[case("2024-02-29 00:00am")]
#[case("2024-02-29 10:00am+01")]
#[case("2024-02-29 20:02+24:01")]
#[case("2024-02-29 20:02-24:01")]
#[case("2024-02-29 20:02+01+02")]
#[case("2024-02-29 20:")]
fn test_invalid_combined_date_time_forms(#[case] input: &str) {
    check_invalid(input);
}

#[rstest]
#[case::fixed_east(
    r#"TZ="<+0530>-5:30" 2024-01-15 12:00:00"#,
    "2024-01-15 06:30:00.000000000 +00:00"
)]
#[case::fixed_west(
    r#"TZ="<-07>+7" 2024-01-15 12:00:00"#,
    "2024-01-15 19:00:00.000000000 +00:00"
)]
#[ignore]
fn test_timezone_edge_cases(#[case] input: &str, #[case] expected: &str) {
    check_canonical(input, expected);
}

#[test]
#[ignore]
fn test_timezone_gap() {
    check_invalid(r#"TZ="America/New_York" 2024-03-10 02:30:00"#);
}

// Relative items.
#[rstest]
#[case::year("1 year", "2025-01-15 12:34:56.000000000 +00:00")]
#[case::years("2 years", "2026-01-15 12:34:56.000000000 +00:00")]
#[case::negative_year("-1 year", "2023-01-15 12:34:56.000000000 +00:00")]
#[case::year_ago("1 year ago", "2023-01-15 12:34:56.000000000 +00:00")]
#[case::year_hence("1 year hence", "2025-01-15 12:34:56.000000000 +00:00")]
#[case::month("1 month", "2024-02-15 12:34:56.000000000 +00:00")]
#[case::months("2 months", "2024-03-15 12:34:56.000000000 +00:00")]
#[case::negative_month("-1 month", "2023-12-15 12:34:56.000000000 +00:00")]
#[case::month_ago("1 month ago", "2023-12-15 12:34:56.000000000 +00:00")]
#[case::fortnight("1 fortnight", "2024-01-29 12:34:56.000000000 +00:00")]
#[case::fortnights("2 fortnights", "2024-02-12 12:34:56.000000000 +00:00")]
#[case::week("1 week", "2024-01-22 12:34:56.000000000 +00:00")]
#[case::weeks("2 weeks", "2024-01-29 12:34:56.000000000 +00:00")]
#[case::day("1 day", "2024-01-16 12:34:56.000000000 +00:00")]
#[case::days("2 days", "2024-01-17 12:34:56.000000000 +00:00")]
#[case::day_ago("1 day ago", "2024-01-14 12:34:56.000000000 +00:00")]
#[case::hour("1 hour", "2024-01-15 13:34:56.000000000 +00:00")]
#[case::hours("2 hours", "2024-01-15 14:34:56.000000000 +00:00")]
#[case::minute("1 minute", "2024-01-15 12:35:56.000000000 +00:00")]
#[case::min("2 min", "2024-01-15 12:36:56.000000000 +00:00")]
#[case::mins("2 mins", "2024-01-15 12:36:56.000000000 +00:00")]
#[case::second("1 second", "2024-01-15 12:34:57.000000000 +00:00")]
#[case::seconds("2 seconds", "2024-01-15 12:34:58.000000000 +00:00")]
#[case::sec("2 sec", "2024-01-15 12:34:58.000000000 +00:00")]
#[case::secs("2 secs", "2024-01-15 12:34:58.000000000 +00:00")]
#[case::tomorrow("tomorrow", "2024-01-16 12:34:56.000000000 +00:00")]
#[case::yesterday("yesterday", "2024-01-14 12:34:56.000000000 +00:00")]
#[case::today("today", "2024-01-15 12:34:56.000000000 +00:00")]
#[case::now("now", "2024-01-15 12:34:56.000000000 +00:00")]
#[case::this_day("this day", "2024-01-15 12:34:56.000000000 +00:00")]
#[case::this_second("this second", "2024-01-15 12:34:56.000000000 +00:00")]
#[case::signed_with_space("+ 3 hours", "2024-01-15 15:34:56.000000000 +00:00")]
#[case::negative_with_space("- 3 hours", "2024-01-15 09:34:56.000000000 +00:00")]
#[case::fractional_seconds("3.5 seconds", "2024-01-15 12:34:59.500000000 +00:00")]
#[case::negative_fractional_seconds("-3.5 seconds", "2024-01-15 12:34:52.500000000 +00:00")]
fn test_relative_items(#[case] input: &str, #[case] expected: &str) {
    check_canonical(&format!("2024-01-15 12:34:56 UTC {input}"), expected);
}

#[rstest]
#[case::leap_year(
    "2020-02-29 12:34:56 UTC 1 year",
    "2021-03-01 12:34:56.000000000 +00:00"
)]
#[case::leap_year_ago(
    "2020-02-29 12:34:56 UTC 1 year ago",
    "2019-03-01 12:34:56.000000000 +00:00"
)]
#[case::month_overflow(
    "2024-01-31 12:34:56 UTC 1 month",
    "2024-03-02 12:34:56.000000000 +00:00"
)]
#[case::month_overflow_ago(
    "2024-03-31 12:34:56 UTC 1 month ago",
    "2024-03-02 12:34:56.000000000 +00:00"
)]
#[case::month_then_day(
    "2024-01-31 12:34:56 UTC 1 month 2 days",
    "2024-03-04 12:34:56.000000000 +00:00"
)]
#[case::mixed_units(
    "2024-01-15 12:34:56 UTC 1 year 2 months 3 weeks 4 days 5 hours",
    "2025-04-09 17:34:56.000000000 +00:00"
)]
#[case::relative_before_time("1 day 12:34:56 2024-01-15", "2024-01-16 12:34:56.000000000 +00:00")]
#[case::relative_after_time("2024-01-15 12:34:56 1 day", "2024-01-16 12:34:56.000000000 +00:00")]
fn test_relative_combinations(#[case] input: &str, #[case] expected: &str) {
    check_canonical(input, expected);
}

#[test]
#[ignore]
fn test_relative_year_then_month() {
    check_canonical(
        "2020-02-29 12:34:56 UTC 1 year 1 month",
        "2021-03-29 12:34:56.000000000 +00:00",
    );
}

// Weekday items.
#[rstest]
#[case::sunday("Sunday")]
#[case::monday("Monday")]
#[case::tuesday("Tuesday")]
#[case::wednesday("Wednesday")]
#[case::thursday("Thursday")]
#[case::friday("Friday")]
#[case::saturday("Saturday")]
#[case::sun("Sun")]
#[case::mon("Mon")]
#[case::tue("Tue")]
#[case::tues("Tues")]
#[case::wed("Wed")]
#[case::wednes("Wednes")]
#[case::thu("Thu")]
#[case::thur("Thur")]
#[case::thurs("Thurs")]
#[case::fri("Fri")]
#[case::sat("Sat")]
fn test_weekday_forms(#[case] weekday: &str) {
    check_canonical(
        &format!("2024-01-15 12:34:56 UTC {weekday},"),
        "2024-01-15 12:34:56.000000000 +00:00",
    );
}

#[rstest]
#[case::next_monday("next monday", "2024-06-17 00:00:00.000000000 +00:00")]
#[case::last_monday("last monday", "2024-06-10 00:00:00.000000000 +00:00")]
#[case::this_monday("this monday", "2024-06-17 00:00:00.000000000 +00:00")]
#[case::third_monday("third monday", "2024-07-01 00:00:00.000000000 +00:00")]
#[case::fourth_monday("fourth monday", "2024-07-08 00:00:00.000000000 +00:00")]
#[case::first_monday("first monday", "2024-06-17 00:00:00.000000000 +00:00")]
#[case::numeric_second_monday("2 monday", "2024-06-24 00:00:00.000000000 +00:00")]
fn test_weekday_from_base(#[case] input: &str, #[case] expected: &str) {
    check_canonical_at_date("2024-06-15 12:34:56", input, expected);
}

// Pure numbers and epoch seconds.
#[rstest]
#[case::calendar_number("20221114", "2022-11-14 00:00:00.000000000 +00:00")]
#[case::calendar_number_with_time("20221114 1234", "2022-11-14 12:34:00.000000000 +00:00")]
#[case::calendar_number_with_short_time("20221114 12", "2022-11-14 12:00:00.000000000 +00:00")]
#[case::time_number_after_date("2024-11-14 1234", "2024-11-14 12:34:00.000000000 +00:00")]
#[case::time_number_after_date_short("2024-11-14 12", "2024-11-14 12:00:00.000000000 +00:00")]
#[case::numeric_year_override("Nov 14 12:34:56 2025", "2025-11-14 12:34:56.000000000 +00:00")]
#[case::epoch_zero("@0", "1970-01-01 00:00:00.000000000 +00:00")]
#[case::epoch_one("@1", "1970-01-01 00:00:01.000000000 +00:00")]
#[case::epoch_negative_one("@-1", "1969-12-31 23:59:59.000000000 +00:00")]
#[case::epoch_positive_fraction("@1.5", "1970-01-01 00:00:01.500000000 +00:00")]
#[case::epoch_negative_fraction("@-1.5", "1969-12-31 23:59:58.500000000 +00:00")]
#[case::epoch_comma_fraction("@1,25", "1970-01-01 00:00:01.250000000 +00:00")]
#[case::epoch_excess_precision("@1234567890.123456789987", "2009-02-13 23:31:30.123456789 +00:00")]
#[case::epoch_with_spaces("@ - 1.5", "1969-12-31 23:59:58.500000000 +00:00")]
fn test_pure_numbers_and_epochs(#[case] input: &str, #[case] expected: &str) {
    check_canonical(input, expected);
}

#[rstest]
#[case("@0 2024-01-01")]
#[case("@0 12:00")]
#[case("@0 1 day")]
#[case("@.5")]
#[case("@+")]
#[case("@1.5seconds")]
fn test_invalid_epochs(#[case] input: &str) {
    check_invalid(input);
}

// General syntax.
#[rstest]
#[case::mixed_case(
    "tHuRsDaY NoV 14 2024 8:02 p.M. UTC",
    "2024-11-14 20:02:00.000000000 +00:00"
)]
#[case::nested_comments(
    "2024-(outer (inner) comment)-11-14",
    "2024-11-14 00:00:00.000000000 +00:00"
)]
#[case::comments_between_time_items(
    "2024-11-14 08:(one)02:(two)03",
    "2024-11-14 08:02:03.000000000 +00:00"
)]
#[case::tabs_and_newlines("2024-11-14\t08:02:03\nUTC", "2024-11-14 08:02:03.000000000 +00:00")]
#[case::date_time_zone_order("UTC 08:02:03 2024-11-14", "2024-11-14 08:02:03.000000000 +00:00")]
#[case::time_date_zone_order("08:02:03 UTC 2024-11-14", "2024-11-14 08:02:03.000000000 +00:00")]
#[case::relative_date_order("1 day 2024-11-14", "2024-11-15 00:00:00.000000000 +00:00")]
#[case::date_relative_order("2024-11-14 1 day", "2024-11-15 00:00:00.000000000 +00:00")]
fn test_general_forms(#[case] input: &str, #[case] expected: &str) {
    check_canonical(input, expected);
}

#[rstest]
#[case("2024-11-(comment")]
#[case("2024-11-14)")]
#[case("14 Smarch 2024")]
#[case("Funday 2024-11-14")]
#[case("2024-11-14 1 blarg")]
#[case("2024-11-14 2024-11-15")]
#[case("2024-11-14 08:00 09:00")]
#[case("2024-11-14 trailing")]
fn test_invalid_general_forms(#[case] input: &str) {
    check_invalid(input);
}

#[test]
#[ignore]
fn test_uppercase_iso_month() {
    check_invalid("2024-NOVEMBER-14 8:02PM UTC");
}

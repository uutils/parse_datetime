// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use jiff::{civil::DateTime, tz::TimeZone};
use rstest::rstest;

mod common;
use common::{check_absolute, check_relative};

// The expected values are produced by GNU date version 8.32
// export LC_TIME=en_US.UTF-8
// export TZ=UTC
// date --rfc-3339=seconds --date="2022-11-14"
//
// Documentation for the date format can be found at:
// https://www.gnu.org/software/coreutils/manual/html_node/Calendar-date-items.html

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
/** TODO: https://github.com/uutils/parse_datetime/issues/160
#[case::year_10000("10000-12-31", "10000-12-31 00:00:00+00:00")]
#[case::year_100000("100000-12-31", "100000-12-31 00:00:00+00:00")]
#[case::year_1000000("1000000-12-31", "1000000-12-31 00:00:00+00:00")]
#[case::year_10000000("10000000-12-31", "10000000-12-31 00:00:00+00:00")]
#[case::max_date("2147485547-12-31", "2147485547-12-31 00:00:00+00:00")]
**/
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
fn test_tz_prefix_with_base_date(#[case] input: &str, #[case] expected: &str) {
    let base = "2020-06-15 12:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    check_relative(base, input, expected);
}

// Test leap year overflow: Feb 29 + years → non-leap year should overflow to March 1
// This matches GNU date behavior
#[rstest]
#[case::feb29_1996_plus_1year("1996-02-29 00:00:00", "1 year", "1997-03-01 00:00:00+00:00")]
#[case::feb29_2020_plus_1year("2020-02-29 00:00:00", "1 year", "2021-03-01 00:00:00+00:00")]
#[case::feb29_2000_plus_1year("2000-02-29 00:00:00", "1 year", "2001-03-01 00:00:00+00:00")]
// Edge case: 0 years should return the same date
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

// Test month arithmetic with day overflow
// Matches GNU date behavior: when adding months causes day clamping,
// overflow to next month (e.g., Jan 31 + 1 month = March 2/3, not Feb 28/29)
#[rstest]
#[case::jan31_plus_1month_leap("2024-01-31 00:00:00", "1 month", "2024-03-02 00:00:00+00:00")]
#[case::jan31_plus_1month_nonleap("2023-01-31 00:00:00", "1 month", "2023-03-03 00:00:00+00:00")]
#[case::mar31_plus_1month("2024-03-31 00:00:00", "1 month", "2024-05-01 00:00:00+00:00")]
#[case::may31_plus_1month("2024-05-31 00:00:00", "1 month", "2024-07-01 00:00:00+00:00")]
#[case::rel_2b("1997-01-19 08:17:48", "7 months ago", "1996-06-19 08:17:48+00:00")]
// Edge case: 0 months should return the same date
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

// Test negative year operations with leap year edge cases
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

// Test negative month operations with day overflow
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

// Test chained operations (multiple relative adjustments in one parse)
// These ensure that year and month overflow logic works correctly when combined
#[rstest]
// Feb 29, 2020 + 1 year = March 1, 2021; + 1 month = April 1, 2021
#[case::feb29_plus_year_plus_month(
    "2020-02-29 00:00:00",
    "1 year 1 month",
    "2021-04-01 00:00:00+00:00"
)]
// Jan 31, 2024 + 1 month = March 2, 2024; + 1 year = March 2, 2025
#[case::jan31_plus_month_plus_year(
    "2024-01-31 00:00:00",
    "1 month 1 year",
    "2025-03-02 00:00:00+00:00"
)]
// Jan 31 + 2 months + 1 day
#[case::jan31_plus_2months_1day(
    "2024-01-31 00:00:00",
    "2 months 1 day",
    "2024-04-01 00:00:00+00:00"
)]
// Feb 29 - 1 year + 1 month (March 1, 2019 + 1 month = April 1, 2019)
#[case::feb29_minus_year_plus_month(
    "2020-02-29 00:00:00",
    "1 year ago 1 month",
    "2019-04-01 00:00:00+00:00"
)]
// Multiple operations with days
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

// Test multiple month additions
// Verifies correct handling when adding multiple months at once
#[rstest]
// Jan 31 + 2 months: Jan 31 -> March 31 (no clamping, month has 31 days)
#[case::jan31_plus_2months("2024-01-31 00:00:00", "2 months", "2024-03-31 00:00:00+00:00")]
// Jan 31 + 3 months: Jan 31 -> April 30 (clamps), overflow to May 1
#[case::jan31_plus_3months("2024-01-31 00:00:00", "3 months", "2024-05-01 00:00:00+00:00")]
// Jan 31 + 6 months: Jan 31 -> July 31 (no overflow)
#[case::jan31_plus_6months("2024-01-31 00:00:00", "6 months", "2024-07-31 00:00:00+00:00")]
// Jan 31 + 7 months: Jan 31 -> Aug 31 (no overflow)
#[case::jan31_plus_7months("2024-01-31 00:00:00", "7 months", "2024-08-31 00:00:00+00:00")]
// Aug 31 + 6 months: Aug 31 -> Feb 28 (2025 non-leap), overflow to March 3
#[case::aug31_plus_6months("2024-08-31 00:00:00", "6 months", "2025-03-03 00:00:00+00:00")]
// May 31 - 3 months: May 31 -> Feb 29 (2024 leap), overflow to March 2
#[case::may31_minus_3months_leap(
    "2024-05-31 00:00:00",
    "3 months ago",
    "2024-03-02 00:00:00+00:00"
)]
// Oct 31 - 8 months: Oct 31 -> Feb 29 (2024 leap), overflow to March 2
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

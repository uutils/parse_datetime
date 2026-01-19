// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use core::panic;

use jiff::{civil::DateTime, tz::TimeZone, Zoned};
use parse_datetime::parse_datetime_at_date;
use rstest::rstest;

// The expected values are produced by GNU date version 8.32
// export LC_TIME=en_US.UTF-8
// export TZ=UTC
// date date --date="12:34:56+09:00" +"%H:%M:%S.%N"
//
// Documentation for the date format can be found at:
// https://www.gnu.org/software/coreutils/manual/html_node/Time-of-day-items.html

pub fn check_time(input: &str, expected: &str, format: &str, base: Option<Zoned>) {
    std::env::set_var("TZ", "UTC0");
    let now = base.unwrap_or(Zoned::now());
    let parsed = match parse_datetime_at_date(now, input) {
        Ok(v) => v,
        Err(e) => panic!("Failed to parse time from value '{input}': {e}"),
    }
    .with_time_zone(TimeZone::UTC);

    assert_eq!(
        format!("{}", parsed.strftime(format)),
        expected,
        "Input value: {input}"
    );
}

#[rstest]
#[case::full_time("12:34:56", "12:34:56.000000000")]
#[case::full_time_with_spaces("12 : 34 : 56", "12:34:56.000000000")]
#[case::full_time_midnight("00:00:00", "00:00:00.000000000")]
#[case::full_time_almost_midnight("23:59:59", "23:59:59.000000000")]
#[case::full_time_decimal_seconds("12:34:56.666", "12:34:56.666000000")]
#[case::full_time_decimal_seconds("12:34:56.999999999", "12:34:56.999999999")]
#[case::full_time_decimal_seconds("12:34:56.9999999999", "12:34:56.999999999")]
#[case::full_time_decimal_seconds_after_comma("12:34:56,666", "12:34:56.666000000")]
#[case::without_seconds("12:34", "12:34:00.000000000")]
fn test_time_24h_format(#[case] input: &str, #[case] expected: &str) {
    check_time(input, expected, "%H:%M:%S%.9f", None);
}

#[rstest]
#[case::full_time_am("12:34:56am", "00:34:56.000000000")]
#[case::full_time_pm("12:34:56pm", "12:34:56.000000000")]
#[case::full_time_am_with_dots("12:34:56a.m.", "00:34:56.000000000")]
#[case::full_time_pm_with_dots("12:34:56p.m.", "12:34:56.000000000")]
#[case::full_time_with_spaces("12 : 34 : 56 am", "00:34:56.000000000")]
#[case::full_time_capital("12:34:56pm", "12:34:56.000000000")]
#[case::full_time_midnight("00:00:00", "00:00:00.000000000")]
#[case::full_time_almost_midnight("23:59:59", "23:59:59.000000000")]
#[case::full_time_decimal_seconds("12:34:56.666pm", "12:34:56.666000000")]
#[case::full_time_decimal_seconds_after_comma("12:34:56,666pm", "12:34:56.666000000")]
#[case::without_seconds("12:34pm", "12:34:00.000000000")]
fn test_time_12h_format(#[case] input: &str, #[case] expected: &str) {
    check_time(input, expected, "%H:%M:%S%.9f", None);
}

#[rstest]
#[case::utc("12:34:56+00:00", "12:34:56.000000000")]
#[case::utc_with_minus("12:34:56-00:00", "12:34:56.000000000")]
#[case::corrected_plus("12:34:56+09:00", "03:34:56.000000000")]
#[case::corrected_minus("12:34:56-09:00", "21:34:56.000000000")]
#[case::corrected_no_colon("12:34:56+0900", "03:34:56.000000000")]
#[case::corrected_plus_hours_only("12:34:56+09", "03:34:56.000000000")]
#[case::corrected_minus_hours_only("12:34:56-09", "21:34:56.000000000")]
#[case::corrected_plus_minutes("12:34:56+09:12", "03:22:56.000000000")]
#[case::corrected_minus_minutes("12:34:56-09:26", "22:00:56.000000000")]
#[case::corrected_plus_single_digit("12:34:56+9", "03:34:56.000000000")]
#[case::corrected_minus_single_digit("12:34:56-9", "21:34:56.000000000")]
#[case::with_space("12:34:56 -09:00", "21:34:56.000000000")]
#[case::with_space("12:34:56 - 09:00", "21:34:56.000000000")]
#[case::with_space_only_hours("12:34:56 -09", "21:34:56.000000000")]
#[case::with_space_one_digit("12:34:56 -9", "21:34:56.000000000")]
#[case::gnu_compatibility("12:34:56+", "12:34:56.000000000")]
#[case::gnu_compatibility("12:34:56+-", "12:34:56.000000000")]
#[case::gnu_compatibility("12:34:56+-01", "13:34:56.000000000")]
#[case::gnu_compatibility("12:34:56+-+++---++", "12:34:56.000000000")]
#[case::gnu_compatibility("12:34:56+1-", "11:34:56.000000000")]
#[case::gnu_compatibility("12:34:56+--+1-+-", "11:34:56.000000000")]
fn test_time_correction(#[case] input: &str, #[case] expected: &str) {
    check_time(input, expected, "%H:%M:%S%.9f", None);
}

#[rstest]
#[case::plus_12("11:34:56+12:00", "2022-06-09 23:34:56")]
#[case::minus_12("12:34:56-12:00", "2022-06-11 00:34:56")]
#[case::plus_1259("12:34:56+12:59", "2022-06-09 23:35:56")]
#[case::minus_1259("12:34:56-12:59", "2022-06-11 01:33:56")]
#[case::plus_24("12:34:56+24:00", "2022-06-09 12:34:56")]
#[case::minus_24("12:34:56-24:00", "2022-06-11 12:34:56")]
#[case::plus_13("11:34:56+13:00", "2022-06-09 22:34:56")]
#[case::minus_13("12:34:56-13:00", "2022-06-11 01:34:56")]
#[case::plus_36("12:34:56 m+24", "2022-06-09 00:34:56")]
#[case::minus_36("12:34:56 y-24:00", "2022-06-12 00:34:56")]
fn test_time_correction_with_overflow(#[case] input: &str, #[case] expected: &str) {
    let now = "2022-06-10 00:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    check_time(input, expected, "%Y-%m-%d %H:%M:%S", Some(now));
}

#[rstest]
#[case("24:00:00")]
#[case("23:60:00")]
#[case("23:59:60")]
#[case("13:00:00am")]
#[case("13:00:00pm")]
#[case("00:00:00am")]
#[case("00:00:00pm")]
#[case("23:59:59 a.m")]
#[case("23:59:59 pm.")]
#[case("23:59:59+24:01")]
#[case("23:59:59-24:01")]
#[case("10:59am+01")]
#[case("10:59+01pm")]
#[case("23:59:59+00:00:00")]
fn test_time_invalid(#[case] input: &str) {
    let result = parse_datetime::parse_datetime(input);
    assert_eq!(
        result,
        Err(parse_datetime::ParseDateTimeError::InvalidInput),
        "Input string '{input}' did not produce an error when parsing"
    );
}

#[rstest]
#[case::months_ago_0("2026-01-12 0 Months ago", "2026-01-12")]
#[case::months_ago_negative_1("2026-01-12 -1 Months ago", "2026-02-12")]
#[case::months_ago_1("2026-01-12 1 Months ago", "2025-12-12")]
#[case::months_ago_2("2026-01-12 2 Months ago", "2025-11-12")]
#[case::months_ago_3("2026-01-12 3 Months ago", "2025-10-12")]
#[case::months_ago_4("2026-01-12 4 Months ago", "2025-09-12")]
#[case::months_ago_5("2026-01-12 5 Months ago", "2025-08-12")]
#[case::months_ago_6("2026-01-12 6 Months ago", "2025-07-12")]
#[case::months_ago_7("2026-01-12 7 Months ago", "2025-06-12")]
#[case::months_ago_8("2026-01-12 8 Months ago", "2025-05-12")]
#[case::months_ago_9("2026-01-12 9 Months ago", "2025-04-12")]
#[case::months_ago_10("2026-01-12 10 Months ago", "2025-03-12")]
#[case::months_ago_11("2026-01-12 11 Months ago", "2025-02-12")]
#[case::months_ago_12("2026-01-12 12 Months ago", "2025-01-12")]
#[case::months_ago_24("2026-01-12 24 Months ago", "2024-01-12")]
#[case::months_ago_36("2026-01-12 36 Months ago", "2023-01-12")]
#[case::months_ago_120("2026-01-12 120 Months ago", "2016-01-12")]
#[case::months_ago_240("2026-01-12 240 Months ago", "2006-01-12")]
fn test_time_months_ago(#[case] input: &str, #[case] expected: &str) {
    let now = "2026-01-12"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    check_time(input, expected, "%Y-%m-%d", Some(now));
}

#[rstest]
#[case::from_29_feb_in_1_month("2026-02-28 1 months", "2026-03-28")]
#[case::from_29_feb_1_month_ago("2026-02-28 1 months ago", "2026-01-28")]
#[case::from_31_jan_in_1_month("2026-01-31 1 months", "2026-03-03")]
#[case::from_31_march_1_month_ago("2026-03-31 1 months ago", "2026-03-03")]
#[case::from_15_march_1_month_ago("2026-03-15 1 months ago", "2026-02-15")]
#[case::from_15_jan_in_1_month("2026-01-15 1 months", "2026-02-15")]
fn test_relative_month_time_non_leap_year(#[case] input: &str, #[case] expected: &str) {
    let now = "2026-01-12"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    check_time(input, expected, "%Y-%m-%d", Some(now));
}

#[rstest]
#[case::from_31_dec_1_month_ago("2026-12-31 1 months ago", "2026-12-01")]
#[case::from_31_dec_3_month_ago("2026-12-31 3 months ago", "2026-10-01")]
#[case::from_31_mar_in_1_month("2026-03-31 1 months", "2026-05-01")]
#[case::from_31_mar_in_3_month("2026-03-31 3 months", "2026-07-01")]
fn test_relative_month_time_dest_month_does_not_have_the_day(
    #[case] input: &str,
    #[case] expected: &str,
) {
    let now = "2026-01-12"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    check_time(input, expected, "%Y-%m-%d", Some(now));
}

#[rstest]
#[case::from_29_feb_in_1_month("2024-02-29 1 months", "2024-03-29")]
#[case::from_29_feb_1_month_ago("2024-02-29 1 months ago", "2024-01-29")]
#[case::from_31_jan_in_1_month("2024-01-31 1 months", "2024-03-02")]
#[case::from_31_march_1_month_ago("2024-03-31 1 months ago", "2024-03-02")]
#[case::from_15_march_1_month_ago("2024-03-15 1 months ago", "2024-02-15")]
#[case::from_15_jan_in_1_month("2024-01-15 1 months", "2024-02-15")]
fn test_relative_month_time_leap_year(#[case] input: &str, #[case] expected: &str) {
    let now = "2026-01-12"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    check_time(input, expected, "%Y-%m-%d", Some(now));
}

#[rstest]
#[case::months_in_0_months("2026-01-12 0 months", "2026-01-12")]
#[case::months_in_negative_1_months("2026-01-12 -1 months", "2025-12-12")]
#[case::months_in_1_months("2026-01-12 1 months", "2026-02-12")]
#[case::months_in_2_months("2026-01-12 2 months", "2026-03-12")]
#[case::months_in_3_months("2026-01-12 3 months", "2026-04-12")]
#[case::months_in_4_months("2026-01-12 4 months", "2026-05-12")]
#[case::months_in_5_months("2026-01-12 5 months", "2026-06-12")]
#[case::months_in_6_months("2026-01-12 6 months", "2026-07-12")]
#[case::months_in_7_months("2026-01-12 7 months", "2026-08-12")]
#[case::months_in_8_months("2026-01-12 8 months", "2026-09-12")]
#[case::months_in_9_months("2026-01-12 9 months", "2026-10-12")]
#[case::months_in_10_months("2026-01-12 10 months", "2026-11-12")]
#[case::months_in_11_months("2026-01-12 11 months", "2026-12-12")]
#[case::months_in_12_months("2026-01-12 12 months", "2027-01-12")]
#[case::months_in_24_months("2026-01-12 24 months", "2028-01-12")]
#[case::months_in_36_months("2026-01-12 36 months", "2029-01-12")]
#[case::months_in_120_months("2026-01-12 120 months", "2036-01-12")]
#[case::months_in_240_months("2026-01-12 240 months", "2046-01-12")]
fn test_time_in_months(#[case] input: &str, #[case] expected: &str) {
    let now = "2026-01-12"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    check_time(input, expected, "%Y-%m-%d", Some(now));
}

#[rstest]
#[case::decimal_1_whole("1.123456789 seconds ago")]
#[case::decimal_2_whole("12.123456789 seconds ago")]
#[case::decimal_3_whole("123.123456789 seconds ago")]
#[case::decimal_4_whole("1234.123456789 seconds ago")]
#[case::decimal_5_whole("12345.123456789 seconds ago")]
#[case::decimal_6_whole("123456.123456789 seconds ago")]
#[case::decimal_7_whole("1234567.123456789 seconds ago")]
#[case::decimal_8_whole("12345678.123456789 seconds ago")]
#[case::decimal_9_whole("123456789.123456789 seconds ago")]
#[case::decimal_10_whole("1234567891.123456789 seconds ago")]
#[case::decimal_11_whole("12345678912.123456789 seconds ago")]
#[case::decimal_12_whole("123456789123.123456789 seconds ago")]
fn test_time_seconds_ago(#[case] input: &str) {
    let result = parse_datetime::parse_datetime(input);
    assert!(
        result.is_ok(),
        "Input string '{input}', produced {result:?}, instead of Ok(Zoned)"
    );
}

#[rstest]
#[case::decimal_13_whole("1234567891234.123456789 seconds ago")]
#[case::decimal_14_whole("12345678912345.123456789 seconds ago")]
#[case::decimal_15_whole("123456789123456.123456789 seconds ago")]
fn test_time_seconds_ago_invalid(#[case] input: &str) {
    let result = parse_datetime::parse_datetime(input);
    assert_eq!(
        result,
        Err(parse_datetime::ParseDateTimeError::InvalidInput),
        "Input string '{input}' did not produce an error when parsing"
    );
}

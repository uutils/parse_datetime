// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// use chrono::{DateTime, Local};
// use parse_datetime::parse_datetime_at_date;
// use rstest::rstest;

// // The expected values are produced by GNU date version 8.32
// // export LC_TIME=en_US.UTF-8
// // export TZ=UTC
// // date date --date="12:34:56+09:00" +"%H:%M:%S.%N"
// //
// // Documentation for the date format can be found at:
// // https://www.gnu.org/software/coreutils/manual/html_node/Time-of-day-items.html

// pub fn check_time(input: &str, expected: &str, format: &str, base: Option<DateTime<Local>>) {
//     std::env::set_var("TZ", "UTC0");
//     let now = base.unwrap_or_else(|| std::time::SystemTime::now().into());
//     let parsed = match parse_datetime_at_date(now, input) {
//         Ok(v) => v,
//         Err(e) => panic!("Failed to parse time from value '{input}': {e}"),
//     }
//     .to_utc();

//     assert_eq!(
//         &format!("{}", parsed.format(format)),
//         expected,
//         "Input value: {input}"
//     );
// }

// #[rstest]
// #[case::full_time("12:34:56", "12:34:56.000000000")]
// #[case::full_time_with_spaces("12 : 34 : 56", "12:34:56.000000000")]
// #[case::full_time_midnight("00:00:00", "00:00:00.000000000")]
// #[case::full_time_almost_midnight("23:59:59", "23:59:59.000000000")]
// #[case::full_time_decimal_seconds("12:34:56.666", "12:34:56.666000000")]
// #[case::full_time_decimal_seconds("12:34:56.999999999", "12:34:56.999999999")]
// #[case::full_time_decimal_seconds("12:34:56.9999999999", "12:34:56.999999999")]
// #[case::full_time_decimal_seconds_after_comma("12:34:56,666", "12:34:56.666000000")]
// #[case::without_seconds("12:34", "12:34:00.000000000")]
// fn test_time_24h_format(#[case] input: &str, #[case] expected: &str) {
//     check_time(input, expected, "%H:%M:%S%.9f", None);
// }

// #[rstest]
// #[case::full_time_am("12:34:56am", "00:34:56.000000000")]
// #[case::full_time_pm("12:34:56pm", "12:34:56.000000000")]
// #[case::full_time_am_with_dots("12:34:56a.m.", "00:34:56.000000000")]
// #[case::full_time_pm_with_dots("12:34:56p.m.", "12:34:56.000000000")]
// #[case::full_time_with_spaces("12 : 34 : 56 am", "00:34:56.000000000")]
// #[case::full_time_capital("12:34:56pm", "12:34:56.000000000")]
// #[case::full_time_midnight("00:00:00", "00:00:00.000000000")]
// #[case::full_time_almost_midnight("23:59:59", "23:59:59.000000000")]
// #[case::full_time_decimal_seconds("12:34:56.666pm", "12:34:56.666000000")]
// #[case::full_time_decimal_seconds_after_comma("12:34:56,666pm", "12:34:56.666000000")]
// #[case::without_seconds("12:34pm", "12:34:00.000000000")]
// fn test_time_12h_format(#[case] input: &str, #[case] expected: &str) {
//     check_time(input, expected, "%H:%M:%S%.9f", None);
// }

// #[rstest]
// #[case::utc("12:34:56+00:00", "12:34:56.000000000")]
// #[case::utc_with_minus("12:34:56-00:00", "12:34:56.000000000")]
// #[case::corrected_plus("12:34:56+09:00", "03:34:56.000000000")]
// #[case::corrected_minus("12:34:56-09:00", "21:34:56.000000000")]
// #[case::corrected_no_colon("12:34:56+0900", "03:34:56.000000000")]
// #[case::corrected_plus_hours_only("12:34:56+09", "03:34:56.000000000")]
// #[case::corrected_minus_hours_only("12:34:56-09", "21:34:56.000000000")]
// #[case::corrected_plus_minutes("12:34:56+09:12", "03:22:56.000000000")]
// #[case::corrected_minus_minutes("12:34:56-09:26", "22:00:56.000000000")]
// #[case::corrected_plus_single_digit("12:34:56+9", "03:34:56.000000000")]
// #[case::corrected_minus_single_digit("12:34:56-9", "21:34:56.000000000")]
// #[case::with_space("12:34:56 -09:00", "21:34:56.000000000")]
// #[case::with_space("12:34:56 - 09:00", "21:34:56.000000000")]
// #[case::with_space_only_hours("12:34:56 -09", "21:34:56.000000000")]
// #[case::with_space_one_digit("12:34:56 -9", "21:34:56.000000000")]
// #[case::gnu_compatibility("12:34:56+", "12:34:56.000000000")]
// #[case::gnu_compatibility("12:34:56+-", "12:34:56.000000000")]
// #[case::gnu_compatibility("12:34:56+-01", "13:34:56.000000000")]
// #[case::gnu_compatibility("12:34:56+-+++---++", "12:34:56.000000000")]
// #[case::gnu_compatibility("12:34:56+1-", "11:34:56.000000000")]
// #[case::gnu_compatibility("12:34:56+--+1-+-", "11:34:56.000000000")]
// fn test_time_correction(#[case] input: &str, #[case] expected: &str) {
//     check_time(input, expected, "%H:%M:%S%.9f", None);
// }

// #[rstest]
// #[case::plus_12("11:34:56+12:00", "2022-06-09 23:34:56")]
// #[case::minus_12("12:34:56-12:00", "2022-06-11 00:34:56")]
// #[case::plus_1259("12:34:56+12:59", "2022-06-09 23:35:56")]
// #[case::minus_1259("12:34:56-12:59", "2022-06-11 01:33:56")]
// /* TODO: https://github.com/uutils/parse_datetime/issues/149
// #[case::plus_24("12:34:56+24:00", "2022-06-09 12:34:56")]
// #[case::minus_24("12:34:56-24:00", "2022-06-11 12:34:56")]
// #[case::plus_13("11:34:56+13:00", "2022-06-09 22:34:56")]
// #[case::minus_13("12:34:56-13:00", "2022-06-11 01:34:56")]
// */
// fn test_time_correction_with_overflow(#[case] input: &str, #[case] expected: &str) {
//     let now = DateTime::parse_from_rfc3339("2022-06-10T00:00:00+00:00").unwrap();
//     check_time(input, expected, "%Y-%m-%d %H:%M:%S", Some(now.into()));
// }

// #[rstest]
// #[case("24:00:00")]
// #[case("23:60:00")]
// #[case("23:59:60")]
// #[case("13:00:00am")]
// #[case("13:00:00pm")]
// #[case("00:00:00am")]
// #[case("00:00:00pm")]
// #[case("23:59:59 a.m")]
// #[case("23:59:59 pm.")]
// #[case("23:59:59+24:01")]
// #[case("23:59:59-24:01")]
// #[case("10:59am+01")]
// #[case("10:59+01pm")]
// #[case("23:59:59+00:00:00")]
// fn test_time_invalid(#[case] input: &str) {
//     let result = parse_datetime::parse_datetime(input);
//     assert_eq!(
//         result,
//         Err(parse_datetime::ParseDateTimeError::InvalidInput),
//         "Input string '{input}' did not produce an error when parsing"
//     );
// }

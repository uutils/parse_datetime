// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use chrono::{Duration, Local, Weekday};
use parse_datetime::{
    parse_datetime_at_date, parse_relative_time::parse_relative_time,
    parse_timestamp::parse_timestamp, parse_weekday::parse_weekday,
};

#[test]
fn test_parse_datetime_at_date() {
    let now = Local::now();
    let after = parse_datetime_at_date(now, "+3 days");

    assert_eq!(
        (now + Duration::days(3)).naive_utc(),
        after.unwrap().naive_utc()
    );
}

#[test]
fn test_parse_relative_time() {
    let one_minute = parse_relative_time("1 minute").unwrap();
    assert_eq!(one_minute, Duration::seconds(60));
}

#[test]
fn test_parse_timestamp() {
    let ts = parse_timestamp("@1234").unwrap();
    assert_eq!(ts, 1234);
}

#[test]
fn test_weekday() {
    let mon = parse_weekday("monday").unwrap();
    assert_eq!(mon, Weekday::Mon);
}

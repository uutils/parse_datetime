use chrono::{DateTime, FixedOffset};
use parse_datetime::{parse_datetime, parse_datetime_at_date};

pub fn check_absolute(input: &str, expected: &str) {
    let parsed = match parse_datetime(input) {
        Ok(v) => v,
        Err(e) => panic!("Failed to parse date from value '{input}': {e}"),
    };

    assert_eq!(
        &parsed.to_rfc3339().replace("T", " "),
        expected,
        "Input value: {input}"
    );
}

pub fn check_relative(now: DateTime<FixedOffset>, input: &str, expected: &str) {
    let parsed = match parse_datetime_at_date(now.into(), input) {
        Ok(v) => v,
        Err(e) => panic!("Failed to parse date from value '{input}': {e}"),
    };
    let expected_parsed = DateTime::parse_from_rfc3339(expected).unwrap();
    assert_eq!(parsed, expected_parsed, "Input value: {input}");
}

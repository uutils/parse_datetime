use chrono::{Duration, Utc};
use humantime_to_duration::{from_str, from_str_at_date, ParseDurationError};

#[test]
fn test_invalid_input() {
    let result = from_str("foobar");
    println!("{result:?}");
    assert_eq!(result, Err(ParseDurationError::InvalidInput));

    let result = from_str("invalid 1");
    assert_eq!(result, Err(ParseDurationError::InvalidInput));
}

#[test]
fn test_duration_parsing() {
    assert_eq!(from_str("1 year").unwrap(), Duration::seconds(31_536_000));
    assert_eq!(
        from_str("-2 years").unwrap(),
        Duration::seconds(-63_072_000)
    );
    assert_eq!(
        from_str("2 years ago").unwrap(),
        Duration::seconds(-63_072_000)
    );
    assert_eq!(from_str("year").unwrap(), Duration::seconds(31_536_000));

    assert_eq!(from_str("1 month").unwrap(), Duration::seconds(2_592_000));
    assert_eq!(
        from_str("1 month and 2 weeks").unwrap(),
        Duration::seconds(3_801_600)
    );
    assert_eq!(
        from_str("1 month, 2 weeks").unwrap(),
        Duration::seconds(3_801_600)
    );
    assert_eq!(
        from_str("1 months 2 weeks").unwrap(),
        Duration::seconds(3_801_600)
    );
    assert_eq!(
        from_str("1 month and 2 weeks ago").unwrap(),
        Duration::seconds(-3_801_600)
    );
    assert_eq!(from_str("2 months").unwrap(), Duration::seconds(5_184_000));
    assert_eq!(from_str("month").unwrap(), Duration::seconds(2_592_000));

    assert_eq!(
        from_str("1 fortnight").unwrap(),
        Duration::seconds(1_209_600)
    );
    assert_eq!(
        from_str("3 fortnights").unwrap(),
        Duration::seconds(3_628_800)
    );
    assert_eq!(from_str("fortnight").unwrap(), Duration::seconds(1_209_600));

    assert_eq!(from_str("1 week").unwrap(), Duration::seconds(604_800));
    assert_eq!(
        from_str("1 week 3 days").unwrap(),
        Duration::seconds(864_000)
    );
    assert_eq!(
        from_str("1 week 3 days ago").unwrap(),
        Duration::seconds(-864_000)
    );
    assert_eq!(from_str("-2 weeks").unwrap(), Duration::seconds(-1_209_600));
    assert_eq!(
        from_str("2 weeks ago").unwrap(),
        Duration::seconds(-1_209_600)
    );
    assert_eq!(from_str("week").unwrap(), Duration::seconds(604_800));

    assert_eq!(from_str("1 day").unwrap(), Duration::seconds(86_400));
    assert_eq!(from_str("2 days ago").unwrap(), Duration::seconds(-172_800));
    assert_eq!(from_str("-2 days").unwrap(), Duration::seconds(-172_800));
    assert_eq!(from_str("day").unwrap(), Duration::seconds(86_400));

    assert_eq!(from_str("1 hour").unwrap(), Duration::seconds(3_600));
    assert_eq!(from_str("1 h").unwrap(), Duration::seconds(3_600));
    assert_eq!(from_str("1 hour ago").unwrap(), Duration::seconds(-3_600));
    assert_eq!(from_str("-2 hours").unwrap(), Duration::seconds(-7_200));
    assert_eq!(from_str("hour").unwrap(), Duration::seconds(3_600));

    assert_eq!(from_str("1 minute").unwrap(), Duration::seconds(60));
    assert_eq!(from_str("1 min").unwrap(), Duration::seconds(60));
    assert_eq!(from_str("2 minutes").unwrap(), Duration::seconds(120));
    assert_eq!(from_str("2 mins").unwrap(), Duration::seconds(120));
    assert_eq!(from_str("2m").unwrap(), Duration::seconds(120));
    assert_eq!(from_str("min").unwrap(), Duration::seconds(60));

    assert_eq!(from_str("1 second").unwrap(), Duration::seconds(1));
    assert_eq!(from_str("1 s").unwrap(), Duration::seconds(1));
    assert_eq!(from_str("2 seconds").unwrap(), Duration::seconds(2));
    assert_eq!(from_str("2 secs").unwrap(), Duration::seconds(2));
    assert_eq!(from_str("2 sec").unwrap(), Duration::seconds(2));
    assert_eq!(from_str("sec").unwrap(), Duration::seconds(1));

    assert_eq!(from_str("now").unwrap(), Duration::seconds(0));
    assert_eq!(from_str("today").unwrap(), Duration::seconds(0));

    assert_eq!(
        from_str("1 year 2 months 4 weeks 3 days and 2 seconds").unwrap(),
        Duration::seconds(39_398_402)
    );
    assert_eq!(
        from_str("1 year 2 months 4 weeks 3 days and 2 seconds ago").unwrap(),
        Duration::seconds(-39_398_402)
    );
}

#[test]
#[should_panic]
fn test_display_parse_duration_error_through_from_str() {
    let invalid_input = "9223372036854775807 seconds and 1 second";
    let _ = from_str(invalid_input).unwrap();
}

#[test]
fn test_display_should_fail() {
    let invalid_input = "Thu Jan 01 12:34:00 2015";
    let error = from_str(invalid_input).unwrap_err();

    assert_eq!(
        format!("{error}"),
        "Invalid input string: cannot be parsed as a relative time"
    );
}

#[test]
fn test_from_str_at_date_day() {
    let today = Utc::now().date_naive();
    let yesterday = today - Duration::days(1);
    assert_eq!(
        from_str_at_date(yesterday, "2 days").unwrap(),
        Duration::days(1)
    );
}

#[test]
fn test_invalid_input_at_date() {
    let today = Utc::now().date_naive();
    let result = from_str_at_date(today, "foobar");
    println!("{result:?}");
    assert_eq!(result, Err(ParseDurationError::InvalidInput));

    let result = from_str_at_date(today, "invalid 1r");
    assert_eq!(result, Err(ParseDurationError::InvalidInput));
}

use humantime_to_duration::{from_str, ParseDurationError};
use time::Duration;

#[test]
fn test_invalid_input() {
    let result = from_str("foobar");
    println!("{result:?}");
    match result {
        Err(ParseDurationError::InvalidInput) => assert!(true),
        _ => assert!(false),
    }

    let result = from_str("invalid 1");
    match result {
        Err(ParseDurationError::InvalidInput) => assert!(true),
        _ => assert!(false),
    }
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
    assert_eq!(from_str("1 hour ago").unwrap(), Duration::seconds(-3_600));
    assert_eq!(from_str("-2 hours").unwrap(), Duration::seconds(-7_200));
    assert_eq!(from_str("hour").unwrap(), Duration::seconds(3_600));

    assert_eq!(from_str("1 minute").unwrap(), Duration::seconds(60));
    assert_eq!(from_str("2 minutes").unwrap(), Duration::seconds(120));
    assert_eq!(from_str("min").unwrap(), Duration::seconds(60));

    assert_eq!(from_str("1 second").unwrap(), Duration::seconds(1));
    assert_eq!(from_str("2 seconds").unwrap(), Duration::seconds(2));
    assert_eq!(from_str("sec").unwrap(), Duration::seconds(1));

    assert_eq!(from_str("now").unwrap(), Duration::ZERO);
    assert_eq!(from_str("today").unwrap(), Duration::ZERO);

    assert_eq!(
        from_str("1 year 2 months 4 weeks 3 days and 2 seconds").unwrap(),
        Duration::seconds(39_398_402)
    );
}

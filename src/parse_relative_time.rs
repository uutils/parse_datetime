// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use crate::ParseDateTimeError;
use chrono::{Duration, Local, NaiveDate, Utc};
use regex::Regex;
/// Parses a relative time string and returns a `Duration` representing the
/// relative time.
///Regex
/// # Arguments
///
/// * `s` - A string slice representing the relative time.
///
///
/// # Supported formats
///
/// The function supports the following formats for relative time:
///
/// * `num` `unit` (e.g., "-1 hour", "+3 days")
/// * `unit` (e.g., "hour", "day")
/// * "now" or "today"
/// * "yesterday"
/// * "tomorrow"
/// * use "ago" for the past
///
/// `[num]` can be a positive or negative integer.
/// [unit] can be one of the following: "fortnight", "week", "day", "hour",
/// "minute", "min", "second", "sec" and their plural forms.
///
/// It is also possible to pass "1 hour 2 minutes" or "2 days and 2 hours"
///
/// # Returns
///
/// * `Ok(Duration)` - If the input string can be parsed as a relative time
/// * `Err(ParseDateTimeError)` - If the input string cannot be parsed as a relative time
///
/// # Errors
///
/// This function will return `Err(ParseDateTimeError::InvalidInput)` if the input string
/// cannot be parsed as a relative time.
///
/// ```
pub fn parse_relative_time(s: &str) -> Result<Duration, ParseDateTimeError> {
    parse_relative_time_at_date(Utc::now().date_naive(), s)
}

/// Parses a duration string and returns a `Duration` instance, with the duration
/// calculated from the specified date.
///
/// # Arguments
///
/// * `date` - A `Date` instance representing the base date for the calculation
/// * `s` - A string slice representing the relative time.
///
/// # Errors
///
/// This function will return `Err(ParseDateTimeError::InvalidInput)` if the input string
/// cannot be parsed as a relative time.
/// ```
pub fn parse_relative_time_at_date(
    date: NaiveDate,
    s: &str,
) -> Result<Duration, ParseDateTimeError> {
    let time_pattern: Regex = Regex::new(
        r"(?x)
        (?:(?P<value>[-+]?\d*)\s*)?
        (\s*(?P<direction>next|last)?\s*)?
        (?P<unit>years?|months?|fortnights?|weeks?|days?|hours?|h|minutes?|mins?|m|seconds?|secs?|s|yesterday|tomorrow|now|today)
        (\s*(?P<separator>and|,)?\s*)?
        (\s*(?P<ago>ago)?)?",
    )?;

    let mut total_duration = Duration::seconds(0);
    let mut is_ago = s.contains(" ago");
    let mut captures_processed = 0;
    let mut total_length = 0;

    for capture in time_pattern.captures_iter(s) {
        captures_processed += 1;

        let value_str = capture
            .name("value")
            .ok_or(ParseDateTimeError::InvalidInput)?
            .as_str();
        let value = if value_str.is_empty() {
            1
        } else {
            value_str
                .parse::<i64>()
                .map_err(|_| ParseDateTimeError::InvalidInput)?
        };

        if let Some(direction) = capture.name("direction") {
            if direction.as_str() == "last" {
                is_ago = true;
            }
        }

        let unit = capture
            .name("unit")
            .ok_or(ParseDateTimeError::InvalidInput)?
            .as_str();

        if capture.name("ago").is_some() {
            is_ago = true;
        }

        let duration = match unit {
            "years" | "year" => Duration::days(value * 365),
            "months" | "month" => Duration::days(value * 30),
            "fortnights" | "fortnight" => Duration::weeks(value * 2),
            "weeks" | "week" => Duration::weeks(value),
            "days" | "day" => Duration::days(value),
            "hours" | "hour" | "h" => Duration::hours(value),
            "minutes" | "minute" | "mins" | "min" | "m" => Duration::minutes(value),
            "seconds" | "second" | "secs" | "sec" | "s" => Duration::seconds(value),
            "yesterday" => Duration::days(-1),
            "tomorrow" => Duration::days(1),
            "now" | "today" => Duration::zero(),
            _ => return Err(ParseDateTimeError::InvalidInput),
        };
        let neg_duration = -duration;
        total_duration =
            match total_duration.checked_add(if is_ago { &neg_duration } else { &duration }) {
                Some(duration) => duration,
                None => return Err(ParseDateTimeError::InvalidInput),
            };

        // Calculate the total length of the matched substring
        if let Some(m) = capture.get(0) {
            total_length += m.end() - m.start();
        }
    }

    // Check if the entire input string has been captured
    if total_length != s.len() {
        return Err(ParseDateTimeError::InvalidInput);
    }

    if captures_processed == 0 {
        Err(ParseDateTimeError::InvalidInput)
    } else {
        let time_now = Local::now().date_naive();
        let date_duration = date - time_now;

        Ok(total_duration + date_duration)
    }
}

#[cfg(test)]
mod tests {

    use super::ParseDateTimeError;
    use super::{parse_relative_time, parse_relative_time_at_date};
    use chrono::{Duration, Local, NaiveDate, Utc};

    #[test]
    fn test_years() {
        assert_eq!(
            parse_relative_time("1 year").unwrap(),
            Duration::seconds(31_536_000)
        );
        assert_eq!(
            parse_relative_time("-2 years").unwrap(),
            Duration::seconds(-63_072_000)
        );
        assert_eq!(
            parse_relative_time("2 years ago").unwrap(),
            Duration::seconds(-63_072_000)
        );
        assert_eq!(
            parse_relative_time("year").unwrap(),
            Duration::seconds(31_536_000)
        );
    }

    #[test]
    fn test_months() {
        assert_eq!(
            parse_relative_time("1 month").unwrap(),
            Duration::seconds(2_592_000)
        );
        assert_eq!(
            parse_relative_time("1 month and 2 weeks").unwrap(),
            Duration::seconds(3_801_600)
        );
        assert_eq!(
            parse_relative_time("1 month and 2 weeks ago").unwrap(),
            Duration::seconds(-3_801_600)
        );
        assert_eq!(
            parse_relative_time("2 months").unwrap(),
            Duration::seconds(5_184_000)
        );
        assert_eq!(
            parse_relative_time("month").unwrap(),
            Duration::seconds(2_592_000)
        );
    }

    #[test]
    fn test_fortnights() {
        assert_eq!(
            parse_relative_time("1 fortnight").unwrap(),
            Duration::seconds(1_209_600)
        );
        assert_eq!(
            parse_relative_time("3 fortnights").unwrap(),
            Duration::seconds(3_628_800)
        );
        assert_eq!(
            parse_relative_time("fortnight").unwrap(),
            Duration::seconds(1_209_600)
        );
    }

    #[test]
    fn test_weeks() {
        assert_eq!(
            parse_relative_time("1 week").unwrap(),
            Duration::seconds(604_800)
        );
        assert_eq!(
            parse_relative_time("1 week 3 days").unwrap(),
            Duration::seconds(864_000)
        );
        assert_eq!(
            parse_relative_time("1 week 3 days ago").unwrap(),
            Duration::seconds(-864_000)
        );
        assert_eq!(
            parse_relative_time("-2 weeks").unwrap(),
            Duration::seconds(-1_209_600)
        );
        assert_eq!(
            parse_relative_time("2 weeks ago").unwrap(),
            Duration::seconds(-1_209_600)
        );
        assert_eq!(
            parse_relative_time("week").unwrap(),
            Duration::seconds(604_800)
        );
    }

    #[test]
    fn test_days() {
        assert_eq!(
            parse_relative_time("1 day").unwrap(),
            Duration::seconds(86400)
        );
        assert_eq!(
            parse_relative_time("2 days ago").unwrap(),
            Duration::seconds(-172_800)
        );
        assert_eq!(
            parse_relative_time("-2 days").unwrap(),
            Duration::seconds(-172_800)
        );
        assert_eq!(
            parse_relative_time("day").unwrap(),
            Duration::seconds(86400)
        );
    }

    #[test]
    fn test_hours() {
        assert_eq!(
            parse_relative_time("1 hour").unwrap(),
            Duration::seconds(3600)
        );
        assert_eq!(
            parse_relative_time("1 hour ago").unwrap(),
            Duration::seconds(-3600)
        );
        assert_eq!(
            parse_relative_time("-2 hours").unwrap(),
            Duration::seconds(-7200)
        );
        assert_eq!(
            parse_relative_time("hour").unwrap(),
            Duration::seconds(3600)
        );
    }

    #[test]
    fn test_minutes() {
        assert_eq!(
            parse_relative_time("1 minute").unwrap(),
            Duration::seconds(60)
        );
        assert_eq!(
            parse_relative_time("2 minutes").unwrap(),
            Duration::seconds(120)
        );
        assert_eq!(parse_relative_time("min").unwrap(), Duration::seconds(60));
    }

    #[test]
    fn test_seconds() {
        assert_eq!(
            parse_relative_time("1 second").unwrap(),
            Duration::seconds(1)
        );
        assert_eq!(
            parse_relative_time("2 seconds").unwrap(),
            Duration::seconds(2)
        );
        assert_eq!(parse_relative_time("sec").unwrap(), Duration::seconds(1));
    }

    #[test]
    fn test_relative_days() {
        assert_eq!(parse_relative_time("now").unwrap(), Duration::seconds(0));
        assert_eq!(parse_relative_time("today").unwrap(), Duration::seconds(0));
        assert_eq!(
            parse_relative_time("yesterday").unwrap(),
            Duration::seconds(-86400)
        );
        assert_eq!(
            parse_relative_time("tomorrow").unwrap(),
            Duration::seconds(86400)
        );
    }

    #[test]
    fn test_no_spaces() {
        assert_eq!(parse_relative_time("-1hour").unwrap(), Duration::hours(-1));
        assert_eq!(parse_relative_time("+3days").unwrap(), Duration::days(3));
        assert_eq!(parse_relative_time("2weeks").unwrap(), Duration::weeks(2));
        assert_eq!(
            parse_relative_time("2weeks 1hour").unwrap(),
            Duration::seconds(1_213_200)
        );
        assert_eq!(
            parse_relative_time("2weeks 1hour ago").unwrap(),
            Duration::seconds(-1_213_200)
        );
        assert_eq!(
            parse_relative_time("+4months").unwrap(),
            Duration::days(4 * 30)
        );
        assert_eq!(
            parse_relative_time("-2years").unwrap(),
            Duration::days(-2 * 365)
        );
        assert_eq!(
            parse_relative_time("15minutes").unwrap(),
            Duration::minutes(15)
        );
        assert_eq!(
            parse_relative_time("-30seconds").unwrap(),
            Duration::seconds(-30)
        );
        assert_eq!(
            parse_relative_time("30seconds ago").unwrap(),
            Duration::seconds(-30)
        );
    }

    #[test]
    fn test_invalid_input() {
        let result = parse_relative_time("foobar");
        println!("{result:?}");
        assert_eq!(result, Err(ParseDateTimeError::InvalidInput));

        let result = parse_relative_time("invalid 1");
        assert_eq!(result, Err(ParseDateTimeError::InvalidInput));
        // Fails for now with a panic
        /*        let result = parse_relative_time("777777777777777771m");
        match result {
            Err(ParseDateTimeError::InvalidInput) => assert!(true),
            _ => assert!(false),
        }*/
    }

    #[test]
    fn test_parse_relative_time_at_date() {
        let date = NaiveDate::from_ymd_opt(2014, 9, 5).unwrap();
        let now = Local::now().date_naive();
        let days_diff = (date - now).num_days();

        assert_eq!(
            parse_relative_time_at_date(date, "1 day").unwrap(),
            Duration::days(days_diff + 1)
        );

        assert_eq!(
            parse_relative_time_at_date(date, "2 hours").unwrap(),
            Duration::days(days_diff) + Duration::hours(2)
        );
    }

    #[test]
    fn test_invalid_input_at_date() {
        let date = NaiveDate::from_ymd_opt(2014, 9, 5).unwrap();
        assert!(matches!(
            parse_relative_time_at_date(date, "invalid"),
            Err(ParseDateTimeError::InvalidInput)
        ));
    }

    #[test]
    fn test_direction() {
        assert_eq!(
            parse_relative_time("last hour").unwrap(),
            Duration::seconds(-3600)
        );
        assert_eq!(
            parse_relative_time("next year").unwrap(),
            Duration::days(365)
        );
        assert_eq!(parse_relative_time("next week").unwrap(), Duration::days(7));
        assert_eq!(
            parse_relative_time("last month").unwrap(),
            Duration::days(-30)
        );
    }

    #[test]
    fn test_duration_parsing() {
        assert_eq!(
            parse_relative_time("1 year").unwrap(),
            Duration::seconds(31_536_000)
        );
        assert_eq!(
            parse_relative_time("-2 years").unwrap(),
            Duration::seconds(-63_072_000)
        );
        assert_eq!(
            parse_relative_time("2 years ago").unwrap(),
            Duration::seconds(-63_072_000)
        );
        assert_eq!(
            parse_relative_time("year").unwrap(),
            Duration::seconds(31_536_000)
        );

        assert_eq!(
            parse_relative_time("1 month").unwrap(),
            Duration::seconds(2_592_000)
        );
        assert_eq!(
            parse_relative_time("1 month and 2 weeks").unwrap(),
            Duration::seconds(3_801_600)
        );
        assert_eq!(
            parse_relative_time("1 month, 2 weeks").unwrap(),
            Duration::seconds(3_801_600)
        );
        assert_eq!(
            parse_relative_time("1 months 2 weeks").unwrap(),
            Duration::seconds(3_801_600)
        );
        assert_eq!(
            parse_relative_time("1 month and 2 weeks ago").unwrap(),
            Duration::seconds(-3_801_600)
        );
        assert_eq!(
            parse_relative_time("2 months").unwrap(),
            Duration::seconds(5_184_000)
        );
        assert_eq!(
            parse_relative_time("month").unwrap(),
            Duration::seconds(2_592_000)
        );

        assert_eq!(
            parse_relative_time("1 fortnight").unwrap(),
            Duration::seconds(1_209_600)
        );
        assert_eq!(
            parse_relative_time("3 fortnights").unwrap(),
            Duration::seconds(3_628_800)
        );
        assert_eq!(
            parse_relative_time("fortnight").unwrap(),
            Duration::seconds(1_209_600)
        );

        assert_eq!(
            parse_relative_time("1 week").unwrap(),
            Duration::seconds(604_800)
        );
        assert_eq!(
            parse_relative_time("1 week 3 days").unwrap(),
            Duration::seconds(864_000)
        );
        assert_eq!(
            parse_relative_time("1 week 3 days ago").unwrap(),
            Duration::seconds(-864_000)
        );
        assert_eq!(
            parse_relative_time("-2 weeks").unwrap(),
            Duration::seconds(-1_209_600)
        );
        assert_eq!(
            parse_relative_time("2 weeks ago").unwrap(),
            Duration::seconds(-1_209_600)
        );
        assert_eq!(
            parse_relative_time("week").unwrap(),
            Duration::seconds(604_800)
        );

        assert_eq!(
            parse_relative_time("1 day").unwrap(),
            Duration::seconds(86_400)
        );
        assert_eq!(
            parse_relative_time("2 days ago").unwrap(),
            Duration::seconds(-172_800)
        );
        assert_eq!(
            parse_relative_time("-2 days").unwrap(),
            Duration::seconds(-172_800)
        );
        assert_eq!(
            parse_relative_time("day").unwrap(),
            Duration::seconds(86_400)
        );

        assert_eq!(
            parse_relative_time("1 hour").unwrap(),
            Duration::seconds(3_600)
        );
        assert_eq!(
            parse_relative_time("1 h").unwrap(),
            Duration::seconds(3_600)
        );
        assert_eq!(
            parse_relative_time("1 hour ago").unwrap(),
            Duration::seconds(-3_600)
        );
        assert_eq!(
            parse_relative_time("-2 hours").unwrap(),
            Duration::seconds(-7_200)
        );
        assert_eq!(
            parse_relative_time("hour").unwrap(),
            Duration::seconds(3_600)
        );

        assert_eq!(
            parse_relative_time("1 minute").unwrap(),
            Duration::seconds(60)
        );
        assert_eq!(parse_relative_time("1 min").unwrap(), Duration::seconds(60));
        assert_eq!(
            parse_relative_time("2 minutes").unwrap(),
            Duration::seconds(120)
        );
        assert_eq!(
            parse_relative_time("2 mins").unwrap(),
            Duration::seconds(120)
        );
        assert_eq!(parse_relative_time("2m").unwrap(), Duration::seconds(120));
        assert_eq!(parse_relative_time("min").unwrap(), Duration::seconds(60));

        assert_eq!(
            parse_relative_time("1 second").unwrap(),
            Duration::seconds(1)
        );
        assert_eq!(parse_relative_time("1 s").unwrap(), Duration::seconds(1));
        assert_eq!(
            parse_relative_time("2 seconds").unwrap(),
            Duration::seconds(2)
        );
        assert_eq!(parse_relative_time("2 secs").unwrap(), Duration::seconds(2));
        assert_eq!(parse_relative_time("2 sec").unwrap(), Duration::seconds(2));
        assert_eq!(parse_relative_time("sec").unwrap(), Duration::seconds(1));

        assert_eq!(parse_relative_time("now").unwrap(), Duration::seconds(0));
        assert_eq!(parse_relative_time("today").unwrap(), Duration::seconds(0));

        assert_eq!(
            parse_relative_time("1 year 2 months 4 weeks 3 days and 2 seconds").unwrap(),
            Duration::seconds(39_398_402)
        );
        assert_eq!(
            parse_relative_time("1 year 2 months 4 weeks 3 days and 2 seconds ago").unwrap(),
            Duration::seconds(-39_398_402)
        );
    }

    #[test]
    #[should_panic]
    fn test_display_parse_duration_error_through_parse_relative_time() {
        let invalid_input = "9223372036854775807 seconds and 1 second";
        let _ = parse_relative_time(invalid_input).unwrap();
    }

    #[test]
    fn test_display_should_fail() {
        let invalid_input = "Thu Jan 01 12:34:00 2015";
        let error = parse_relative_time(invalid_input).unwrap_err();

        assert_eq!(
            format!("{error}"),
            "Invalid input string: cannot be parsed as a relative time"
        );
    }

    #[test]
    fn test_parse_relative_time_at_date_day() {
        let today = Utc::now().date_naive();
        let yesterday = today - Duration::days(1);
        assert_eq!(
            parse_relative_time_at_date(yesterday, "2 days").unwrap(),
            Duration::days(1)
        );
    }

    #[test]
    fn test_invalid_input_at_date_relative() {
        let today = Utc::now().date_naive();
        let result = parse_relative_time_at_date(today, "foobar");
        println!("{result:?}");
        assert_eq!(result, Err(ParseDateTimeError::InvalidInput));

        let result = parse_relative_time_at_date(today, "invalid 1r");
        assert_eq!(result, Err(ParseDateTimeError::InvalidInput));
    }
}

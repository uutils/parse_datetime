// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use crate::ParseDateTimeError;
use chrono::{DateTime, Days, Duration, Months, TimeZone, Utc};
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
    let now = Utc::now();
    let parsed = parse_relative_time_at_date(now, s)?;
    Ok(parsed - now)
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
fn parse_relative_time_at_date<T: TimeZone>(
    mut datetime: DateTime<T>,
    s: &str,
) -> Result<DateTime<T>, ParseDateTimeError> {
    let time_pattern: Regex = Regex::new(
        r"(?x)
        (?:(?P<value>[-+]?\d*)\s*)?
        (\s*(?P<direction>next|last)?\s*)?
        (?P<unit>years?|months?|fortnights?|weeks?|days?|hours?|h|minutes?|mins?|m|seconds?|secs?|s|yesterday|tomorrow|now|today)
        (\s*(?P<separator>and|,)?\s*)?
        (\s*(?P<ago>ago)?)?",
    )?;

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

        let new_datetime = match unit {
            "years" | "year" => add_months(datetime, value * 12, is_ago),
            "months" | "month" => add_months(datetime, value, is_ago),
            "fortnights" | "fortnight" => add_days(datetime, value * 14, is_ago),
            "weeks" | "week" => add_days(datetime, value * 7, is_ago),
            "days" | "day" => add_days(datetime, value, is_ago),
            "hours" | "hour" | "h" => add_duration(datetime, Duration::hours(value), is_ago),
            "minutes" | "minute" | "mins" | "min" | "m" => {
                add_duration(datetime, Duration::minutes(value), is_ago)
            }
            "seconds" | "second" | "secs" | "sec" | "s" => {
                add_duration(datetime, Duration::seconds(value), is_ago)
            }
            "yesterday" => add_days(datetime, 1, true),
            "tomorrow" => add_days(datetime, 1, false),
            "now" | "today" => Some(datetime),
            _ => return Err(ParseDateTimeError::InvalidInput),
        };
        datetime = match new_datetime {
            Some(dt) => dt,
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
        Ok(datetime)
    }
}

fn add_months<T: TimeZone>(
    datetime: DateTime<T>,
    months: i64,
    mut is_ago: bool,
) -> Option<DateTime<T>> {
    let months = if months < 0 {
        is_ago = !is_ago;
        u32::try_from(-months).ok()?
    } else {
        u32::try_from(months).ok()?
    };
    if is_ago {
        datetime.checked_sub_months(Months::new(months))
    } else {
        datetime.checked_add_months(Months::new(months))
    }
}

fn add_days<T: TimeZone>(
    datetime: DateTime<T>,
    days: i64,
    mut is_ago: bool,
) -> Option<DateTime<T>> {
    let days = if days < 0 {
        is_ago = !is_ago;
        u64::try_from(-days).ok()?
    } else {
        u64::try_from(days).ok()?
    };
    if is_ago {
        datetime.checked_sub_days(Days::new(days))
    } else {
        datetime.checked_add_days(Days::new(days))
    }
}

fn add_duration<T: TimeZone>(
    datetime: DateTime<T>,
    duration: Duration,
    is_ago: bool,
) -> Option<DateTime<T>> {
    let duration = if is_ago { -duration } else { duration };
    datetime.checked_add_signed(duration)
}

#[cfg(test)]
mod tests {
    use super::ParseDateTimeError;
    use super::{parse_relative_time, parse_relative_time_at_date};
    use chrono::{Days, Duration, Months, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};

    #[test]
    fn test_years() {
        let now = Utc::now();
        assert_eq!(
            now + parse_relative_time("1 year").unwrap(),
            now.checked_add_months(Months::new(12)).unwrap()
        );
        assert_eq!(
            now + parse_relative_time("-2 years").unwrap(),
            now.checked_sub_months(Months::new(24)).unwrap()
        );
        assert_eq!(
            now + parse_relative_time("2 years ago").unwrap(),
            now.checked_sub_months(Months::new(24)).unwrap()
        );
        assert_eq!(
            now + parse_relative_time("year").unwrap(),
            now.checked_add_months(Months::new(12)).unwrap()
        );
    }

    #[test]
    fn test_months() {
        let now = Utc::now();
        assert_eq!(
            now + parse_relative_time("1 month").unwrap(),
            now.checked_add_months(Months::new(1)).unwrap()
        );
        assert_eq!(
            now + parse_relative_time("1 month and 2 weeks").unwrap(),
            now.checked_add_months(Months::new(1))
                .unwrap()
                .checked_add_days(Days::new(14))
                .unwrap()
        );
        assert_eq!(
            now + parse_relative_time("1 month and 2 weeks ago").unwrap(),
            now.checked_sub_months(Months::new(1))
                .unwrap()
                .checked_sub_days(Days::new(14))
                .unwrap()
        );
        assert_eq!(
            now + parse_relative_time("2 months").unwrap(),
            now.checked_add_months(Months::new(2)).unwrap()
        );
        assert_eq!(
            now + parse_relative_time("month").unwrap(),
            now.checked_add_months(Months::new(1)).unwrap()
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
        let now = Utc::now();
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
            now + parse_relative_time("+4months").unwrap(),
            now.checked_add_months(Months::new(4)).unwrap()
        );
        assert_eq!(
            now + parse_relative_time("-2years").unwrap(),
            now.checked_sub_months(Months::new(24)).unwrap()
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
        let datetime = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2014, 9, 5).unwrap(),
            NaiveTime::from_hms_opt(0, 2, 3).unwrap(),
        ));
        let now = Utc::now();
        let diff = datetime - now;

        assert_eq!(
            parse_relative_time_at_date(datetime, "1 day").unwrap(),
            now + diff + Duration::days(1)
        );

        assert_eq!(
            parse_relative_time_at_date(datetime, "2 hours").unwrap(),
            now + diff + Duration::hours(2)
        );
    }

    #[test]
    fn test_direction() {
        let now = Utc::now();
        assert_eq!(
            parse_relative_time("last hour").unwrap(),
            Duration::seconds(-3600)
        );
        assert_eq!(
            now + parse_relative_time("next year").unwrap(),
            now.checked_add_months(Months::new(12)).unwrap()
        );
        assert_eq!(parse_relative_time("next week").unwrap(), Duration::days(7));
        assert_eq!(
            now + parse_relative_time("last month").unwrap(),
            now.checked_sub_months(Months::new(1)).unwrap()
        );
    }

    #[test]
    fn test_duration_parsing() {
        let now = Utc::now();
        assert_eq!(
            now + parse_relative_time("1 year").unwrap(),
            now.checked_add_months(Months::new(12)).unwrap()
        );
        assert_eq!(
            now + parse_relative_time("-2 years").unwrap(),
            now.checked_sub_months(Months::new(24)).unwrap()
        );
        assert_eq!(
            now + parse_relative_time("2 years ago").unwrap(),
            now.checked_sub_months(Months::new(24)).unwrap()
        );
        assert_eq!(
            now + parse_relative_time("year").unwrap(),
            now.checked_add_months(Months::new(12)).unwrap()
        );

        assert_eq!(
            now + parse_relative_time("1 month").unwrap(),
            now.checked_add_months(Months::new(1)).unwrap()
        );
        assert_eq!(
            now + parse_relative_time("1 month and 2 weeks").unwrap(),
            now.checked_add_months(Months::new(1))
                .unwrap()
                .checked_add_days(Days::new(14))
                .unwrap()
        );
        assert_eq!(
            now + parse_relative_time("1 month, 2 weeks").unwrap(),
            now.checked_add_months(Months::new(1))
                .unwrap()
                .checked_add_days(Days::new(14))
                .unwrap()
        );
        assert_eq!(
            now + parse_relative_time("1 months 2 weeks").unwrap(),
            now.checked_add_months(Months::new(1))
                .unwrap()
                .checked_add_days(Days::new(14))
                .unwrap()
        );
        assert_eq!(
            now + parse_relative_time("1 month and 2 weeks ago").unwrap(),
            now.checked_sub_months(Months::new(1))
                .unwrap()
                .checked_sub_days(Days::new(14))
                .unwrap()
        );
        assert_eq!(
            now + parse_relative_time("2 months").unwrap(),
            now.checked_add_months(Months::new(2)).unwrap()
        );
        assert_eq!(
            now + parse_relative_time("month").unwrap(),
            now.checked_add_months(Months::new(1)).unwrap()
        );

        assert_eq!(
            now + parse_relative_time("1 fortnight").unwrap(),
            now.checked_add_days(Days::new(14)).unwrap()
        );
        assert_eq!(
            now + parse_relative_time("3 fortnights").unwrap(),
            now.checked_add_days(Days::new(3 * 14)).unwrap()
        );
        assert_eq!(
            now + parse_relative_time("fortnight").unwrap(),
            now.checked_add_days(Days::new(14)).unwrap()
        );

        assert_eq!(
            now + parse_relative_time("1 week").unwrap(),
            now.checked_add_days(Days::new(7)).unwrap()
        );
        assert_eq!(
            now + parse_relative_time("1 week 3 days").unwrap(),
            now.checked_add_days(Days::new(7 + 3)).unwrap()
        );
        assert_eq!(
            now + parse_relative_time("1 week 3 days ago").unwrap(),
            now.checked_sub_days(Days::new(7 + 3)).unwrap()
        );
        assert_eq!(
            now + parse_relative_time("-2 weeks").unwrap(),
            now.checked_sub_days(Days::new(14)).unwrap()
        );
        assert_eq!(
            now + parse_relative_time("2 weeks ago").unwrap(),
            now.checked_sub_days(Days::new(14)).unwrap()
        );
        assert_eq!(
            now + parse_relative_time("week").unwrap(),
            now.checked_add_days(Days::new(7)).unwrap()
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
            now + parse_relative_time("1 year 2 months 4 weeks 3 days and 2 seconds").unwrap(),
            now.checked_add_months(Months::new(12 + 2))
                .unwrap()
                .checked_add_days(Days::new(4 * 7 + 3))
                .unwrap()
                .checked_add_signed(Duration::seconds(2))
                .unwrap()
        );
        assert_eq!(
            now + parse_relative_time("1 year 2 months 4 weeks 3 days and 2 seconds ago").unwrap(),
            now.checked_sub_months(Months::new(12 + 2))
                .unwrap()
                .checked_sub_days(Days::new(4 * 7 + 3))
                .unwrap()
                .checked_sub_signed(Duration::seconds(2))
                .unwrap()
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
        let now = Utc::now();
        let now_yesterday = now - Duration::days(1);
        assert_eq!(
            parse_relative_time_at_date(now_yesterday, "2 days").unwrap(),
            now + Duration::days(1)
        );
    }

    #[test]
    fn test_parse_relative_time_at_date_month() {
        // Use January because it has 31 days rather than 30
        let now = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));
        assert_eq!(
            parse_relative_time_at_date(now, "1 month").unwrap(),
            now.checked_add_months(Months::new(1)).unwrap()
        );
    }

    #[test]
    fn test_parse_relative_time_at_date_year() {
        // Use 2024 because it's a leap year and has 366 days rather than 365
        let now = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));
        assert_eq!(
            parse_relative_time_at_date(now, "1 year").unwrap(),
            now.checked_add_months(Months::new(12)).unwrap()
        );
    }

    #[test]
    fn test_invalid_input_at_date_relative() {
        let now = Utc::now();
        let result = parse_relative_time_at_date(now, "foobar");
        assert_eq!(result, Err(ParseDateTimeError::InvalidInput));

        let result = parse_relative_time_at_date(now, "invalid 1r");
        assert_eq!(result, Err(ParseDateTimeError::InvalidInput));
    }
}

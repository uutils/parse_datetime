// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// Expose parse_datetime
pub mod parse_datetime;

use chrono::{DateTime, Days, Duration, Months, TimeZone};
use regex::{Error as RegexError, Regex};
use std::error::Error;
use std::fmt::{self, Display};

#[derive(Debug, PartialEq)]
pub enum ParseDurationError {
    InvalidRegex(RegexError),
    InvalidInput,
}

impl Display for ParseDurationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseDurationError::InvalidRegex(err) => {
                write!(f, "Invalid regex for time pattern: {err}")
            }
            ParseDurationError::InvalidInput => {
                write!(
                    f,
                    "Invalid input string: cannot be parsed as a relative time"
                )
            }
        }
    }
}

impl Error for ParseDurationError {}

impl From<RegexError> for ParseDurationError {
    fn from(err: RegexError) -> Self {
        ParseDurationError::InvalidRegex(err)
    }
}

/// Adds a relative duration to the given date and returns the obtained date.
///
/// # Arguments
///
/// * `date` - A `DateTime` instance representing the base date for the calculation
/// * `s` - A string slice representing the relative time.
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
/// # Errors
///
/// This function will return `Err(ParseDurationError::InvalidInput)` if the input string
/// cannot be parsed as a relative time.
///
/// # Examples
///
/// ```
/// use chrono::{DateTime, Utc};
/// use parse_datetime::{add_relative_str};
/// let date: DateTime<Utc> = "2014-09-05 15:43:21Z".parse::<DateTime<Utc>>().unwrap();
/// assert_eq!(
///     add_relative_str(date, "4 months 25 days").unwrap().to_string(),
///     "2015-01-30 15:43:21 UTC"
/// );
/// ```
pub fn add_relative_str<Tz>(date: DateTime<Tz>, s: &str) -> Result<DateTime<Tz>, ParseDurationError>
where
    Tz: TimeZone,
{
    let time_pattern: Regex = Regex::new(
        r"(?x)
        (?:(?P<value>[-+]?\d*)\s*)?
        (\s*(?P<direction>next|last)?\s*)?
        (?P<unit>years?|months?|fortnights?|weeks?|days?|hours?|h|minutes?|mins?|m|seconds?|secs?|s|yesterday|tomorrow|now|today)
        (\s*(?P<separator>and|,)?\s*)?
        (\s*(?P<ago>ago)?)?",
    )?;

    let mut date = date.clone();
    let mut is_ago = s.contains(" ago");
    let mut captures_processed = 0;
    let mut total_length = 0;

    for capture in time_pattern.captures_iter(s) {
        captures_processed += 1;

        let value_str = capture
            .name("value")
            .ok_or(ParseDurationError::InvalidInput)?
            .as_str();
        let value = if value_str.is_empty() {
            1
        } else {
            value_str
                .parse::<i64>()
                .map_err(|_| ParseDurationError::InvalidInput)?
        };

        if let Some(direction) = capture.name("direction") {
            if direction.as_str() == "last" {
                is_ago = true;
            }
        }

        let unit = capture
            .name("unit")
            .ok_or(ParseDurationError::InvalidInput)?
            .as_str();

        if capture.name("ago").is_some() {
            is_ago = true;
        }
        let value = if is_ago { -value } else { value };

        let add_months = |date: DateTime<Tz>, months: i64| {
            if months.is_negative() {
                date - Months::new(months.unsigned_abs() as u32)
            } else {
                date + Months::new(months.unsigned_abs() as u32)
            }
        };
        let add_days = |date: DateTime<Tz>, days: i64| {
            if days.is_negative() {
                date - Days::new(days.unsigned_abs())
            } else {
                date + Days::new(days.unsigned_abs())
            }
        };

        date = match unit {
            "years" | "year" => add_months(date, 12 * value),
            "months" | "month" => add_months(date, value),
            "fortnights" | "fortnight" => add_days(date, 14 * value),
            "weeks" | "week" => add_days(date, 7 * value),
            "days" | "day" => add_days(date, value),
            "hours" | "hour" | "h" => date + Duration::hours(value),
            "minutes" | "minute" | "mins" | "min" | "m" => date + Duration::minutes(value),
            "seconds" | "second" | "secs" | "sec" | "s" => date + Duration::seconds(value),
            "yesterday" => add_days(date, -1),
            "tomorrow" => add_days(date, 1),
            "now" | "today" => date,
            _ => return Err(ParseDurationError::InvalidInput),
        };

        // Calculate the total length of the matched substring
        if let Some(m) = capture.get(0) {
            total_length += m.end() - m.start();
        }
    }

    // Check if the entire input string has been captured
    if total_length != s.len() {
        return Err(ParseDurationError::InvalidInput);
    }

    if captures_processed == 0 {
        Err(ParseDurationError::InvalidInput)
    } else {
        Ok(date)
    }
}

#[cfg(test)]
mod tests {

    use super::add_relative_str;
    use super::ParseDurationError;
    use chrono::{DateTime, Utc};

    #[test]
    fn test_years() {
        assert_add_relative_str_eq("1 year", "2015-09-05 15:43:21 UTC");
        assert_add_relative_str_eq("-2 years", "2012-09-05 15:43:21 UTC");
        assert_add_relative_str_eq("2 years ago", "2012-09-05 15:43:21 UTC");
        assert_add_relative_str_eq("year", "2015-09-05 15:43:21 UTC");
    }

    #[test]
    fn test_months() {
        assert_add_relative_str_eq("1 month", "2014-10-05 15:43:21 UTC");
        assert_add_relative_str_eq("1 month and 2 weeks", "2014-10-19 15:43:21 UTC");
        assert_add_relative_str_eq("1 month and 2 weeks ago", "2014-07-22 15:43:21 UTC");
        assert_add_relative_str_eq("2 months", "2014-11-05 15:43:21 UTC");
        assert_add_relative_str_eq("month", "2014-10-05 15:43:21 UTC");
    }

    #[test]
    fn test_fortnights() {
        assert_add_relative_str_eq("1 fortnight", "2014-09-19 15:43:21 UTC");
        assert_add_relative_str_eq("3 fortnights", "2014-10-17 15:43:21 UTC");
        assert_add_relative_str_eq("fortnight", "2014-09-19 15:43:21 UTC");
    }

    #[test]
    fn test_weeks() {
        assert_add_relative_str_eq("1 week", "2014-09-12 15:43:21 UTC");
        assert_add_relative_str_eq("1 week 3 days", "2014-09-15 15:43:21 UTC");
        assert_add_relative_str_eq("1 week 3 days ago", "2014-08-26 15:43:21 UTC");
        assert_add_relative_str_eq("-2 weeks", "2014-08-22 15:43:21 UTC");
        assert_add_relative_str_eq("2 weeks ago", "2014-08-22 15:43:21 UTC");
        assert_add_relative_str_eq("week", "2014-09-12 15:43:21 UTC");
    }

    #[test]
    fn test_days() {
        assert_add_relative_str_eq("1 day", "2014-09-06 15:43:21 UTC");
        assert_add_relative_str_eq("2 days ago", "2014-09-03 15:43:21 UTC");
        assert_add_relative_str_eq("-2 days", "2014-09-03 15:43:21 UTC");
        assert_add_relative_str_eq("day", "2014-09-06 15:43:21 UTC");
    }

    #[test]
    fn test_hours() {
        assert_add_relative_str_eq("1 hour", "2014-09-05 16:43:21 UTC");
        assert_add_relative_str_eq("1 hour ago", "2014-09-05 14:43:21 UTC");
        assert_add_relative_str_eq("-2 hours", "2014-09-05 13:43:21 UTC");
        assert_add_relative_str_eq("hour", "2014-09-05 16:43:21 UTC");
    }

    #[test]
    fn test_minutes() {
        assert_add_relative_str_eq("1 minute", "2014-09-05 15:44:21 UTC");
        assert_add_relative_str_eq("2 minutes", "2014-09-05 15:45:21 UTC");
        assert_add_relative_str_eq("min", "2014-09-05 15:44:21 UTC");
    }

    #[test]
    fn test_seconds() {
        assert_add_relative_str_eq("1 second", "2014-09-05 15:43:22 UTC");
        assert_add_relative_str_eq("2 seconds", "2014-09-05 15:43:23 UTC");
        assert_add_relative_str_eq("sec", "2014-09-05 15:43:22 UTC");
    }

    #[test]
    fn test_relative_days() {
        assert_add_relative_str_eq("now", "2014-09-05 15:43:21 UTC");
        assert_add_relative_str_eq("today", "2014-09-05 15:43:21 UTC");
        assert_add_relative_str_eq("yesterday", "2014-09-04 15:43:21 UTC");
        assert_add_relative_str_eq("tomorrow", "2014-09-06 15:43:21 UTC");
    }

    #[test]
    fn test_no_spaces() {
        assert_add_relative_str_eq("-1hour", "2014-09-05 14:43:21 UTC");
        assert_add_relative_str_eq("+3days", "2014-09-08 15:43:21 UTC");
        assert_add_relative_str_eq("2weeks", "2014-09-19 15:43:21 UTC");
        assert_add_relative_str_eq("2weeks 1hour", "2014-09-19 16:43:21 UTC");
        assert_add_relative_str_eq("2weeks 1hour ago", "2014-08-22 14:43:21 UTC");
        assert_add_relative_str_eq("+4months", "2015-01-05 15:43:21 UTC");
        assert_add_relative_str_eq("-2years", "2012-09-05 15:43:21 UTC");
        assert_add_relative_str_eq("15minutes", "2014-09-05 15:58:21 UTC");
        assert_add_relative_str_eq("-30seconds", "2014-09-05 15:42:51 UTC");
        assert_add_relative_str_eq("30seconds ago", "2014-09-05 15:42:51 UTC");
    }

    #[test]
    fn test_invalid_input() {
        let date: DateTime<Utc> = "2014-09-05 15:43:21Z".parse::<DateTime<Utc>>().unwrap();
        let result = add_relative_str(date, "foobar");
        assert_eq!(result, Err(ParseDurationError::InvalidInput));

        let result = add_relative_str(date, "invalid 1");
        assert_eq!(result, Err(ParseDurationError::InvalidInput));
        // Fails for now with a panic
        /*        let result = add_relative_str(date, "777777777777777771m");
        match result {
            Err(ParseDurationError::InvalidInput) => assert!(true),
            _ => assert!(false),
        }*/
    }

    #[test]
    fn test_add_relative_str() {
        assert_add_relative_str_eq("0 seconds", "2014-09-05 15:43:21 UTC");
        assert_add_relative_str_eq("1 day", "2014-09-06 15:43:21 UTC");
        assert_add_relative_str_eq("2 hours", "2014-09-05 17:43:21 UTC");
        assert_add_relative_str_eq("1 year ago", "2013-09-05 15:43:21 UTC");
        assert_add_relative_str_eq("1 year", "2015-09-05 15:43:21 UTC");
        assert_add_relative_str_eq("4 years", "2018-09-05 15:43:21 UTC");
        assert_add_relative_str_eq("2 months ago", "2014-07-05 15:43:21 UTC");
        assert_add_relative_str_eq("15 days ago", "2014-08-21 15:43:21 UTC");
        assert_add_relative_str_eq("1 week ago", "2014-08-29 15:43:21 UTC");
        assert_add_relative_str_eq("5 hours ago", "2014-09-05 10:43:21 UTC");
        assert_add_relative_str_eq("30 minutes ago", "2014-09-05 15:13:21 UTC");
        assert_add_relative_str_eq("10 seconds", "2014-09-05 15:43:31 UTC");
        assert_add_relative_str_eq("last hour", "2014-09-05 14:43:21 UTC");
        assert_add_relative_str_eq("next year", "2015-09-05 15:43:21 UTC");
        assert_add_relative_str_eq("next week", "2014-09-12 15:43:21 UTC");
        assert_add_relative_str_eq("last month", "2014-08-05 15:43:21 UTC");
        assert_add_relative_str_eq("4 months 25 days", "2015-01-30 15:43:21 UTC");
        assert_add_relative_str_eq("4 months 25 days 1 month", "2015-02-28 15:43:21 UTC");
        assert_add_relative_str_eq(
            "1 year 2 months 4 weeks 3 days and 2 seconds",
            "2015-12-06 15:43:23 UTC",
        );
        assert_add_relative_str_eq(
            "1 year 2 months 4 weeks 3 days and 2 seconds ago",
            "2013-06-04 15:43:19 UTC",
        );
    }

    /// Adds the given relative string to the date `2014-09-05 15:43:21 UTC` and compares it with the expected result.
    fn assert_add_relative_str_eq(str: &str, expected: &str) {
        let date: DateTime<Utc> = "2014-09-05 15:43:21 UTC".parse::<DateTime<Utc>>().unwrap();
        assert_eq!(
            (add_relative_str(date, str).unwrap()).to_string(),
            expected,
            "'{}' relative from {}",
            str,
            date
        );
    }
}

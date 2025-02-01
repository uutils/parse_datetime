// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use crate::ParseDateTimeError;
use chrono::{
    DateTime, Datelike, Days, Duration, LocalResult, Months, NaiveDate, NaiveDateTime, TimeZone,
};
use regex::Regex;

/// Number of days in each month.
///
/// Months are 0-indexed, so January is at index 0. The number of days
/// in February is 28.
const DAYS_PER_MONTH: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// Parses a relative time string and adds the duration that it represents to the
/// given date.
///
/// # Arguments
///
/// * `date` - A `Date` instance representing the base date for the calculation
/// * `s` - A string slice representing the relative time.
///
/// If `s` is empty, the `date` is returned as-is.
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
pub fn parse_relative_time_at_date<T: TimeZone>(
    mut datetime: DateTime<T>,
    s: &str,
) -> Result<DateTime<T>, ParseDateTimeError> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(datetime);
    }
    let time_pattern: Regex = Regex::new(
        r"(?x)
        (?:(?P<value>[-+]?\d*)\s*)?
        (\s*(?P<direction>next|this|last)?\s*)?
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

        let direction = capture.name("direction").map_or("", |d| d.as_str());

        if direction == "last" {
            is_ago = true;
        }

        let unit = capture
            .name("unit")
            .ok_or(ParseDateTimeError::InvalidInput)?
            .as_str();

        if capture.name("ago").is_some() {
            is_ago = true;
        }

        let new_datetime = if direction == "this" {
            add_days(datetime, 0, is_ago)
        } else {
            match unit {
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
                _ => None,
            }
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
        checked_add_months(datetime, months)
    }
}

/// Whether the given year is a leap year.
fn is_leap_year(year: i32) -> bool {
    NaiveDate::from_ymd_opt(year, 1, 1).is_some_and(|d| d.leap_year())
}

/// Get the number of days in the month in a particular year.
///
/// The year is required because February has 29 days in leap years.
fn days_in_month(year: i32, month: u32) -> u32 {
    if is_leap_year(year) && month == 2 {
        29
    } else {
        DAYS_PER_MONTH[month as usize - 1]
    }
}

/// Get the datetime at the given number of months ahead.
///
/// If the date is out of range or would be ambiguous (in the case of a
/// fold in the local time), return `None`.
///
/// If the day would be out of range in the new month (for example, if
/// `datetime` has day 31 but the resulting month only has 30 days),
/// then the surplus days are rolled over into the following month.
///
/// # Examples
///
/// Surplus days are rolled over
///
/// ```rust,ignore
/// use chrono::{NaiveDate, TimeZone, Utc};
/// let datetime = Utc.from_utc_datetime(
///     &NaiveDate::from_ymd_opt(1996, 3, 31).unwrap().into()
/// );
/// let new_datetime = checked_add_months(datetime, 1).unwrap();
/// assert_eq!(
///     new_datetime,
///     Utc.from_utc_datetime(&NaiveDate::from_ymd_opt(1996, 5, 1).unwrap().into()),
/// );
/// ```
fn checked_add_months<T>(datetime: DateTime<T>, months: u32) -> Option<DateTime<T>>
where
    T: TimeZone,
{
    // The starting date.
    let ref_year = datetime.year();
    let ref_month = datetime.month();
    let ref_date_in_months = 12 * ref_year + (ref_month as i32) - 1;

    // The year, month, and day of the target date.
    let target_date_in_months = ref_date_in_months.checked_add(months as i32)?;
    let year = target_date_in_months.div_euclid(12);
    let month = target_date_in_months.rem_euclid(12) + 1;
    let day = datetime.day();

    // Account for overflow when getting the correct day in the next
    // month. For example,
    //
    //     $ date -I --date '1996-01-31 +1 month'  # a leap year
    //     1996-03-02
    //     $ date -I --date '1997-01-31 +1 month'  # a non-leap year
    //     1997-03-03
    //
    let (month, day) = if day > days_in_month(year, month as u32) {
        (month + 1, day - days_in_month(year, month as u32))
    } else {
        (month, datetime.day())
    };

    // Create the new timezone-naive datetime.
    let new_date = NaiveDate::from_ymd_opt(year, month as u32, day)?;
    let time = datetime.time();
    let new_naive_datetime = NaiveDateTime::new(new_date, time);

    // Make it timezone-aware.
    let offset = T::from_offset(datetime.offset());
    match offset.from_local_datetime(&new_naive_datetime) {
        LocalResult::Single(d) => Some(d),
        LocalResult::Ambiguous(_, _) | LocalResult::None => None,
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
    use super::parse_relative_time_at_date;
    use super::ParseDateTimeError;
    use chrono::{Days, Duration, Months, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};

    fn parse_duration(s: &str) -> Result<Duration, ParseDateTimeError> {
        let now = Utc::now();
        let parsed = parse_relative_time_at_date(now, s)?;
        Ok(parsed - now)
    }

    #[test]
    fn test_empty_string() {
        let now = Utc::now();
        assert_eq!(parse_relative_time_at_date(now, "").unwrap(), now);
    }

    #[test]
    fn test_years() {
        let now = Utc::now();
        assert_eq!(
            parse_relative_time_at_date(now, "1 year").unwrap(),
            now.checked_add_months(Months::new(12)).unwrap()
        );
        assert_eq!(parse_relative_time_at_date(now, "this year").unwrap(), now);
        assert_eq!(
            parse_relative_time_at_date(now, "-2 years").unwrap(),
            now.checked_sub_months(Months::new(24)).unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "2 years ago").unwrap(),
            now.checked_sub_months(Months::new(24)).unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "year").unwrap(),
            now.checked_add_months(Months::new(12)).unwrap()
        );
    }

    #[test]
    fn test_leap_day() {
        // $ date -I --date '1996-02-29 +1 year'
        // 1997-03-01
        // $ date -I --date '1996-02-29 +12 months'
        // 1997-03-01
        let datetime = Utc.from_utc_datetime(&NaiveDate::from_ymd_opt(1996, 2, 29).unwrap().into());
        let expected = Utc.from_utc_datetime(&NaiveDate::from_ymd_opt(1997, 3, 1).unwrap().into());
        let parse = |s| parse_relative_time_at_date(datetime, s).unwrap();
        assert_eq!(parse("+1 year"), expected);
        assert_eq!(parse("+12 months"), expected);
        assert_eq!(parse("+366 days"), expected);
    }

    #[test]
    fn test_months() {
        use crate::parse_relative_time::add_months;

        let now = Utc::now();
        assert_eq!(
            parse_relative_time_at_date(now, "1 month").unwrap(),
            add_months(now, 1, false).unwrap(),
        );
        assert_eq!(parse_relative_time_at_date(now, "this month").unwrap(), now);
        assert_eq!(
            parse_relative_time_at_date(now, "1 month and 2 weeks").unwrap(),
            add_months(now, 1, false)
                .unwrap()
                .checked_add_days(Days::new(14))
                .unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "1 month and 2 weeks ago").unwrap(),
            add_months(now, 1, true)
                .unwrap()
                .checked_sub_days(Days::new(14))
                .unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "2 months").unwrap(),
            now.checked_add_months(Months::new(2)).unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "month").unwrap(),
            add_months(now, 1, false).unwrap(),
        );
    }

    #[test]
    fn test_overflow_days_in_month() {
        // $ date -I --date '1996-03-31 1 month'
        // 1996-05-01
        let datetime = Utc.from_utc_datetime(&NaiveDate::from_ymd_opt(1996, 3, 31).unwrap().into());
        let expected = Utc.from_utc_datetime(&NaiveDate::from_ymd_opt(1996, 5, 1).unwrap().into());
        let parse = |s| parse_relative_time_at_date(datetime, s).unwrap();
        assert_eq!(parse("1 month"), expected);
    }

    #[test]
    fn test_fortnights() {
        assert_eq!(
            parse_duration("1 fortnight").unwrap(),
            Duration::seconds(1_209_600)
        );
        assert_eq!(
            parse_duration("this fortnight").unwrap(),
            Duration::seconds(0)
        );
        assert_eq!(
            parse_duration("3 fortnights").unwrap(),
            Duration::seconds(3_628_800)
        );
        assert_eq!(
            parse_duration("fortnight").unwrap(),
            Duration::seconds(1_209_600)
        );
    }

    #[test]
    fn test_weeks() {
        assert_eq!(
            parse_duration("1 week").unwrap(),
            Duration::seconds(604_800)
        );
        assert_eq!(parse_duration("this week").unwrap(), Duration::seconds(0));
        assert_eq!(
            parse_duration("1 week 3 days").unwrap(),
            Duration::seconds(864_000)
        );
        assert_eq!(
            parse_duration("1 week 3 days ago").unwrap(),
            Duration::seconds(-864_000)
        );
        assert_eq!(
            parse_duration("-2 weeks").unwrap(),
            Duration::seconds(-1_209_600)
        );
        assert_eq!(
            parse_duration("2 weeks ago").unwrap(),
            Duration::seconds(-1_209_600)
        );
        assert_eq!(parse_duration("week").unwrap(), Duration::seconds(604_800));
    }

    #[test]
    fn test_days() {
        assert_eq!(parse_duration("1 day").unwrap(), Duration::seconds(86400));
        assert_eq!(
            parse_duration("2 days ago").unwrap(),
            Duration::seconds(-172_800)
        );
        assert_eq!(parse_duration("this day").unwrap(), Duration::seconds(0));
        assert_eq!(
            parse_duration("-2 days").unwrap(),
            Duration::seconds(-172_800)
        );
        assert_eq!(parse_duration("day").unwrap(), Duration::seconds(86400));
    }

    #[test]
    fn test_hours() {
        assert_eq!(parse_duration("1 hour").unwrap(), Duration::seconds(3600));
        assert_eq!(
            parse_duration("1 hour ago").unwrap(),
            Duration::seconds(-3600)
        );
        assert_eq!(parse_duration("this hour").unwrap(), Duration::seconds(0));
        assert_eq!(
            parse_duration("-2 hours").unwrap(),
            Duration::seconds(-7200)
        );
        assert_eq!(parse_duration("hour").unwrap(), Duration::seconds(3600));
    }

    #[test]
    fn test_minutes() {
        assert_eq!(parse_duration("this minute").unwrap(), Duration::seconds(0));
        assert_eq!(parse_duration("1 minute").unwrap(), Duration::seconds(60));
        assert_eq!(parse_duration("2 minutes").unwrap(), Duration::seconds(120));
        assert_eq!(parse_duration("min").unwrap(), Duration::seconds(60));
    }

    #[test]
    fn test_seconds() {
        assert_eq!(parse_duration("this second").unwrap(), Duration::seconds(0));
        assert_eq!(parse_duration("1 second").unwrap(), Duration::seconds(1));
        assert_eq!(parse_duration("2 seconds").unwrap(), Duration::seconds(2));
        assert_eq!(parse_duration("sec").unwrap(), Duration::seconds(1));
    }

    #[test]
    fn test_relative_days() {
        assert_eq!(parse_duration("now").unwrap(), Duration::seconds(0));
        assert_eq!(parse_duration("today").unwrap(), Duration::seconds(0));
        assert_eq!(
            parse_duration("yesterday").unwrap(),
            Duration::seconds(-86400)
        );
        assert_eq!(
            parse_duration("tomorrow").unwrap(),
            Duration::seconds(86400)
        );
    }

    #[test]
    fn test_no_spaces() {
        let now = Utc::now();
        assert_eq!(parse_duration("-1hour").unwrap(), Duration::hours(-1));
        assert_eq!(parse_duration("+3days").unwrap(), Duration::days(3));
        assert_eq!(parse_duration("2weeks").unwrap(), Duration::weeks(2));
        assert_eq!(
            parse_duration("2weeks 1hour").unwrap(),
            Duration::seconds(1_213_200)
        );
        assert_eq!(
            parse_duration("2weeks 1hour ago").unwrap(),
            Duration::seconds(-1_213_200)
        );
        assert_eq!(parse_duration("thismonth").unwrap(), Duration::days(0));
        assert_eq!(
            parse_relative_time_at_date(now, "+4months").unwrap(),
            now.checked_add_months(Months::new(4)).unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "-2years").unwrap(),
            now.checked_sub_months(Months::new(24)).unwrap()
        );
        assert_eq!(parse_duration("15minutes").unwrap(), Duration::minutes(15));
        assert_eq!(
            parse_duration("-30seconds").unwrap(),
            Duration::seconds(-30)
        );
        assert_eq!(
            parse_duration("30seconds ago").unwrap(),
            Duration::seconds(-30)
        );
    }

    #[test]
    fn test_invalid_input() {
        let result = parse_duration("foobar");
        println!("{result:?}");
        assert_eq!(result, Err(ParseDateTimeError::InvalidInput));

        let result = parse_duration("invalid 1");
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
            parse_duration("last hour").unwrap(),
            Duration::seconds(-3600)
        );
        assert_eq!(
            parse_relative_time_at_date(now, "next year").unwrap(),
            now.checked_add_months(Months::new(12)).unwrap()
        );
        assert_eq!(parse_duration("next week").unwrap(), Duration::days(7));
        assert_eq!(
            parse_relative_time_at_date(now, "last month").unwrap(),
            now.checked_sub_months(Months::new(1)).unwrap()
        );

        assert_eq!(parse_duration("this month").unwrap(), Duration::days(0));

        assert_eq!(parse_duration("this year").unwrap(), Duration::days(0));
    }

    #[test]
    fn test_duration_parsing() {
        use crate::parse_relative_time::add_months;

        let now = Utc::now();
        assert_eq!(
            parse_relative_time_at_date(now, "1 year").unwrap(),
            now.checked_add_months(Months::new(12)).unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "-2 years").unwrap(),
            now.checked_sub_months(Months::new(24)).unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "2 years ago").unwrap(),
            now.checked_sub_months(Months::new(24)).unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "year").unwrap(),
            now.checked_add_months(Months::new(12)).unwrap()
        );

        assert_eq!(
            parse_relative_time_at_date(now, "1 month").unwrap(),
            add_months(now, 1, false).unwrap(),
        );
        assert_eq!(
            parse_relative_time_at_date(now, "1 month and 2 weeks").unwrap(),
            add_months(now, 1, false)
                .unwrap()
                .checked_add_days(Days::new(14))
                .unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "1 month, 2 weeks").unwrap(),
            add_months(now, 1, false)
                .unwrap()
                .checked_add_days(Days::new(14))
                .unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "1 months 2 weeks").unwrap(),
            add_months(now, 1, false)
                .unwrap()
                .checked_add_days(Days::new(14))
                .unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "1 month and 2 weeks ago").unwrap(),
            now.checked_sub_months(Months::new(1))
                .unwrap()
                .checked_sub_days(Days::new(14))
                .unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "2 months").unwrap(),
            now.checked_add_months(Months::new(2)).unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "month").unwrap(),
            add_months(now, 1, false).unwrap(),
        );

        assert_eq!(
            parse_relative_time_at_date(now, "1 fortnight").unwrap(),
            now.checked_add_days(Days::new(14)).unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "3 fortnights").unwrap(),
            now.checked_add_days(Days::new(3 * 14)).unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "fortnight").unwrap(),
            now.checked_add_days(Days::new(14)).unwrap()
        );

        assert_eq!(
            parse_relative_time_at_date(now, "1 week").unwrap(),
            now.checked_add_days(Days::new(7)).unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "1 week 3 days").unwrap(),
            now.checked_add_days(Days::new(7 + 3)).unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "1 week 3 days ago").unwrap(),
            now.checked_sub_days(Days::new(7 + 3)).unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "-2 weeks").unwrap(),
            now.checked_sub_days(Days::new(14)).unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "2 weeks ago").unwrap(),
            now.checked_sub_days(Days::new(14)).unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "week").unwrap(),
            now.checked_add_days(Days::new(7)).unwrap()
        );

        assert_eq!(parse_duration("1 day").unwrap(), Duration::seconds(86_400));
        assert_eq!(
            parse_duration("2 days ago").unwrap(),
            Duration::seconds(-172_800)
        );
        assert_eq!(
            parse_duration("-2 days").unwrap(),
            Duration::seconds(-172_800)
        );
        assert_eq!(parse_duration("day").unwrap(), Duration::seconds(86_400));

        assert_eq!(parse_duration("1 hour").unwrap(), Duration::seconds(3_600));
        assert_eq!(parse_duration("1 h").unwrap(), Duration::seconds(3_600));
        assert_eq!(
            parse_duration("1 hour ago").unwrap(),
            Duration::seconds(-3_600)
        );
        assert_eq!(
            parse_duration("-2 hours").unwrap(),
            Duration::seconds(-7_200)
        );
        assert_eq!(parse_duration("hour").unwrap(), Duration::seconds(3_600));

        assert_eq!(parse_duration("1 minute").unwrap(), Duration::seconds(60));
        assert_eq!(parse_duration("1 min").unwrap(), Duration::seconds(60));
        assert_eq!(parse_duration("2 minutes").unwrap(), Duration::seconds(120));
        assert_eq!(parse_duration("2 mins").unwrap(), Duration::seconds(120));
        assert_eq!(parse_duration("2m").unwrap(), Duration::seconds(120));
        assert_eq!(parse_duration("min").unwrap(), Duration::seconds(60));

        assert_eq!(parse_duration("1 second").unwrap(), Duration::seconds(1));
        assert_eq!(parse_duration("1 s").unwrap(), Duration::seconds(1));
        assert_eq!(parse_duration("2 seconds").unwrap(), Duration::seconds(2));
        assert_eq!(parse_duration("2 secs").unwrap(), Duration::seconds(2));
        assert_eq!(parse_duration("2 sec").unwrap(), Duration::seconds(2));
        assert_eq!(parse_duration("sec").unwrap(), Duration::seconds(1));

        assert_eq!(parse_duration("now").unwrap(), Duration::seconds(0));
        assert_eq!(parse_duration("today").unwrap(), Duration::seconds(0));

        assert_eq!(
            parse_relative_time_at_date(now, "1 year 2 months 4 weeks 3 days and 2 seconds")
                .unwrap(),
            now.checked_add_months(Months::new(12 + 2))
                .unwrap()
                .checked_add_days(Days::new(4 * 7 + 3))
                .unwrap()
                .checked_add_signed(Duration::seconds(2))
                .unwrap()
        );
        assert_eq!(
            parse_relative_time_at_date(now, "1 year 2 months 4 weeks 3 days and 2 seconds ago")
                .unwrap(),
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
        let _ = parse_duration(invalid_input).unwrap();
    }

    #[test]
    fn test_display_should_fail() {
        let invalid_input = "Thu Jan 01 12:34:00 2015";
        let error = parse_duration(invalid_input).unwrap_err();

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

// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use chrono::{DateTime, Datelike, Days, NaiveTime, TimeZone};

use crate::{
    parse::{self, WeekdayItem},
    ParseDateTimeError,
};

pub fn parse_weekday_at_date<T: TimeZone>(
    mut datetime: DateTime<T>,
    s: &str,
) -> Result<DateTime<T>, ParseDateTimeError> {
    let WeekdayItem { weekday, ordinal } =
        parse::parse_weekday(s.trim()).map_err(|_| ParseDateTimeError::InvalidInput)?;

    datetime = datetime.with_time(NaiveTime::MIN).unwrap(); // infallible

    let mut ordinal = ordinal.unwrap_or(0);
    if datetime.weekday() != weekday && ordinal > 0 {
        ordinal -= 1;
    }

    let days_delta = (i64::from(weekday.num_days_from_monday())
        - i64::from(datetime.weekday().num_days_from_monday()))
    .rem_euclid(7)
        + ordinal * 7;

    let datetime = if days_delta < 0 {
        datetime.checked_sub_days(Days::new(-days_delta as u64))
    } else {
        datetime.checked_add_days(Days::new(days_delta as u64))
    };
    datetime.ok_or(ParseDateTimeError::InvalidInput)
}

#[cfg(test)]
mod tests {

    use super::*;
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};

    #[test]
    fn test_parse_weekday_at_date_this_weekday() {
        // Jan 1 2025 is a Wed
        let now = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));
        // Check "this <same weekday>"
        assert_eq!(parse_weekday_at_date(now, "this wednesday").unwrap(), now);
        assert_eq!(parse_weekday_at_date(now, "this wed").unwrap(), now);
        // Other days
        assert_eq!(
            parse_weekday_at_date(now, "this thursday").unwrap(),
            now.checked_add_days(Days::new(1)).unwrap()
        );
        assert_eq!(
            parse_weekday_at_date(now, "this thur").unwrap(),
            now.checked_add_days(Days::new(1)).unwrap()
        );
        assert_eq!(
            parse_weekday_at_date(now, "this thu").unwrap(),
            now.checked_add_days(Days::new(1)).unwrap()
        );
        assert_eq!(
            parse_weekday_at_date(now, "this friday").unwrap(),
            now.checked_add_days(Days::new(2)).unwrap()
        );
        assert_eq!(
            parse_weekday_at_date(now, "this fri").unwrap(),
            now.checked_add_days(Days::new(2)).unwrap()
        );
        assert_eq!(
            parse_weekday_at_date(now, "this saturday").unwrap(),
            now.checked_add_days(Days::new(3)).unwrap()
        );
        assert_eq!(
            parse_weekday_at_date(now, "this sat").unwrap(),
            now.checked_add_days(Days::new(3)).unwrap()
        );
        // "this" with a day of the week that comes before today should return the next instance of
        // that day
        assert_eq!(
            parse_weekday_at_date(now, "this sunday").unwrap(),
            now.checked_add_days(Days::new(4)).unwrap()
        );
        assert_eq!(
            parse_weekday_at_date(now, "this sun").unwrap(),
            now.checked_add_days(Days::new(4)).unwrap()
        );
        assert_eq!(
            parse_weekday_at_date(now, "this monday").unwrap(),
            now.checked_add_days(Days::new(5)).unwrap()
        );
        assert_eq!(
            parse_weekday_at_date(now, "this mon").unwrap(),
            now.checked_add_days(Days::new(5)).unwrap()
        );
        assert_eq!(
            parse_weekday_at_date(now, "this tuesday").unwrap(),
            now.checked_add_days(Days::new(6)).unwrap()
        );
        assert_eq!(
            parse_weekday_at_date(now, "this tue").unwrap(),
            now.checked_add_days(Days::new(6)).unwrap()
        );
    }

    #[test]
    fn test_parse_weekday_at_date_last_weekday() {
        // Jan 1 2025 is a Wed
        let now = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));
        // Check "last <same weekday>"
        assert_eq!(
            parse_weekday_at_date(now, "last wed").unwrap(),
            now.checked_sub_days(Days::new(7)).unwrap()
        );
        // Check "last <day after today>"
        assert_eq!(
            parse_weekday_at_date(now, "last thu").unwrap(),
            now.checked_sub_days(Days::new(6)).unwrap()
        );
        // Check "last <day before today>"
        assert_eq!(
            parse_weekday_at_date(now, "last tue").unwrap(),
            now.checked_sub_days(Days::new(1)).unwrap()
        );
    }

    #[test]
    fn test_parse_weekday_at_date_next_weekday() {
        // Jan 1 2025 is a Wed
        let now = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));
        // Check "next <same weekday>"
        assert_eq!(
            parse_weekday_at_date(now, "next wed").unwrap(),
            now.checked_add_days(Days::new(7)).unwrap()
        );
        // Check "next <day after today>"
        assert_eq!(
            parse_weekday_at_date(now, "next thu").unwrap(),
            now.checked_add_days(Days::new(1)).unwrap()
        );
        // Check "next <day before today>"
        assert_eq!(
            parse_weekday_at_date(now, "next tue").unwrap(),
            now.checked_add_days(Days::new(6)).unwrap()
        );
    }

    #[test]
    fn test_parse_weekday_at_date_number_weekday() {
        // Jan 1 2025 is a Wed
        let now = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));
        assert_eq!(
            parse_weekday_at_date(now, "1 wed").unwrap(),
            now.checked_add_days(Days::new(7)).unwrap()
        );
        assert_eq!(
            parse_weekday_at_date(now, "1 thu").unwrap(),
            now.checked_add_days(Days::new(1)).unwrap()
        );
        assert_eq!(
            parse_weekday_at_date(now, "1 tue").unwrap(),
            now.checked_add_days(Days::new(6)).unwrap()
        );
        assert_eq!(
            parse_weekday_at_date(now, "2 wed").unwrap(),
            now.checked_add_days(Days::new(14)).unwrap()
        );
        assert_eq!(
            parse_weekday_at_date(now, "2 thu").unwrap(),
            now.checked_add_days(Days::new(8)).unwrap()
        );
        assert_eq!(
            parse_weekday_at_date(now, "2 tue").unwrap(),
            now.checked_add_days(Days::new(13)).unwrap()
        );
    }

    #[test]
    fn test_parse_weekday_at_date_weekday_truncates_time() {
        // Jan 1 2025 is a Wed
        let now = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        ));
        let now_midnight = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));
        assert_eq!(
            parse_weekday_at_date(now, "this wed").unwrap(),
            now_midnight
        );
        assert_eq!(
            parse_weekday_at_date(now, "last wed").unwrap(),
            now_midnight.checked_sub_days(Days::new(7)).unwrap()
        );
        assert_eq!(
            parse_weekday_at_date(now, "next wed").unwrap(),
            now_midnight.checked_add_days(Days::new(7)).unwrap()
        );
    }

    #[test]
    fn test_parse_weekday_at_date_invalid_weekday() {
        // Jan 1 2025 is a Wed
        let now = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));
        assert_eq!(
            parse_weekday_at_date(now, "this fooday"),
            Err(ParseDateTimeError::InvalidInput)
        );
    }
}

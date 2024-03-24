use chrono::{DateTime, FixedOffset, Local, NaiveTime, TimeZone};
use regex::Regex;

mod time_only_formats {
    pub const HH_MM: &str = "%R";
    pub const HH_MM_SS: &str = "%T";
    pub const TWELVEHOUR: &str = "%r";
}

pub(crate) fn parse_time_only(date: DateTime<Local>, s: &str) -> Option<DateTime<FixedOffset>> {
    let re =
        Regex::new(r"^(?<time>.*?)(?:(?<sign>\+|-)(?<h>[0-9]{1,2}):?(?<m>[0-9]{0,2}))?$").unwrap();
    let captures = re.captures(s)?;

    let parsed_offset = match captures.name("h") {
        Some(hours) if !(hours.as_str().is_empty()) => {
            let mut offset_in_sec = hours.as_str().parse::<i32>().unwrap() * 3600;
            match captures.name("m") {
                Some(minutes) if !(minutes.as_str().is_empty()) => {
                    offset_in_sec += minutes.as_str().parse::<i32>().unwrap() * 60;
                }
                _ => (),
            };
            offset_in_sec *= if &captures["sign"] == "-" { -1 } else { 1 };
            FixedOffset::east_opt(offset_in_sec)
        }
        _ => None,
    };

    for fmt in [
        time_only_formats::HH_MM,
        time_only_formats::HH_MM_SS,
        time_only_formats::TWELVEHOUR,
    ] {
        if let Ok(parsed) = NaiveTime::parse_from_str(captures["time"].trim(), fmt) {
            let parsed_dt = date.date_naive().and_time(parsed);
            let offset = match parsed_offset {
                Some(offset) => offset,
                None => *date.offset(),
            };
            return offset.from_local_datetime(&parsed_dt).single();
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use crate::parse_time_only_str::parse_time_only;
    use chrono::{DateTime, Local, TimeZone};
    use std::env;

    fn get_test_date() -> DateTime<Local> {
        Local.with_ymd_and_hms(2024, 3, 3, 0, 0, 0).unwrap()
    }

    #[test]
    fn test_time_only() {
        env::set_var("TZ", "UTC");
        let parsed_time = parse_time_only(get_test_date(), "21:04")
            .unwrap()
            .timestamp();
        assert_eq!(parsed_time, 1709499840)
    }

    #[test]
    fn test_time_with_offset() {
        env::set_var("TZ", "UTC");
        let parsed_time = parse_time_only(get_test_date(), "21:04 +0530")
            .unwrap()
            .timestamp();
        assert_eq!(parsed_time, 1709480040);
    }

    #[test]
    fn test_time_with_hour_only_offset() {
        env::set_var("TZ", "UTC");
        let parsed_time = parse_time_only(get_test_date(), "22:04 +01")
            .unwrap()
            .timestamp();
        assert_eq!(parsed_time, 1709499840);
    }

    #[test]
    fn test_time_with_hour_only_neg_offset() {
        env::set_var("TZ", "UTC");
        let parsed_time = parse_time_only(get_test_date(), "17:04 -04")
            .unwrap()
            .timestamp();
        assert_eq!(parsed_time, 1709499840);
    }

    #[test]
    fn test_time_with_seconds() {
        env::set_var("TZ", "UTC");
        let parsed_time = parse_time_only(get_test_date(), "21:04:30")
            .unwrap()
            .timestamp();
        assert_eq!(parsed_time, 1709499870)
    }

    #[test]
    fn test_time_with_seconds_with_offset() {
        env::set_var("TZ", "UTC");
        let parsed_time = parse_time_only(get_test_date(), "21:04:30 +0530")
            .unwrap()
            .timestamp();
        assert_eq!(parsed_time, 1709480070)
    }

    #[test]
    fn test_twelve_hour_time() {
        env::set_var("TZ", "UTC");
        let parsed_time = parse_time_only(get_test_date(), "9:04:00 PM")
            .unwrap()
            .timestamp();
        assert_eq!(parsed_time, 1709499840)
    }
}

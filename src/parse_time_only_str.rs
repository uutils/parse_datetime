use chrono::{DateTime, FixedOffset, Local, NaiveTime, TimeZone};
use regex::Regex;

mod time_only_formats {
    pub const HH_MM: &str = "%R";
    pub const HH_MM_SS: &str = "%T";
    pub const TWELVE_HOUR: &str = "%r";
}

/// Convert a military time zone string to a time zone offset.
///
/// Military time zones are the letters A through Z except J. They are
/// described in RFC 5322.
fn to_offset(tz: &str) -> Option<FixedOffset> {
    let hour = match tz {
        "A" => 1,
        "B" => 2,
        "C" => 3,
        "D" => 4,
        "E" => 5,
        "F" => 6,
        "G" => 7,
        "H" => 8,
        "I" => 9,
        "K" => 10,
        "L" => 11,
        "M" => 12,
        "N" => -1,
        "O" => -2,
        "P" => -3,
        "Q" => -4,
        "R" => -5,
        "S" => -6,
        "T" => -7,
        "U" => -8,
        "V" => -9,
        "W" => -10,
        "X" => -11,
        "Y" => -12,
        "Z" => 0,
        _ => return None,
    };
    let offset_in_sec = hour * 3600;
    FixedOffset::east_opt(offset_in_sec)
}

/// Parse a time string without an offset and apply an offset to it.
///
/// Multiple formats are attempted when parsing the string.
fn parse_time_with_offset_multi(
    date: DateTime<Local>,
    offset: FixedOffset,
    s: &str,
) -> Option<DateTime<FixedOffset>> {
    for fmt in [
        time_only_formats::HH_MM,
        time_only_formats::HH_MM_SS,
        time_only_formats::TWELVE_HOUR,
    ] {
        let parsed = match NaiveTime::parse_from_str(s, fmt) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let parsed_dt = date.date_naive().and_time(parsed);
        if let Some(dt) = offset.from_local_datetime(&parsed_dt).single() {
            return Some(dt);
        }
    }
    None
}

pub(crate) fn parse_time_only(date: DateTime<Local>, s: &str) -> Option<DateTime<FixedOffset>> {
    let re =
        Regex::new(r"^(?<time>.*?)(?:(?<sign>\+|-)(?<h>[0-9]{1,2}):?(?<m>[0-9]{0,2}))?$").unwrap();
    let captures = re.captures(s)?;

    // Parse the sign, hour, and minute to get a `FixedOffset`, if possible.
    let parsed_offset = match captures.name("h") {
        Some(hours) if !(hours.as_str().is_empty()) => {
            let mut offset_in_sec = hours.as_str().parse::<i32>().unwrap() * 3600;
            match captures.name("m") {
                Some(minutes) if !(minutes.as_str().is_empty()) => {
                    offset_in_sec += minutes.as_str().parse::<i32>().unwrap() * 60;
                }
                _ => (),
            }
            offset_in_sec *= if &captures["sign"] == "-" { -1 } else { 1 };
            FixedOffset::east_opt(offset_in_sec)
        }
        _ => None,
    };

    // Parse the time and apply the parsed offset.
    let s = captures["time"].trim();
    let offset = match parsed_offset {
        Some(offset) => offset,
        None => *date.offset(),
    };
    if let Some(result) = parse_time_with_offset_multi(date, offset, s) {
        return Some(result);
    }

    // Military time zones are specified in RFC 5322, Section 4.3
    // "Obsolete Date and Time".
    // <https://datatracker.ietf.org/doc/html/rfc5322>
    //
    // We let the parsing above handle "5:00 AM" so at this point we
    // should be guaranteed that we don't have an AM/PM suffix. That
    // way, we can safely parse "5:00M" here without interference.
    let re = Regex::new(r"(?<time>.*?)(?<tz>[A-IKLMN-YZ])").unwrap();
    let captures = re.captures(s)?;
    if let Some(tz) = captures.name("tz") {
        let s = captures["time"].trim();
        let offset = match to_offset(tz.as_str()) {
            Some(offset) => offset,
            None => *date.offset(),
        };
        if let Some(result) = parse_time_with_offset_multi(date, offset, s) {
            return Some(result);
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
        assert_eq!(parsed_time, 1709499840);
    }

    #[test]
    fn test_military_time_zones() {
        env::set_var("TZ", "UTC");
        let date = get_test_date();
        let actual = parse_time_only(date, "05:00C").unwrap().timestamp();
        // Computed via `date -u -d "2024-03-03 05:00:00C" +%s`, using a
        // version of GNU date after v8.32 (earlier versions had a bug).
        let expected = 1709431200;
        assert_eq!(actual, expected);
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
        assert_eq!(parsed_time, 1709499870);
    }

    #[test]
    fn test_time_with_seconds_with_offset() {
        env::set_var("TZ", "UTC");
        let parsed_time = parse_time_only(get_test_date(), "21:04:30 +0530")
            .unwrap()
            .timestamp();
        assert_eq!(parsed_time, 1709480070);
    }

    #[test]
    fn test_twelve_hour_time() {
        env::set_var("TZ", "UTC");
        let parsed_time = parse_time_only(get_test_date(), "9:04:00 PM")
            .unwrap()
            .timestamp();
        assert_eq!(parsed_time, 1709499840);
    }
}

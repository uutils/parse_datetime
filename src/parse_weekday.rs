// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use chrono::Weekday;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::value;
use nom::{self, IResult};

// Helper macro to simplify tag matching
macro_rules! tag_match {
    ($day:expr, $($pattern:expr),+) => {
        value($day, alt(($(tag($pattern)),+)))
    };
}

pub(crate) fn parse_weekday(s: &str) -> Option<Weekday> {
    let s = s.trim().to_lowercase();
    let s = s.as_str();

    let parse_result: IResult<&str, Weekday> = nom::combinator::all_consuming(alt((
        tag_match!(Weekday::Mon, "monday", "mon"),
        tag_match!(Weekday::Tue, "tuesday", "tues", "tue"),
        tag_match!(Weekday::Wed, "wednesday", "wednes", "wed"),
        tag_match!(Weekday::Thu, "thursday", "thurs", "thur", "thu"),
        tag_match!(Weekday::Fri, "friday", "fri"),
        tag_match!(Weekday::Sat, "saturday", "sat"),
        tag_match!(Weekday::Sun, "sunday", "sun"),
    )))(s);

    match parse_result {
        Ok((_, weekday)) => Some(weekday),
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {

    use chrono::Weekday::*;

    use crate::parse_weekday::parse_weekday;

    #[test]
    fn test_valid_weekdays() {
        let days = [
            ("mon", Mon),
            ("monday", Mon),
            ("tue", Tue),
            ("tues", Tue),
            ("tuesday", Tue),
            ("wed", Wed),
            ("wednes", Wed),
            ("wednesday", Wed),
            ("thu", Thu),
            ("thursday", Thu),
            ("fri", Fri),
            ("friday", Fri),
            ("sat", Sat),
            ("saturday", Sat),
            ("sun", Sun),
            ("sunday", Sun),
        ];

        for (name, weekday) in days {
            assert_eq!(parse_weekday(name), Some(weekday));
            assert_eq!(parse_weekday(&format!(" {}", name)), Some(weekday));
            assert_eq!(parse_weekday(&format!(" {} ", name)), Some(weekday));
            assert_eq!(parse_weekday(&format!("{} ", name)), Some(weekday));

            let (left, right) = name.split_at(1);
            let (test_str1, test_str2) = (
                format!("{}{}", left.to_uppercase(), right.to_lowercase()),
                format!("{}{}", left.to_lowercase(), right.to_uppercase()),
            );

            assert_eq!(parse_weekday(&test_str1), Some(weekday));
            assert_eq!(parse_weekday(&test_str2), Some(weekday));
        }
    }

    #[test]
    fn test_invalid_weekdays() {
        let days = [
            "mond",
            "tuesda",
            "we",
            "th",
            "fr",
            "sa",
            "su",
            "garbageday",
            "tomorrow",
            "yesterday",
        ];
        for day in days {
            assert!(parse_weekday(day).is_none());
        }
    }
}

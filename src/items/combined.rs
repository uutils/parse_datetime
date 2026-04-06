// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Parse an ISO 8601 date and time item
//!
//! The GNU docs state:
//!
//! > The ISO 8601 date and time of day extended format consists of an ISO 8601
//! > date, a ‘T’ character separator, and an ISO 8601 time of day. This format
//! > is also recognized if the ‘T’ is replaced by a space.
//! >
//! > In this format, the time of day should use 24-hour notation. Fractional
//! > seconds are allowed, with either comma or period preceding the fraction.
//! > ISO 8601 fractional minutes and hours are not supported. Typically, hosts
//! > support nanosecond timestamp resolution; excess precision is silently
//! > discarded.
use winnow::{
    combinator::{alt, trace},
    ModalResult, Parser,
};

use crate::items::space;

use super::{date, primitive::s, time};

#[derive(PartialEq, Debug, Clone, Default)]
pub(crate) struct DateTime {
    pub(crate) date: date::Date,
    pub(crate) time: time::Time,
}

fn remaining_starts_with_meridiem(input: &str) -> bool {
    let trimmed = input.trim_start();
    trimmed.starts_with("am")
        || trimmed.starts_with("pm")
        || trimmed.starts_with("a.m.")
        || trimmed.starts_with("p.m.")
}

pub(crate) fn parse(input: &mut &str) -> ModalResult<DateTime> {
    let date = trace("iso_date", alt((date::iso1, date::iso2))).parse_next(input)?;
    // Note: the `T` is lowercased by the main parse function
    alt((s('t').void(), (' ', space).void())).parse_next(input)?;

    let mut iso_input = *input;
    if let Ok(parsed_time) = trace("iso_time", time::iso).parse_next(&mut iso_input) {
        if !remaining_starts_with_meridiem(iso_input) {
            *input = iso_input;
            return Ok(DateTime {
                date,
                time: parsed_time,
            });
        }
    }

    let time = trace("iso_time", time::parse).parse_next(input)?;
    Ok(DateTime { date, time })
}

#[cfg(test)]
mod tests {
    use super::{parse, DateTime};
    use crate::items::{date::Date, time::Time};

    #[test]
    fn some_date() {
        let reference = Some(DateTime {
            date: Date {
                day: 10,
                month: 10,
                year: Some(2022),
            },
            time: Time {
                hour: 10,
                minute: 10,
                second: 55,
                nanosecond: 0,
                offset: None,
            },
        });

        for mut s in [
            "2022-10-10t10:10:55",
            "2022-10-10 10:10:55",
            "2022-10-10    t   10:10:55",
            "2022-10-10       10:10:55",
            "2022-10-10  (A comment!) t   10:10:55",
            "2022-10-10  (A comment!)   10:10:55",
        ] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).ok(), reference, "Failed string: {old_s}")
        }
    }

    #[test]
    fn date_and_time_ampm() {
        let reference = Some(DateTime {
            date: Date {
                day: 15,
                month: 6,
                year: Some(2024),
            },
            time: Time {
                hour: 15,
                minute: 0,
                second: 0,
                nanosecond: 0,
                offset: None,
            },
        });

        for mut s in [
            "2024-06-15 3:00 pm",
            "2024-06-15 3:00pm",
            "2024-06-15 3:00 p.m.",
        ] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).ok(), reference, "Failed string: {old_s}");
        }
    }
}

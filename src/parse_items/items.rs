// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use nom::branch::alt;
use nom::character::complete::space0;
use nom::combinator::all_consuming;
use nom::sequence::preceded;
use nom::Parser;

use crate::parse_items::items::calendar_day::{
    calendar_day, month_day, RawCalendarDay, RawMonthDay,
};
use crate::parse_items::items::seconds_epoch::{seconds_epoch, SecondsEpoch};
use crate::parse_items::items::time_of_day::{time_of_day, RawTimeOfDay, TimeZoneCorrection};
use crate::parse_items::items::tz_identifier::{tz_identifier, TzIdentifier};
use crate::parse_items::{PError, PResult};

pub mod calendar_day;
pub mod seconds_epoch;
pub mod time_of_day;
pub mod tz_identifier;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Item {
    SecondsEpoch(SecondsEpoch),
    TimeZoneRule(TzIdentifier),
    CalendarDay(RawCalendarDay), // replace NaiveDay?
    MonthDay(RawMonthDay),       // replace ?
    TimeOfDay(RawTimeOfDay),     // replace NaiveTime?
    TimeZoneCorrection(TimeZoneCorrection),
}

fn single_parser(input: &str) -> PResult<Vec<Item>> {
    preceded(
        space0,
        alt((
            tz_identifier,
            seconds_epoch,
            calendar_day,
            month_day,
            time_of_day,
        )),
    )
    .parse(input)
}

pub fn parse(mut input: &str) -> Result<Vec<Item>, PError> {
    let mut all_items = vec![];
    loop {
        let (tail, items) = single_parser(input).map_err(|err| match err {
            nom::Err::Error(e) => e,
            nom::Err::Failure(e) => e,
            nom::Err::Incomplete(_) => panic!("Should only use complete parsers"),
        })?;

        for item in items {
            all_items.push(item)
        }
        input = tail;

        if all_consuming(space0::<&str, PError>).parse(input).is_ok() {
            break Ok(all_items);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_items::items::calendar_day::{RawCalendarDay, RawMonthDay};
    use crate::parse_items::items::seconds_epoch::SecondsEpoch;
    use crate::parse_items::items::time_of_day::{RawTimeOfDay, TimeZoneCorrection};
    use crate::parse_items::items::tz_identifier::TzIdentifier;
    use crate::parse_items::items::{parse, Item};

    #[test]
    fn some_items() {
        let result =
            parse("TZ=\"Europe/Amsterdam\" @123.456 14nov2022 11/14 12:34:56.789123456+01:30 11pm");
        assert_eq!(
            result,
            Ok(vec![
                Item::TimeZoneRule(TzIdentifier("Europe/Amsterdam".to_owned())),
                Item::SecondsEpoch(SecondsEpoch {
                    seconds: 123,
                    nanoseconds: 456000000
                }),
                Item::CalendarDay(RawCalendarDay {
                    year: 2022,
                    month: 11,
                    day: 14,
                }),
                Item::MonthDay(RawMonthDay { month: 11, day: 14 }),
                Item::TimeOfDay(RawTimeOfDay {
                    hours: 12,
                    minutes: 34,
                    seconds: 56,
                    nanoseconds: 789123456,
                }),
                Item::TimeZoneCorrection(TimeZoneCorrection {
                    hours: 1,
                    minutes: 30,
                }),
                Item::TimeOfDay(RawTimeOfDay {
                    hours: 23,
                    minutes: 0,
                    seconds: 0,
                    nanoseconds: 0,
                }),
            ])
        )
    }
}

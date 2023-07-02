// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use nom::branch::alt;
use nom::bytes::complete::{tag, tag_no_case, take};
use nom::character::complete;
use nom::character::complete::digit1;
use nom::combinator::{map, map_parser, not, opt, peek, value, verify};
use nom::sequence::preceded;
use nom::Parser;

use crate::parse_items::items::Item;
use crate::parse_items::nano_seconds::nano_seconds;
use crate::parse_items::singleton_list;
use crate::parse_items::{fixed_number, PResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawTimeOfDay {
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
    pub nanoseconds: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeZoneCorrection {
    pub hours: i8,
    pub minutes: u8,
}

pub fn time_of_day(input: &str) -> PResult<Vec<Item>> {
    alt((
        time_of_day_24,
        singleton_list(map(time_of_day_12, Item::TimeOfDay)),
    ))
    .parse(input)
}

// 08:02:00.0000pm
// 08:02:00pm
// 08:02pm
// 8pm
fn time_of_day_12(input: &str) -> PResult<RawTimeOfDay> {
    let (tail, hours) = complete::u8.parse(input)?;
    let (tail, minutes) = opt(preceded(tag(":"), fixed_number::u8(2))).parse(tail)?;
    let (tail, seconds) = if minutes.is_some() {
        opt(preceded(tag(":"), fixed_number::u8(2))).parse(tail)?
    } else {
        (tail, None)
    };
    let (tail, nanoseconds) = if seconds.is_some() {
        opt(nano_seconds).parse(tail)?
    } else {
        (tail, None)
    };
    let (tail, meridiem) = alt((
        value(0, tag_no_case("am")),
        value(0, tag_no_case("a.m.")),
        value(12, tag_no_case("pm")),
        value(12, tag_no_case("p.m.")),
    ))
    .parse(tail)?;
    // 12 < 1...
    let hours = (hours % 12) + meridiem;

    Ok((
        tail,
        RawTimeOfDay {
            hours,
            minutes: minutes.unwrap_or_default(),
            seconds: seconds.unwrap_or_default(),
            nanoseconds: nanoseconds.unwrap_or_default(),
        },
    ))
}

fn time_zone_correction(input: &str) -> PResult<TimeZoneCorrection> {
    let (tail, sign) = alt((value(1, tag("+")), value(-1, tag("-")))).parse(input)?;
    let (tail, hours) = verify(fixed_number::u8(2), |&hour| hour <= 24).parse(tail)?;
    let (tail, minutes) = opt(alt((
        fixed_number::u8(2),
        preceded(tag(":"), fixed_number::u8(2)),
    )))
    .parse(tail)?;
    peek(not(map_parser(take(1u8), digit1))).parse(tail)?;
    Ok((
        tail,
        TimeZoneCorrection {
            hours: sign * hours as i8,
            minutes: minutes.unwrap_or_default(),
        },
    ))
}

// 20:02:00.0000-0500
// 20:02:00.0000
// 20:02:00-0500
// 20:02:00
// 20:02-0500
// 20:02
fn time_of_day_24(input: &str) -> PResult<Vec<Item>> {
    let (tail, hours) = fixed_number::u8(2).parse(input)?;
    let (tail, minutes) = preceded(tag(":"), fixed_number::u8(2)).parse(tail)?;
    let (tail, seconds) = opt(preceded(tag(":"), fixed_number::u8(2))).parse(tail)?;
    let (tail, nanoseconds) = if seconds.is_some() {
        opt(nano_seconds).parse(tail)?
    } else {
        (tail, None)
    };

    let time_of_day = RawTimeOfDay {
        hours,
        minutes,
        seconds: seconds.unwrap_or_default(),
        nanoseconds: nanoseconds.unwrap_or_default(),
    };

    if let (tail, Some(tz_correction)) = opt(time_zone_correction).parse(tail)? {
        Ok((
            tail,
            vec![
                Item::TimeOfDay(time_of_day),
                Item::TimeZoneCorrection(tz_correction),
            ],
        ))
    } else {
        Ok((tail, vec![Item::TimeOfDay(time_of_day)]))
    }
}

#[cfg(test)]
mod tests {
    use nom::Parser;

    use crate::parse_items::tests::ptest;

    use super::*;

    macro_rules! tzc {
        ($name:ident : $input:literal => $hours:literal:$minutes:literal + $tail:literal) => {
            ptest! { $name : time_zone_correction($input) => TimeZoneCorrection { hours: $hours, minutes: $minutes }, $tail }
        };
        ($name:ident : $input:literal => X) => {
            ptest! { $name : time_zone_correction($input) => X }
        };
    }

    tzc! { positive : "+12:34a" => 12:34 + "a" }
    tzc! { negative : "-12:34a" => -12:34 + "a" }
    tzc! { more_numbers : "+12:345" => X }
    tzc! { without_colon : "+1234a" => 12:34 + "a" }
    tzc! { without_minutes : "+12a" => 12:00 + "a" }
    // tzc! { without_minutes_with_colon : "+12:a" => X }
    // tzc! { single_digit_hour : "+1:23a" => X }
    // tzc! { single_digit_minute : "+12:3a" => X }
    // tzc! { negative_minutes : "+12:-34a" => X }

    macro_rules! t12 {
        ($name:ident : $input:literal => $hours:literal:$minutes:literal:$seconds:literal:$nanoseconds:literal + $tail:literal) => {
            ptest! { $name : time_of_day_12($input) => RawTimeOfDay { hours: $hours, minutes: $minutes, seconds: $seconds, nanoseconds: $nanoseconds }, $tail }
        };
        ($name:ident : $input:literal => X) => {
            ptest! { $name : time_of_day_12($input) => X }
        };
    }

    t12! { twentythree  : "11pm"            => 23:00:00:0000      + "" }
    t12! { midnight     : "12am"            => 00:00:00:0000      + "" }
    t12! { one          : "1am"             => 01:00:00:0000      + "" }
    t12! { morning      : "8am"             => 08:00:00:0000      + "" }
    t12! { real_eleven  : "11am"            => 11:00:00:0000      + "" }
    t12! { noon         : "12pm"            => 12:00:00:0000      + "" }
    t12! { afternoon    : "8pm"             => 20:00:00:0000      + "" }
    t12! { minutes      : "8:10am"          => 08:10:00:0000      + "" }
    t12! { seconds      : "8:10:20am"       => 08:10:20:0000      + "" }
    t12! { nanoseconds  : "8:10:20.12345am" => 08:10:20:123450000 + "" }

    t12! { nanoseconds_wo_seconds : "8:10.12345am" => X }
}

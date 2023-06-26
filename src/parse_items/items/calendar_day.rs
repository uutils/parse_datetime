// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use nom::branch::alt;
use nom::bytes::complete::{tag, tag_no_case};
use nom::character::complete;
use nom::character::complete::{digit1, space0, space1};
use nom::combinator::{consumed, map, not, opt, peek, value};
use nom::sequence::{delimited, preceded, terminated, tuple};
use nom::Parser;

use crate::parse_items::items::Item;
use crate::parse_items::singleton_list;
use crate::parse_items::{fixed_number, PResult};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RawCalendarDay {
    pub day: u8,
    pub month: u8,
    pub year: u16,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RawMonthDay {
    pub day: u8,
    pub month: u8,
}

pub fn calendar_day(input: &str) -> PResult<Vec<Item>> {
    singleton_list(map(raw_calendar_day, |calendar_day: RawCalendarDay| {
        Item::CalendarDay(calendar_day)
    }))
    .parse(input)
}

fn raw_calendar_day(input: &str) -> PResult<RawCalendarDay> {
    alt((iso_gnu, us_format, letter, letter_us)).parse(input)
}

pub fn month_day(input: &str) -> PResult<Vec<Item>> {
    singleton_list(map(raw_month_day, Item::MonthDay)).parse(input)
}

fn raw_month_day(input: &str) -> PResult<RawMonthDay> {
    let (tail, (month, day)) = alt((
        tuple((complete::u8, preceded(space0_slash, complete::u8))),
        tuple((month, preceded(space0, complete::u8))),
        map(
            tuple((complete::u8, preceded(space0, month))),
            |(day, month)| (month, day),
        ),
    ))
    .parse(input)?;

    Ok((tail, RawMonthDay { month, day }))
}

fn space0_dash(input: &str) -> PResult<&str> {
    let (tail, (consumed, _)) = consumed(delimited(space0, tag("-"), space0)).parse(input)?;
    Ok((tail, consumed))
}

fn year_wo_century(input: &str) -> PResult<u16> {
    let (tail, year) = fixed_number::u8(2).parse(input)?;
    let year = year as u16 + if year <= 68 { 2000 } else { 1900 };
    Ok((tail, year))
}

fn iso_gnu(input: &str) -> PResult<RawCalendarDay> {
    let (input, (year, month, day)) = tuple((
        alt((
            terminated(year_wo_century, space0_dash),
            terminated(complete::u16, space0_dash),
        )),
        terminated(complete::u8, space0_dash),
        complete::u8,
    ))
    .parse(input)?;

    Ok((input, RawCalendarDay { day, month, year }))
}

fn space0_slash(input: &str) -> PResult<&str> {
    let (tail, (consumed, _)) = consumed(delimited(space0, tag("/"), space0)).parse(input)?;
    Ok((tail, consumed))
}

fn year(input: &str) -> PResult<u16> {
    alt((
        terminated(year_wo_century, peek(not(digit1))),
        complete::u16,
    ))
    .parse(input)
}

fn us_format(input: &str) -> PResult<RawCalendarDay> {
    let (input, (month, day, year)) = tuple((
        terminated(complete::u8, space0_slash),
        terminated(complete::u8, space0_slash),
        year,
    ))
    .parse(input)?;

    Ok((input, RawCalendarDay { day, month, year }))
}

fn month(input: &str) -> PResult<u8> {
    alt((
        value(1, alt((tag_no_case("january"), tag_no_case("jan")))),
        value(2, alt((tag_no_case("february"), tag_no_case("feb")))),
        value(3, alt((tag_no_case("march"), tag_no_case("mar")))),
        value(4, alt((tag_no_case("april"), tag_no_case("apr")))),
        value(5, tag_no_case("may")),
        value(6, alt((tag_no_case("june"), tag_no_case("jun")))),
        value(7, alt((tag_no_case("july"), tag_no_case("jul")))),
        value(8, alt((tag_no_case("august"), tag_no_case("aug")))),
        value(
            9,
            alt((
                tag_no_case("september"),
                tag_no_case("sept"),
                tag_no_case("sep"),
            )),
        ),
        value(10, alt((tag_no_case("october"), tag_no_case("oct")))),
        value(11, alt((tag_no_case("november"), tag_no_case("nov")))),
        value(12, alt((tag_no_case("december"), tag_no_case("dec")))),
    ))
    .parse(input)
}

fn space0_opt_dash(input: &str) -> PResult<&str> {
    let (tail, (consumed, _)) = consumed(tuple((space0, opt(tag("-")), space0))).parse(input)?;
    Ok((tail, consumed))
}

fn letter(input: &str) -> PResult<RawCalendarDay> {
    let (tail, (day, month, year)) = tuple((
        terminated(complete::u8, space0_opt_dash),
        terminated(month, space0_opt_dash),
        year,
    ))
    .parse(input)?;

    Ok((tail, RawCalendarDay { day, month, year }))
}

fn letter_us(input: &str) -> PResult<RawCalendarDay> {
    let (tail, (month, day, year)) = tuple((
        terminated(month, space0_opt_dash),
        terminated(
            complete::u8,
            alt((delimited(space0, tag(","), space1), space0_opt_dash)),
        ),
        year,
    ))
    .parse(input)?;

    Ok((tail, RawCalendarDay { day, month, year }))
}

#[cfg(test)]
mod tests {
    use nom::Parser;

    use crate::parse_items::tests::ptest;

    use super::*;

    macro_rules! cd {
        ($name:ident : $input:literal => $year:literal-$month:literal-$day:literal + $tail:literal) => {
            ptest! { $name : raw_calendar_day($input) => RawCalendarDay { year: $year, month: $month, day: $day }, $tail }
        };
        ($name:ident : $input:literal => X) => {
            ptest! { $name : raw_calendar_day($input) => X }
        };
    }

    cd! { iso_like      : "23-45-67abc"   => 2023-45-67 + "abc" }
    cd! { iso_like_2000 : "68-45-67abc"   => 2068-45-67 + "abc" }
    cd! { iso_like_1900 : "69-45-67abc"   => 1969-45-67 + "abc" }
    cd! { us            : "34/12/5678abc" => 5678-34-12 + "abc" }
    cd! { us_short_year : "34/12/56abc"   => 2056-34-12 + "abc" }

    cd! { gnu_iso                : "2022-11-14"        => 2022-11-14    + "" }
    cd! { gnu_iso_zero_prefix    : "022-011-014"       => 0022-11-14    + "" }
    cd! { gnu_iso_wo_century     : "22-11-14"          => 2022-11-14    + "" }
    cd! { gnu_us                 : "11/14/2022"        => 2022-11-14    + "" }
    cd! { gnu_us_wo_century      : "11/14/22"          => 2022-11-14    + "" }
    cd! { gnu_us_zeroes          : "011/014/022"       => 0022-11-14    + "" }
    cd! { gnu_us_spaces          : "11 / 14 / 2022"    => 2022-11-14    + "" }
    cd! { gnu_letter             : "14 November 2022"  => 2022-11-14    + "" }
    cd! { gnu_letter_abbr        : "14 Nov 2022"       => 2022-11-14    + "" }
    cd! { gnu_letter_us          : "November 14, 2022" => 2022-11-14    + "" }
    cd! { gnu_letter_us_wo_comma : "November 14 2022"  => 2022-11-14    + "" }
    cd! { gnu_lit_month_1        : "14-nov-2022"       => 2022-11-14    + "" }
    cd! { gnu_lit_month_spaces   : "14 - nov - 2022"   => 2022-11-14    + "" }
    cd! { gnu_lit_month_2        : "14nov2022"         => 2022-11-14    + "" }

    cd! { written_ordinals_1 : "first nov 2022"  => X }
    cd! { written_ordinals_2 : "eleven nov 2022" => X }
    cd! { written_ordinals_3 : "nov eleven 2022" => X }
    cd! { written_ordinals_4 : "2022-eleven-14"  => X }
    cd! { written_ordinals_5 : "2022-11-eleven"  => X }
    cd! { written_ordinals_6 : "22-11-eleven"    => X }

    cd! { letter_1 : "14 november 2022" => 2022-11-14 + "" }
    cd! { letter_2 : "14 nov 2022"      => 2022-11-14 + "" }
    cd! { letter_3 : "14nov2022"        => 2022-11-14 + "" }
    cd! { letter_4 : "14-nov-2022"      => 2022-11-14 + "" }
    cd! { letter_5 : "14 nov 22"        => 2022-11-14 + "" }

    cd! { us_letter_1 : "november 14 2022"  => 2022-11-14 + "" }
    cd! { us_letter_2 : "nov 14 2022"       => 2022-11-14 + "" }
    cd! { us_letter_3 : "nov14 2022"        => 2022-11-14 + "" }
    cd! { us_letter_4 : "nov-14-2022"       => 2022-11-14 + "" }
    cd! { us_letter_5 : "nov 14 22"         => 2022-11-14 + "" }
    cd! { us_letter_6 : "november 14, 2022" => 2022-11-14 + "" }
    cd! { us_letter_7 : "nov 14, 2022"      => 2022-11-14 + "" }
    cd! { us_letter_8 : "nov14, 2022"       => 2022-11-14 + "" }
    cd! { us_letter_9 : "nov 14, 2022"      => 2022-11-14 + "" }

    macro_rules! md {
        ($name:ident : $input:literal => xxxx-$month:literal-$day:literal + $tail:literal) => {
            ptest! { $name : raw_month_day($input) => RawMonthDay {  month: $month, day: $day }, $tail }
        };
        ($name:ident : $input:literal => X) => {
            ptest! { $name : raw_month_day($input) => X }
        };
    }

    md! { written_month       : "14 november" => xxxx-11-14 + "" }
    md! { written_month_short : "14 nov"      => xxxx-11-14 + "" }
    md! { written_month_us    : "nov 14"      => xxxx-11-14 + "" }
    md! { us_slash            : "11/14"       => xxxx-11-14 + "" }
}

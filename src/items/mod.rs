// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore chrono

//! From the GNU docs:
//!
//! > A date is a string, possibly empty, containing many items separated by
//! > whitespace. The whitespace may be omitted when no ambiguity arises. The
//! > empty string means the beginning of today (i.e., midnight). Order of the
//! > items is immaterial. A date string may contain many flavors of items:
//! >  - calendar date items
//! >  - time of day items
//! >  - time zone items
//! >  - combined date and time of day items
//! >  - day of the week items
//! >  - relative items
//! >  - pure numbers.
//!
//! We put all of those in separate modules:
//!  - [`date`]
//!  - [`time`]
//!  - [`time_zone`]
//!  - [`combined`]
//!  - [`weekday`]
//!  - [`relative`]
//!  - [`number]

use chrono::{NaiveDateTime, NaiveTime, Weekday};
use winnow::{combinator::alt, PResult, Parser};
mod date;

pub enum Item {
    Date(date::Date),
    TimeOfDay(NaiveTime),
    _TimeZone,
    Combined(NaiveDateTime),
    Weekday(Weekday),
    _Relative,
    _PureNumber,
}

pub fn parse(input: &mut &str) -> PResult<Item> {
    alt((date::parse.map(Item::Date),)).parse_next(input)
}

mod time {}

mod time_zone {}

mod combined {}

mod weekday {}

mod relative {}

mod number {}

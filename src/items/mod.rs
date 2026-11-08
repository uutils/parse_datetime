// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore multispace0

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
//!  - [`combined`]
//!  - [`date`]
//!  - [`epoch`]
//!  - [`offset`]
//!  - [`pure`]
//!  - [`relative`]
//!  - [`time`]
//!  - [`timezone`]
//!  - [`weekday`]
//!  - [`year`]

// date and time items
mod combined;
mod date;
mod epoch;
mod names;
mod offset;
mod pure;
mod relative;
mod time;
mod timezone;
mod weekday;
mod year;

// utility modules
mod builder;
mod ordinal;
mod parser;
mod primitive;
mod resolve;

pub(crate) mod error;

pub(crate) use parser::{parse_at_date, parse_at_date_with_trace, parse_at_local};

#[cfg(test)]
mod tests;

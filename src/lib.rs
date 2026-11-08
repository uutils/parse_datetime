// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! A Rust crate for parsing human-readable relative time strings and
//! human-readable datetime strings.
//! The function supports the following formats for time:
//!
//! * ISO formats
//! * timezone offsets, e.g., "UTC-0100"
//! * unix timestamps, e.g., "@12"
//! * relative time to now, e.g. "+1 hour"
//!
mod api;
mod extended;
mod items;
mod types;

#[cfg(test)]
mod tests;

pub use api::{parse_datetime, parse_datetime_at_date, parse_datetime_at_date_with_trace};
pub use extended::{DateParts, ExtendedDateTime, TimeParts};
pub use types::{
    MonthYearAdjustment, NonexistentLocalTime, ParseDateTimeError, ParseTrace, ParsedDate,
    ParsedDateTime, ParsedDateTimeWithTrace, ParsedItem, ParsedTime, ParsedYear,
    RelativeAdjustment, GNU_MAX_YEAR,
};

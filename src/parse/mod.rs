// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use relative_time::relative_times;
use weekday::weekday;

mod primitive;
mod relative_time;
mod weekday;

// TODO: more specific errors?
#[derive(Debug)]
pub(crate) struct ParseError;

pub(crate) use relative_time::RelativeTime;
pub(crate) use relative_time::TimeUnit;
pub(crate) use weekday::WeekdayItem;

/// Parses a string of relative times into a vector of `RelativeTime` structs.
pub(crate) fn parse_relative_times(input: &str) -> Result<Vec<RelativeTime>, ParseError> {
    relative_times(input)
        .map(|(_, times)| times)
        .map_err(|_| ParseError)
}

/// Parses a string of weekday into a `WeekdayItem` struct.
pub(crate) fn parse_weekday(input: &str) -> Result<WeekdayItem, ParseError> {
    weekday(input)
        .map(|(_, weekday_item)| weekday_item)
        .map_err(|_| ParseError)
}

/// Finds a value in a list of pairs by its key.
fn find_in_pairs<T: Clone>(pairs: &[(&str, T)], key: &str) -> Option<T> {
    pairs.iter().find_map(|(k, v)| {
        if k.eq_ignore_ascii_case(key) {
            Some(v.clone())
        } else {
            None
        }
    })
}

// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use relative_time::relative_times;

mod relative_time;

// TODO: more specific errors?
#[derive(Debug)]
pub(crate) struct ParseError;

pub(crate) use relative_time::RelativeTime;
pub(crate) use relative_time::TimeUnit;

/// Parses a string of relative times into a vector of `RelativeTime` structs.
pub(crate) fn parse_relative_times(input: &str) -> Result<Vec<RelativeTime>, ParseError> {
    relative_times(input)
        .map(|(_, times)| times)
        .map_err(|_| ParseError)
}

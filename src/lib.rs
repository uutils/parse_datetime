// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::error::Error;
use std::fmt::{self, Display};

use chrono::{DateTime, FixedOffset};

#[derive(Debug, PartialEq)]
pub enum ParseDurationError {
    InvalidInput,
}

impl Display for ParseDurationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseDurationError::InvalidInput => {
                write!(
                    f,
                    "Invalid input string: cannot be parsed as a relative time"
                )
            }
        }
    }
}

impl Error for ParseDurationError {}

fn parse_datetime(s: &str) -> Result<DateTime<FixedOffset>, ParseDurationError> {
    todo!()
}

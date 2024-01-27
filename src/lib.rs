// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore datetime

use std::error::Error;
use std::fmt::{self, Display};

use items::Item;
use winnow::Parser;

mod items;

#[derive(Debug, PartialEq)]
pub enum ParseDateTimeError {
    InvalidInput,
}

impl Display for ParseDateTimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseDateTimeError::InvalidInput => {
                write!(
                    f,
                    "Invalid input string: cannot be parsed as a relative time"
                )
            }
        }
    }
}

impl Error for ParseDateTimeError {}

pub fn parse_datetime(input: &str) -> Result<Item, ParseDateTimeError> {
    let input = input.to_ascii_lowercase();
    match items::parse.parse_next(&mut input.as_ref()) {
        Ok(x) => Ok(x),
        Err(_) => Err(ParseDateTimeError::InvalidInput),
    }
}

// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use crate::{parse, ParseDateTimeError};

pub(crate) fn parse_timestamp(s: &str) -> Result<i64, ParseDateTimeError> {
    // If the timestamp contains excess precision, it is truncated toward minus
    // infinity.
    parse::parse_timestamp(s)
        .map(|f| f.floor() as i64)
        .map_err(|_| ParseDateTimeError::InvalidInput)
}

#[cfg(test)]
mod tests {

    use crate::parse_timestamp::parse_timestamp;

    #[test]
    fn test_valid_timestamp() {
        assert_eq!(parse_timestamp("@1234"), Ok(1234));
        assert_eq!(parse_timestamp("@99999"), Ok(99999));
        assert_eq!(parse_timestamp("@-4"), Ok(-4));
        assert_eq!(parse_timestamp("@-99999"), Ok(-99999));
        assert_eq!(parse_timestamp("@+4"), Ok(4));
        assert_eq!(parse_timestamp("@0"), Ok(0));

        // gnu date accepts numbers signs and uses the last sign
        assert_eq!(parse_timestamp("@---+12"), Ok(12));
        assert_eq!(parse_timestamp("@+++-12"), Ok(-12));
        assert_eq!(parse_timestamp("@+----+12"), Ok(12));
        assert_eq!(parse_timestamp("@++++-123"), Ok(-123));

        // with excess precision
        assert_eq!(parse_timestamp("@1234.567"), Ok(1234));
        assert_eq!(parse_timestamp("@-1234,567"), Ok(-1235));
    }

    #[test]
    fn test_invalid_timestamp() {
        assert!(parse_timestamp("@").is_err());
        assert!(parse_timestamp("@+--+").is_err());
        assert!(parse_timestamp("@+1ab2").is_err());
    }
}

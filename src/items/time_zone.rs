// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Parse a time zone items
//!
//! The GNU docs state:
//!
//! > Normally, dates are interpreted using the rules of the current time zone,
//! > which in turn are specified by the TZ environment variable, or by a
//! > system default if TZ is not set. To specify a different set of default
//! > time zone rules that apply just to one date, start the date with a string
//! > of the form ‘TZ="rule"’. The two quote characters (‘"’) must be present
//! > in the date, and any quotes or backslashes within rule must be escaped by
//! > a backslash.
//!

use winnow::PResult;

pub fn parse(_input: &mut &str) -> PResult<()> {
    todo!()
}

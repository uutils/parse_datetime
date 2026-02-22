// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

mod common;

use common::{check_absolute, check_relative};
use jiff::{civil::DateTime, tz::TimeZone};

#[test]
fn helper_formats_extended_values() {
    check_absolute("10000-01-01", "10000-01-01 00:00:00+00:00");
}

#[test]
fn helper_formats_extended_rollover() {
    check_absolute("9999-12-31 +1 day", "10000-01-01 00:00:00+00:00");
}

#[test]
fn helper_formats_relative_extended_values() {
    let base = "2000-01-01 00:00:00"
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    check_relative(base, "10000-01-01 +1 day", "10000-01-02 00:00:00+00:00");
}

#![no_main]

use arbitrary::Arbitrary;
use jiff::{civil::DateTime, tz::TimeZone};
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct Input {
    /// Year for the base date (biased toward boundary years).
    base_year_selector: u8,
    /// Year to embed in a constructed large-year input string.
    input_year: u32,
    month: u8,
    day: u8,
    /// Suffix appended after the constructed date (e.g. relative items).
    suffix: String,
    /// Whether to also call parse_datetime (no base).
    try_no_base: bool,
}

fn base_year(selector: u8) -> i16 {
    match selector % 6 {
        0 => 2024,
        1 => 9998,
        2 => 9999,
        3 => 1,
        4 => 100,
        _ => (selector as i16) * 40,
    }
}

fn clamp_year(y: u32) -> u32 {
    // Focus on the interesting range: 9990..=100_000 and 0..=20_000
    match y % 4 {
        0 => 9990 + (y % 20),          // near boundary
        1 => 10000 + (y % 90_000),     // large years
        2 => y % 20_000,               // general range
        _ => 2_147_485_540 + (y % 10), // near GNU_MAX_YEAR
    }
}

fuzz_target!(|input: Input| {
    let year = clamp_year(input.input_year);
    let month = (input.month % 12) + 1;
    let day = (input.day % 28) + 1;
    let date_str = format!("{year:04}-{month:02}-{day:02} {}", input.suffix);

    // Test parse_datetime (uses current time as base).
    if input.try_no_base {
        let _ = parse_datetime::parse_datetime(&date_str);
    }

    // Test parse_datetime_at_date with a controlled base.
    let by = base_year(input.base_year_selector);
    if let Ok(base) = DateTime::new(by, 1, 1, 0, 0, 0, 0) {
        if let Ok(base) = base.to_zoned(TimeZone::UTC) {
            let _ = parse_datetime::parse_datetime_at_date(base, &date_str);
        }
    }

    // Also try a bare large year as a pure number.
    let bare = format!("{year}");
    let _ = parse_datetime::parse_datetime(&bare);
});

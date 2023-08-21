# parse_datetime

[![Crates.io](https://img.shields.io/crates/v/parse_datetime.svg)](https://crates.io/crates/parse_datetime)
[![License](http://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/uutils/parse_datetime/blob/main/LICENSE)
[![CodeCov](https://codecov.io/gh/uutils/parse_datetime/branch/main/graph/badge.svg)](https://codecov.io/gh/uutils/parse_datetime)

A Rust crate for parsing human-readable relative time strings and converting them to a `Duration`, or parsing human-readable datetime strings and converting them to a `DateTime`.

## Features

- Parses a variety of human-readable and standard time formats.
- Supports positive and negative durations.
- Allows for chaining time units (e.g., "1 hour 2 minutes" or "2 days and 2 hours").
- Calculate durations relative to a specified date.
- Relies on Chrono

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
parse_datetime = "0.4.0"
```

Then, import the crate and use the `from_str` and `from_str_at_date` functions:
```rs
use parse_datetime::{from_str, from_str_at_date};
use chrono::Duration;

let duration = from_str("+3 days");
assert_eq!(duration.unwrap(), Duration::days(3));

let today = Utc::today().naive_utc();
let yesterday = today - Duration::days(1);
assert_eq!(
    from_str_at_date(yesterday, "2 days").unwrap(),
    Duration::days(1)
);
```

For DateTime parsing, import the `parse_datetime` module:
```rs
use parse_datetime::parse_datetime::from_str;
use chrono::{Local, TimeZone};

let dt = from_str("2021-02-14 06:37:47");
assert_eq!(dt.unwrap(), Local.with_ymd_and_hms(2021, 2, 14, 6, 37, 47).unwrap());
```

### Supported Formats

The `from_str` and `from_str_at_date` functions support the following formats for relative time:

- `num` `unit` (e.g., "-1 hour", "+3 days")
- `unit` (e.g., "hour", "day")
- "now" or "today"
- "yesterday"
- "tomorrow"
- use "ago" for the past
- use "next" or "last" with `unit` (e.g., "next week", "last year")
- combined units with "and" or "," (e.g., "2 years and 1 month", "1 day, 2 hours" or "2 weeks 1 second")

`num` can be a positive or negative integer.
`unit` can be one of the following: "fortnight", "week", "day", "hour", "minute", "min", "second", "sec" and their plural forms.

## Return Values

### Duration

The `from_str` and `from_str_at_date` functions return:

- `Ok(Duration)` - If the input string can be parsed as a relative time
- `Err(ParseDurationError)` - If the input string cannot be parsed as a relative time

This function will return `Err(ParseDurationError::InvalidInput)` if the input string
cannot be parsed as a relative time.

### parse_datetime

The `from_str` function returns:

- `Ok(DateTime<FixedOffset>)` - If the input string can be parsed as a datetime
- `Err(ParseDurationError::InvalidInput)` - If the input string cannot be parsed

## Fuzzer

To run the fuzzer:
```
$ cargo fuzz run fuzz_from_str
```

## License

This project is licensed under the [MIT License](LICENSE).

## Note

At some point, this crate was called humantime_to_duration.
It has been renamed to cover more cases.

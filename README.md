# humantime_to_duration

[![Crates.io](https://img.shields.io/crates/v/humantime_to_duration.svg)](https://crates.io/crates/humantime_to_duration)
[![License](http://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/uutils/humantime_to_duration/blob/main/LICENSE)
[![CodeCov](https://codecov.io/gh/uutils/humantime_to_duration/branch/main/graph/badge.svg)](https://codecov.io/gh/uutils/humantime_to_duration)

A Rust crate for parsing human-readable relative time strings and converting them to a `Duration`.

## Features

- Parses a variety of human-readable time formats.
- Supports positive and negative durations.
- Allows for chaining time units (e.g., "1 hour 2 minutes" or "2 days and 2 hours").
- Calculate durations relative to a specified date.
- Relies on Chrono

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
humantime_to_duration = "0.3.0"
```

Then, import the crate and use the `from_str` and `from_str_at_date` functions:
```
use humantime_to_duration::{from_str, from_str_at_date};
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

### Supported Formats

The `from_str` and `from_str_at_date` functions support the following formats for relative time:

- `num` `unit` (e.g., "-1 hour", "+3 days")
- `unit` (e.g., "hour", "day")
- "now" or "today"
- "yesterday"
- "tomorrow"
- use "ago" for the past
- combined units with "and" or "," (e.g., "2 years and 1 month", "1 day, 2 hours" or "2 weeks 1 second")

`num` can be a positive or negative integer.
`unit` can be one of the following: "fortnight", "week", "day", "hour", "minute", "min", "second", "sec" and their plural forms.

## Return Values

The `from_str` and `from_str_at_date` functions return:

- `Ok(Duration)` - If the input string can be parsed as a relative time
- `Err(ParseDurationError)` - If the input string cannot be parsed as a relative time

This function will return `Err(ParseDurationError::InvalidInput)` if the input string
cannot be parsed as a relative time.

## Fuzzer

To run the fuzzer:
```
$ cargo fuzz run fuzz_from_str
```

## License

This project is licensed under the [MIT License](LICENSE).

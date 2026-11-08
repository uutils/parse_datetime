// For the full copyright and license information, view the LICENSE file.

use std::error::Error;
use std::fmt::{self, Display};

use jiff::{civil, Zoned};

use crate::{items, ExtendedDateTime};

/// Maximum year accepted by GNU `date`.
pub const GNU_MAX_YEAR: u32 = 2_147_485_547;

/// Parsed datetime output.
///
/// - [`ParsedDateTime::InRange`] contains a standard [`jiff::Zoned`] value.
/// - [`ParsedDateTime::Extended`] contains an out-of-range year representation
///   (for example, years greater than `9999`) that cannot be represented by
///   `jiff::Zoned`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedDateTime {
    InRange(Zoned),
    Extended(ExtendedDateTime),
}

/// A parsed datetime together with the semantic events and build stages used
/// to produce it.
///
/// The trace is deliberately made up of values rather than already formatted
/// diagnostic strings. Consumers such as `date --debug` can therefore render
/// the same parse without duplicating the parser's grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDateTimeWithTrace {
    pub datetime: ParsedDateTime,
    pub trace: ParseTrace,
}

/// Semantic information collected while parsing and resolving a datetime.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParseTrace {
    /// Items in parser reduction order.
    pub items: Vec<ParsedItem>,
    /// The timezone rule supplied in the input, when one was present.
    pub timezone_rule: Option<String>,
    /// The value after absolute date/time/weekday/zone fields, before relative
    /// date and time adjustments.
    pub starting: Option<Zoned>,
    /// The value after applying relative years, months, and days.
    pub after_date_adjustment: Option<Zoned>,
    /// The value after applying relative hours, minutes, seconds, and
    /// nanoseconds.
    pub after_time_adjustment: Option<Zoned>,
    /// The calendar fields before GNU normalizes an overflowing
    /// month/year adjustment, when one was applied.
    pub month_year_adjustment: Option<MonthYearAdjustment>,
    /// The cumulative relative date adjustment.
    pub date_adjustment: RelativeAdjustment,
    /// The cumulative relative time adjustment.
    pub time_adjustment: RelativeAdjustment,
    /// Whether a parsed weekday was ignored because an explicit calendar date
    /// was also supplied.
    pub weekday_ignored: bool,
    /// A local wall-clock value that falls in a timezone gap. The parser still
    /// records the compatible normalization so callers can explain the error.
    pub nonexistent_local_time: Option<NonexistentLocalTime>,
}

/// Calendar fields produced before a relative month/year adjustment is
/// normalized. For example, 2016-10-31 minus one month produces 2016-09-31,
/// which normalizes to 2016-10-01.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonthYearAdjustment {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

/// A date item as recognized by the parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDate {
    /// The textual year value. `digits == 0` means the year was inherited from
    /// the base date.
    pub year: Option<ParsedYear>,
    pub month: u8,
    pub day: u8,
    /// Compact all-digit dates are reported as a `number` part by GNU date.
    pub number: bool,
}

/// A year together with the number of digits used to write it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedYear {
    pub value: u32,
    pub digits: u8,
}

/// A time item as recognized by the parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedTime {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub nanosecond: u32,
    /// A numeric timezone correction attached to this time item, if any.
    pub offset_seconds: Option<i32>,
}

/// A cumulative relative adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RelativeAdjustment {
    pub years: i64,
    pub months: i64,
    pub days: i64,
    pub hours: i64,
    pub minutes: i64,
    pub seconds: i64,
    pub nanoseconds: i64,
}

impl RelativeAdjustment {
    /// Returns true when this adjustment changes the calendar date.
    pub fn has_date_component(self) -> bool {
        self.years != 0 || self.months != 0 || self.days != 0
    }

    /// Returns true when this adjustment changes the time of day.
    pub fn has_time_component(self) -> bool {
        self.hours != 0 || self.minutes != 0 || self.seconds != 0 || self.nanoseconds != 0
    }
}

/// A parser reduction, before the build stages resolve it against a base date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedItem {
    Date(ParsedDate),
    Time(ParsedTime),
    Weekday {
        ordinal: i32,
        day: u8,
    },
    /// The relative value is cumulative, matching GNU date's diagnostics.
    Relative(RelativeAdjustment),
    Offset {
        seconds: i32,
    },
    NamedZone {
        name: String,
        seconds: i32,
    },
    LocalZone {
        isdst: i8,
    },
    Timestamp {
        seconds: i64,
        nanoseconds: i32,
    },
    NumberYear(ParsedYear),
    NumberTime(ParsedTime),
}

/// Details for a local time that was normalized across a timezone gap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonexistentLocalTime {
    pub provided: civil::DateTime,
    pub normalized: Option<Zoned>,
}

impl ParsedDateTime {
    pub fn as_zoned(&self) -> Option<&Zoned> {
        match self {
            ParsedDateTime::InRange(z) => Some(z),
            ParsedDateTime::Extended(_) => None,
        }
    }

    pub fn into_zoned(self) -> Option<Zoned> {
        match self {
            ParsedDateTime::InRange(z) => Some(z),
            ParsedDateTime::Extended(_) => None,
        }
    }

    /// Returns the number of seconds since the Unix epoch, using floor
    /// division (rounding toward negative infinity).
    ///
    /// This matches the behavior of GNU `date +%s` for negative fractional
    /// timestamps. For example, a timestamp of −1.5 seconds returns −2
    /// (not −1 as truncation toward zero would give).
    ///
    /// Note: `jiff::Timestamp::as_second()` uses truncation toward zero,
    /// which differs from the POSIX/GNU convention for `%s`. This method
    /// corrects for that.
    pub fn unix_epoch_second(&self) -> i64 {
        match self {
            ParsedDateTime::InRange(z) => {
                let ts = z.timestamp();
                let s = ts.as_second();
                let ns = ts.subsec_nanosecond();
                // jiff stores negative fractional timestamps with a negative
                // subsec_nanosecond (e.g. -1.5s → as_second=-1, subsec=-500ms).
                // Floor semantics require subtracting 1 from the second when
                // the nanosecond component is negative.
                if ns < 0 {
                    s - 1
                } else {
                    s
                }
            }
            ParsedDateTime::Extended(ext) => ext.unix_seconds(),
        }
    }

    /// Returns the sub-second nanosecond component, always in `0..1_000_000_000`.
    ///
    /// This is the complement of [`unix_epoch_second`](Self::unix_epoch_second):
    /// the full timestamp equals `unix_epoch_second() + subsec_nanosecond() / 1e9`.
    ///
    /// For negative fractional timestamps the nanosecond is re-normalized to be
    /// non-negative (matching the GNU `timespec` convention).
    pub fn subsec_nanosecond(&self) -> i32 {
        match self {
            ParsedDateTime::InRange(z) => {
                let ns = z.timestamp().subsec_nanosecond();
                if ns < 0 {
                    1_000_000_000 + ns
                } else {
                    ns
                }
            }
            ParsedDateTime::Extended(_) => 0,
        }
    }

    /// Unwraps the `InRange` variant, panicking if this is an `Extended` value.
    ///
    /// This is a convenience for contexts where the caller is certain the result
    /// is in range (e.g., tests). Prefer [`into_zoned`](Self::into_zoned) or
    /// pattern matching in production code.
    pub fn expect_in_range(self) -> Zoned {
        self.into_zoned()
            .expect("ParsedDateTime is not representable as jiff::Zoned")
    }
}

impl fmt::Display for ParsedDateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParsedDateTime::InRange(z) => {
                let ns = z.datetime().time().subsec_nanosecond();
                if ns != 0 {
                    write!(f, "{}", z.strftime("%Y-%m-%d %H:%M:%S%.9f%:z"))
                } else {
                    write!(f, "{}", z.strftime("%Y-%m-%d %H:%M:%S%:z"))
                }
            }
            ParsedDateTime::Extended(dt) => write!(f, "{dt}"),
        }
    }
}

/// An `Extended` value never compares equal to a `Zoned`.
impl PartialEq<Zoned> for ParsedDateTime {
    fn eq(&self, other: &Zoned) -> bool {
        match self {
            ParsedDateTime::InRange(z) => z == other,
            ParsedDateTime::Extended(_) => false,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ParseDateTimeError {
    InvalidInput,
}

impl Display for ParseDateTimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseDateTimeError::InvalidInput => write!(f, "Invalid input string"),
        }
    }
}

impl Error for ParseDateTimeError {}

impl From<items::error::Error> for ParseDateTimeError {
    fn from(_: items::error::Error) -> Self {
        ParseDateTimeError::InvalidInput
    }
}

// For the full copyright and license information, view the LICENSE file.

use super::{error, relative, weekday};
use crate::{ExtendedDateTime, ParsedItem, RelativeAdjustment};
use jiff::{civil, Span, Zoned};

pub(super) fn relative_adjustment(relative: &[relative::Relative]) -> RelativeAdjustment {
    let mut result = RelativeAdjustment::default();
    for rel in relative {
        match *rel {
            relative::Relative::Years(value) => result.years += i64::from(value),
            relative::Relative::Months(value) => result.months += i64::from(value),
            relative::Relative::Days(value) => result.days += i64::from(value),
            relative::Relative::Hours(value) => result.hours += i64::from(value),
            relative::Relative::Minutes(value) => result.minutes += i64::from(value),
            relative::Relative::Seconds(seconds, nanoseconds) => {
                result.seconds += seconds;
                result.nanoseconds += i64::from(nanoseconds);
            }
        }
    }
    result
}

impl RelativeAdjustment {
    pub(super) fn date_part(self) -> Self {
        Self {
            years: self.years,
            months: self.months,
            days: self.days,
            ..Self::default()
        }
    }

    pub(super) fn time_part(self) -> Self {
        Self {
            hours: self.hours,
            minutes: self.minutes,
            seconds: self.seconds,
            nanoseconds: self.nanoseconds,
            ..Self::default()
        }
    }
}

pub(super) fn apply_relative_date(
    mut dt: Zoned,
    relative: RelativeAdjustment,
) -> Result<Zoned, error::Error> {
    // Apply calendar year/month adjustments together before normalizing the
    // date. This intentionally differs from applying each relative item in
    // lexical order: `2020-02-29 +1 year +1 month` becomes 2021-03-29.
    if relative.years != 0 || relative.months != 0 {
        let (year, month) = shifted_year_month(&dt, relative)?;
        let first = civil::Date::new(year, month as i8, 1)?;
        let target = first.checked_add(Span::new().try_days(i64::from(dt.day() - 1))?)?;
        dt = dt.with().date(target).build()?;
    }
    if relative.days != 0 {
        dt = dt.checked_add(Span::new().try_days(relative.days)?)?;
    }
    Ok(dt)
}

pub(super) fn shifted_year_month(
    dt: &Zoned,
    relative: RelativeAdjustment,
) -> Result<(i16, u8), error::Error> {
    let month_index = i64::from(dt.year())
        .checked_mul(12)
        .and_then(|value| value.checked_add(i64::from(dt.month()) - 1))
        .and_then(|value| value.checked_add(relative.years.checked_mul(12)?))
        .and_then(|value| value.checked_add(relative.months))
        .ok_or("relative month/year overflow")?;
    let year = month_index.div_euclid(12);
    let month = month_index.rem_euclid(12) + 1;
    let year = i16::try_from(year).map_err(|_| "relative year overflow")?;
    Ok((year, month as u8))
}

pub(super) fn apply_relative_time(
    dt: Zoned,
    relative: RelativeAdjustment,
) -> Result<Zoned, error::Error> {
    let seconds = relative
        .hours
        .checked_mul(3_600)
        .and_then(|hours| {
            relative
                .minutes
                .checked_mul(60)
                .and_then(|minutes| hours.checked_add(minutes))
        })
        .and_then(|seconds| seconds.checked_add(relative.seconds))
        .ok_or("relative time overflow")?;
    let extra_seconds = relative.nanoseconds.div_euclid(1_000_000_000);
    let nanoseconds = relative.nanoseconds.rem_euclid(1_000_000_000);
    let seconds = seconds
        .checked_add(extra_seconds)
        .ok_or("relative time overflow")?;
    // Relative::Seconds stores negative fractional values in timestamp form,
    // e.g. (-1, 500_000_000) for -0.5 seconds. Jiff spans require all
    // components to have the same sign, so normalize using truncating
    // division/remainder instead of multiplying large second values by 1e9.
    let (seconds, nanoseconds) = if seconds < 0 && nanoseconds > 0 {
        (
            seconds.checked_add(1).ok_or("relative time overflow")?,
            -(1_000_000_000 - nanoseconds),
        )
    } else {
        (seconds, nanoseconds)
    };
    let span = Span::new()
        .try_seconds(seconds)?
        .try_nanoseconds(nanoseconds)?;
    Ok(dt.checked_add(span)?)
}

pub(super) fn matching_local_zone(dt: &Zoned, name: &str) -> Option<jiff::tz::TimeZone> {
    matching_local_zone_at(dt, dt.datetime(), name)
}

pub(super) fn matching_local_zone_at(
    dt: &Zoned,
    local: civil::DateTime,
    name: &str,
) -> Option<jiff::tz::TimeZone> {
    // GNU's universal abbreviations always denote fixed UTC, even when the
    // effective local zone happens to print the same abbreviation.
    if matches!(name, "utc" | "ut" | "gmt" | "z") {
        return None;
    }

    let zone = dt.time_zone();
    zone.iana_name()?;
    let ambiguous = zone.to_ambiguous_zoned(local);
    for candidate in [
        ambiguous.clone().compatible(),
        ambiguous.clone().earlier(),
        ambiguous.later(),
    ]
    .into_iter()
    .flatten()
    {
        if candidate
            .strftime("%Z")
            .to_string()
            .eq_ignore_ascii_case(name)
        {
            return Some(zone.clone());
        }
    }
    None
}

pub(super) fn named_zone_is_dst(name: &str) -> bool {
    name.ends_with("dt")
        || matches!(
            name,
            "bst" | "west" | "eest" | "cest" | "mest" | "mesz" | "nzdt"
        )
}

pub(super) fn move_standard_named_zones_after_relative(items: &mut Vec<ParsedItem>) {
    let mut index = 0;
    while index < items.len() {
        let move_zone = matches!(
            items.get(index),
            Some(ParsedItem::NamedZone { name, .. })
                if !named_zone_is_dst(name)
                    && items[index + 1..]
                        .iter()
                        .any(|item| matches!(item, ParsedItem::Relative(_)))
        );
        if move_zone {
            let zone = items.remove(index);
            let insert_at = items
                .iter()
                .rposition(|item| matches!(item, ParsedItem::Relative(_)))
                .map_or(items.len(), |position| position + 1);
            items.insert(insert_at, zone);
        } else {
            index += 1;
        }
    }
}

pub(super) fn surrogate_year_for_rules(year: u32) -> i16 {
    // Keep weekday/leap-year parity by preserving `year mod 400`, but avoid
    // upper-bound years that can overflow when resolving a zoned instant.
    const BASE: i64 = 9_599;
    let mapped = BASE + (i64::from(year) - BASE).rem_euclid(400);
    mapped as i16
}

pub(super) fn resolve_rule_offset_for_extended(
    tz: &jiff::tz::TimeZone,
    dt: &ExtendedDateTime,
) -> Result<i32, error::Error> {
    let surrogate_year = surrogate_year_for_rules(dt.year);
    let surrogate_dt = civil::DateTime::new(
        surrogate_year,
        dt.month as i8,
        dt.day as i8,
        dt.hour as i8,
        dt.minute as i8,
        dt.second as i8,
        dt.nanosecond as i32,
    )?;
    let zoned = tz.to_ambiguous_zoned(surrogate_dt).compatible()?;
    Ok(zoned.offset().seconds())
}

pub(super) fn weekday_monday0(day: weekday::Day) -> u8 {
    match day {
        weekday::Day::Monday => 0,
        weekday::Day::Tuesday => 1,
        weekday::Day::Wednesday => 2,
        weekday::Day::Thursday => 3,
        weekday::Day::Friday => 4,
        weekday::Day::Saturday => 5,
        weekday::Day::Sunday => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_a_local_timezone_abbreviation() {
        let zone = jiff::tz::TimeZone::get("Europe/Paris").unwrap();
        let datetime = "2024-01-15 12:00:00"
            .parse::<civil::DateTime>()
            .unwrap()
            .to_zoned(zone)
            .unwrap();

        assert!(matching_local_zone(&datetime, "cet").is_some());
    }

    #[test]
    fn applies_negative_fractional_relative_time() {
        let datetime = "2024-01-01 00:00:00"
            .parse::<civil::DateTime>()
            .unwrap()
            .to_zoned(jiff::tz::TimeZone::UTC)
            .unwrap();
        let relative = relative_adjustment(&[relative::Relative::Seconds(-1, 500_000_000)]);
        let shifted = apply_relative_time(datetime, relative).unwrap();

        assert_eq!(
            shifted.strftime("%Y-%m-%d %H:%M:%S%.9f").to_string(),
            "2023-12-31 23:59:59.500000000"
        );
    }

    #[test]
    fn recognizes_daylight_named_zones() {
        assert!(named_zone_is_dst("edt"));
        assert!(named_zone_is_dst("cest"));
        assert!(!named_zone_is_dst("cet"));
    }
}

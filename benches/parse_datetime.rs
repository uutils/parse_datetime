use codspeed_criterion_compat::{criterion_group, criterion_main, Criterion};
use jiff::Zoned;
use parse_datetime::{parse_datetime, parse_datetime_at_date};

fn bench_iso_datetime(c: &mut Criterion) {
    c.bench_function("parse_iso_datetime", |b| {
        b.iter(|| parse_datetime("2021-02-14 06:37:47 +0000"))
    });
}

fn bench_iso_datetime_t_sep(c: &mut Criterion) {
    c.bench_function("parse_iso_datetime_t_separator", |b| {
        b.iter(|| parse_datetime("2021-02-14T22:37:47-0800"))
    });
}

fn bench_date_only(c: &mut Criterion) {
    c.bench_function("parse_date_only", |b| {
        b.iter(|| parse_datetime("1997-01-01"))
    });
}

fn bench_date_slash_format(c: &mut Criterion) {
    c.bench_function("parse_date_slash_format", |b| {
        b.iter(|| parse_datetime("05/07/1987"))
    });
}

fn bench_epoch_timestamp(c: &mut Criterion) {
    c.bench_function("parse_epoch_timestamp", |b| {
        b.iter(|| parse_datetime("@1613371067"))
    });
}

fn bench_relative_time(c: &mut Criterion) {
    let now = Zoned::now();
    c.bench_function("parse_relative_time", |b| {
        b.iter(|| parse_datetime_at_date(now.clone(), "+3 days"))
    });
}

fn bench_relative_time_complex(c: &mut Criterion) {
    let now = Zoned::now();
    c.bench_function("parse_relative_time_complex", |b| {
        b.iter(|| parse_datetime_at_date(now.clone(), "1 year 3 months 2 days ago"))
    });
}

fn bench_relative_keywords(c: &mut Criterion) {
    c.bench_function("parse_yesterday", |b| {
        b.iter(|| parse_datetime("yesterday"))
    });
    c.bench_function("parse_tomorrow", |b| b.iter(|| parse_datetime("tomorrow")));
    c.bench_function("parse_now", |b| b.iter(|| parse_datetime("now")));
}

fn bench_weekday(c: &mut Criterion) {
    c.bench_function("parse_weekday", |b| b.iter(|| parse_datetime("wednesday")));
}

fn bench_timezone_offset(c: &mut Criterion) {
    c.bench_function("parse_timezone_offset", |b| {
        b.iter(|| parse_datetime("UTC+07:00"))
    });
}

fn bench_datetime_with_delta(c: &mut Criterion) {
    c.bench_function("parse_datetime_with_delta", |b| {
        b.iter(|| parse_datetime("1997-01-01 00:00:00 +0000 +1 year"))
    });
}

fn bench_ctime_format(c: &mut Criterion) {
    c.bench_function("parse_ctime_format", |b| {
        b.iter(|| parse_datetime("Wed Jan  1 00:00:00 1997"))
    });
}

fn bench_datetime_with_timezone_name(c: &mut Criterion) {
    c.bench_function("parse_datetime_with_tz_name", |b| {
        b.iter(|| parse_datetime("1997-01-19 08:17:48 BRT"))
    });
}

fn bench_datetime_ending_in_z(c: &mut Criterion) {
    c.bench_function("parse_datetime_ending_in_z", |b| {
        b.iter(|| parse_datetime("2023-06-03 12:00:01Z"))
    });
}

fn bench_invalid_input(c: &mut Criterion) {
    c.bench_function("parse_invalid_input", |b| {
        b.iter(|| parse_datetime("NotADate"))
    });
}

fn bench_extended_year(c: &mut Criterion) {
    c.bench_function("parse_extended_year", |b| {
        b.iter(|| parse_datetime("10000-01-01"))
    });
}

fn bench_extended_year_rollover(c: &mut Criterion) {
    c.bench_function("parse_extended_year_rollover", |b| {
        b.iter(|| parse_datetime("9999-12-31 +1 day"))
    });
}

fn bench_extended_year_relative(c: &mut Criterion) {
    let base = jiff::civil::DateTime::from(jiff::civil::date(2000, 1, 1))
        .to_zoned(jiff::tz::TimeZone::UTC)
        .unwrap();
    c.bench_function("parse_extended_year_relative", |b| {
        b.iter(|| parse_datetime_at_date(base.clone(), "10000-01-01 +1 day"))
    });
}

fn bench_extended_large_year(c: &mut Criterion) {
    c.bench_function("parse_extended_large_year", |b| {
        b.iter(|| parse_datetime("999999-06-15"))
    });
}

criterion_group!(
    benches,
    bench_iso_datetime,
    bench_iso_datetime_t_sep,
    bench_date_only,
    bench_date_slash_format,
    bench_epoch_timestamp,
    bench_relative_time,
    bench_relative_time_complex,
    bench_relative_keywords,
    bench_weekday,
    bench_timezone_offset,
    bench_datetime_with_delta,
    bench_ctime_format,
    bench_datetime_with_timezone_name,
    bench_datetime_ending_in_z,
    bench_invalid_input,
    bench_extended_year,
    bench_extended_year_rollover,
    bench_extended_year_relative,
    bench_extended_large_year,
);
criterion_main!(benches);

// For the full copyright and license information, view the LICENSE file.

//! Shared names used by calendar-date and weekday grammar items.

pub(super) fn month_number(name: &str) -> Option<u8> {
    Some(match name {
        "january" | "jan" => 1,
        "february" | "feb" => 2,
        "march" | "mar" => 3,
        "april" | "apr" => 4,
        "may" => 5,
        "june" | "jun" => 6,
        "july" | "jul" => 7,
        "august" | "aug" => 8,
        "september" | "sep" | "sept" => 9,
        "october" | "oct" => 10,
        "november" | "nov" => 11,
        "december" | "dec" => 12,
        _ => return None,
    })
}

pub(super) fn weekday_number(name: &str) -> Option<u8> {
    Some(match name {
        "monday" | "mon" | "mon." => 0,
        "tuesday" | "tue" | "tue." | "tues" => 1,
        "wednesday" | "wed" | "wed." | "wednes" => 2,
        "thursday" | "thu" | "thu." | "thur" | "thurs" => 3,
        "friday" | "fri" | "fri." => 4,
        "saturday" | "sat" | "sat." => 5,
        "sunday" | "sun" | "sun." => 6,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{month_number, weekday_number};

    #[test]
    fn recognizes_all_month_names() {
        for (name, number) in [
            ("january", 1),
            ("jan", 1),
            ("february", 2),
            ("feb", 2),
            ("march", 3),
            ("mar", 3),
            ("april", 4),
            ("apr", 4),
            ("may", 5),
            ("june", 6),
            ("jun", 6),
            ("july", 7),
            ("jul", 7),
            ("august", 8),
            ("aug", 8),
            ("september", 9),
            ("sep", 9),
            ("sept", 9),
            ("october", 10),
            ("oct", 10),
            ("november", 11),
            ("nov", 11),
            ("december", 12),
            ("dec", 12),
        ] {
            assert_eq!(month_number(name), Some(number), "{name}");
        }
        assert_eq!(month_number("not-a-month"), None);
    }

    #[test]
    fn recognizes_all_weekday_names() {
        for (name, number) in [
            ("monday", 0),
            ("mon", 0),
            ("mon.", 0),
            ("tuesday", 1),
            ("tue", 1),
            ("tue.", 1),
            ("tues", 1),
            ("wednesday", 2),
            ("wed", 2),
            ("wed.", 2),
            ("wednes", 2),
            ("thursday", 3),
            ("thu", 3),
            ("thu.", 3),
            ("thur", 3),
            ("thurs", 3),
            ("friday", 4),
            ("fri", 4),
            ("fri.", 4),
            ("saturday", 5),
            ("sat", 5),
            ("sat.", 5),
            ("sunday", 6),
            ("sun", 6),
            ("sun.", 6),
        ] {
            assert_eq!(weekday_number(name), Some(number), "{name}");
        }
        assert_eq!(weekday_number("not-a-weekday"), None);
    }
}

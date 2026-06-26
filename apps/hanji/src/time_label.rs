use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CivilDateTime {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
}

pub(crate) fn document_modified_time(path: &Path) -> SystemTime {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or_else(|_| SystemTime::now())
}

pub(crate) fn format_last_edited_time(time: SystemTime) -> String {
    format!(
        "Last edited {}",
        format_civil_datetime(local_datetime(time))
    )
}

#[cfg(unix)]
fn local_datetime(time: SystemTime) -> CivilDateTime {
    let seconds = unix_seconds(time);
    let raw_time = seconds as libc::time_t;
    let mut local_time = std::mem::MaybeUninit::<libc::tm>::uninit();
    let result = unsafe { libc::localtime_r(&raw_time, local_time.as_mut_ptr()) };

    if result.is_null() {
        return utc_datetime_from_unix_seconds(seconds);
    }

    let local_time = unsafe { local_time.assume_init() };
    CivilDateTime {
        year: local_time.tm_year + 1900,
        month: (local_time.tm_mon + 1) as u32,
        day: local_time.tm_mday as u32,
        hour: local_time.tm_hour as u32,
        minute: local_time.tm_min as u32,
    }
}

#[cfg(not(unix))]
fn local_datetime(time: SystemTime) -> CivilDateTime {
    utc_datetime_from_unix_seconds(unix_seconds(time))
}

fn unix_seconds(time: SystemTime) -> i64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs().min(i64::MAX as u64) as i64,
        Err(error) => -(error.duration().as_secs().min(i64::MAX as u64) as i64),
    }
}

fn format_civil_datetime(datetime: CivilDateTime) -> String {
    let month_name = MONTH_NAMES
        .get(datetime.month.saturating_sub(1) as usize)
        .copied()
        .unwrap_or(MONTH_NAMES[0]);
    let (hour, meridiem) = match datetime.hour {
        0 => (12, "AM"),
        1..=11 => (datetime.hour, "AM"),
        12 => (12, "PM"),
        hour => (hour - 12, "PM"),
    };

    format!(
        "{month_name} {}, {}, {}:{:02} {meridiem}",
        datetime.day, datetime.year, hour, datetime.minute
    )
}

fn utc_datetime_from_unix_seconds(seconds: i64) -> CivilDateTime {
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);

    CivilDateTime {
        year,
        month,
        day,
        hour: (seconds_of_day / 3_600) as u32,
        minute: ((seconds_of_day % 3_600) / 60) as u32,
    }
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };

    (year as i32, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_document_timestamp_copy() {
        assert_eq!(
            format_civil_datetime(CivilDateTime {
                year: 2026,
                month: 6,
                day: 27,
                hour: 1,
                minute: 16,
            }),
            "June 27, 2026, 1:16 AM"
        );
    }

    #[test]
    fn formats_midnight_and_noon() {
        assert_eq!(
            format_civil_datetime(CivilDateTime {
                year: 2026,
                month: 6,
                day: 27,
                hour: 0,
                minute: 5,
            }),
            "June 27, 2026, 12:05 AM"
        );
        assert_eq!(
            format_civil_datetime(CivilDateTime {
                year: 2026,
                month: 6,
                day: 27,
                hour: 12,
                minute: 5,
            }),
            "June 27, 2026, 12:05 PM"
        );
    }

    #[test]
    fn converts_unix_epoch_to_civil_date() {
        assert_eq!(
            utc_datetime_from_unix_seconds(0),
            CivilDateTime {
                year: 1970,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
            }
        );
    }
}

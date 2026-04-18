use std::{fmt::Write, time::Duration};

const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 60 * SECONDS_PER_MINUTE;
const SECONDS_PER_DAY: u64 = 24 * SECONDS_PER_HOUR;
const SECONDS_PER_WEEK: u64 = 7 * SECONDS_PER_DAY;
const SECONDS_PER_MONTH: u64 = 30 * SECONDS_PER_DAY; // Is actually 30.41
const SECONDS_PER_YEAR: u64 = 365 * SECONDS_PER_DAY;

/// Formats durations in a rought manner
/// Meant to be used to dislpay and approximate time since a event
pub fn format_duration_approx(f: &mut impl Write, duration: &Duration) {
    let seconds = duration.as_secs();
    if seconds > SECONDS_PER_YEAR * 2 {
        write!(f, "{} years", seconds / SECONDS_PER_YEAR).unwrap();
    } else if seconds > SECONDS_PER_MONTH * 2 {
        write!(f, "{} months", seconds / SECONDS_PER_MONTH).unwrap();
    } else if seconds > SECONDS_PER_WEEK * 2 {
        write!(f, "{} weeks", seconds / SECONDS_PER_WEEK).unwrap();
    } else if seconds > SECONDS_PER_DAY * 2 {
        write!(f, "{} days", seconds / SECONDS_PER_DAY).unwrap();
    } else if seconds > SECONDS_PER_HOUR * 2 {
        write!(f, "{} hours", seconds / SECONDS_PER_HOUR).unwrap();
    } else if seconds > SECONDS_PER_MINUTE * 2 {
        write!(f, "{} minutes", seconds / SECONDS_PER_MINUTE).unwrap();
    } else {
        write!(f, "now").unwrap();
    }
}

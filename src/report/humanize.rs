use std::time::Duration;

const SEC_PER_MIN: u64 = 60;
const SEC_PER_HOUR: u64 = 60 * 60;
const SEC_PER_DAY: u64 = 60 * 60 * 24;

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
pub(crate) fn humanize_age(dur: Duration) -> String {
    let secs = dur.as_secs();
    if secs < SEC_PER_HOUR {
        let mins = secs as f64 / SEC_PER_MIN as f64;
        format!("{mins:.1} min")
    } else if secs < SEC_PER_DAY {
        let hrs = secs as f64 / SEC_PER_HOUR as f64;
        format!("{hrs:.1} hr")
    } else {
        let days = secs as f64 / SEC_PER_DAY as f64;
        format!("{days:.1} days")
    }
}

#[must_use]
pub fn humanize_age_public(dur: Duration) -> String {
    humanize_age(dur)
}

use chrono::{DateTime, Local, LocalResult, NaiveDateTime, TimeZone};

use super::error::GitRewriteError;

pub(crate) fn compute_bounds(
    timestamps: &[DateTime<Local>],
    now: DateTime<Local>,
) -> (Option<u64>, Option<u64>) {
    if timestamps.is_empty() {
        return (None, None);
    }
    let earliest = timestamps.iter().min().copied();
    let latest = timestamps.iter().max().copied();
    let earliest_secs = earliest.map(|dt| diff_seconds(now, dt));
    let latest_secs = latest.map(|dt| diff_seconds(now, dt));
    (earliest_secs, latest_secs)
}

pub(crate) fn diff_seconds(now: DateTime<Local>, other: DateTime<Local>) -> u64 {
    let diff = now.signed_duration_since(other);
    diff.num_seconds().try_into().unwrap_or_default()
}

pub(crate) fn parse_local_datetime(
    match_key: &str,
    value: &str,
) -> Result<DateTime<Local>, GitRewriteError> {
    let naive = NaiveDateTime::parse_from_str(value, "%m/%d/%y %I:%M %p").map_err(|source| {
        GitRewriteError::DateParse {
            match_key: match_key.to_string(),
            value: value.to_string(),
            source,
        }
    })?;
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(dt) | LocalResult::Ambiguous(dt, _) => Ok(dt),
        LocalResult::None => Err(GitRewriteError::DateOutOfRange {
            match_key: match_key.to_string(),
            value: value.to_string(),
        }),
    }
}

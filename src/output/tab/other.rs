use std::time::Duration;

use tabled::{
    builder::Builder,
    settings::{Alignment, Modify, Panel, object::Columns},
};

use crate::{ReportData, humanize_age_public, types::UntrackedReason};

use super::{
    TabStyle,
    style::{apply_style, apply_title_line},
};

pub(crate) fn render(data: &ReportData, style: TabStyle) -> String {
    if data.untracked_repos.is_empty() {
        let mut builder = Builder::default();
        builder.push_record(["(none)"]);
        let mut table = builder.build();
        apply_style(&mut table, style);
        table.with(Panel::header(" Other Repos "));
        return table.to_string();
    }

    let mut builder = Builder::default();
    builder.push_record(["Source", "Status", "Commits", "Earliest", "Latest"]);

    for entry in &data.untracked_repos {
        let commits = entry
            .revs
            .map_or_else(|| "n/a".to_string(), |r| r.to_string());
        let earliest = entry.earliest_secs.map_or_else(
            || "n/a".to_string(),
            |secs| humanize_age_public(Duration::from_secs(secs)),
        );
        let latest = entry.latest_secs.map_or_else(
            || "n/a".to_string(),
            |secs| humanize_age_public(Duration::from_secs(secs)),
        );
        let status = match entry.reason {
            UntrackedReason::Ignored => "ignored",
            UntrackedReason::MissingConfig => "untracked",
            UntrackedReason::MissingRepo => "missing",
        };
        builder.push_record([
            format!("{}:{}", entry.repo, entry.branch),
            status.to_string(),
            commits,
            earliest,
            latest,
        ]);
    }

    let mut table = builder.build();
    apply_style(&mut table, style);
    table.with(Modify::new(Columns::new(2..3)).with(Alignment::right()));
    apply_title_line(&mut table, "Other Repos");
    table.to_string()
}

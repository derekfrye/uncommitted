use std::time::Duration;

use tabled::{
    builder::Builder,
    settings::{Alignment, Modify, Panel, object::Columns},
};

use crate::{ReportData, humanize_age_public};

use super::{
    TabStyle,
    style::{apply_style, apply_title_line},
};

pub(crate) fn render(data: &ReportData, style: TabStyle, show_root: bool) -> String {
    if data.pushable.is_empty() {
        let mut builder = Builder::default();
        builder.push_record(["(none)"]);
        let mut table = builder.build();
        apply_style(&mut table, style);
        table.with(Panel::header(" Pushable Commits "));
        return table.to_string();
    }

    let mut builder = Builder::default();
    if show_root {
        builder.push_record(["Root", "Repo", "Branch", "Revs", "Earliest", "Latest"]);
    } else {
        builder.push_record(["Repo", "Branch", "Revs", "Earliest", "Latest"]);
    }

    let mut rows = data.pushable.clone();
    rows.sort_by(|a, b| {
        let left_root = if show_root { &a.root_display } else { "" };
        let right_root = if show_root { &b.root_display } else { "" };
        (left_root, &a.repo, &a.branch).cmp(&(right_root, &b.repo, &b.branch))
    });

    for entry in &rows {
        let earliest = entry.earliest_secs.map_or_else(
            || "n/a".to_string(),
            |secs| humanize_age_public(Duration::from_secs(secs)),
        );
        let latest = entry.latest_secs.map_or_else(
            || "n/a".to_string(),
            |secs| humanize_age_public(Duration::from_secs(secs)),
        );
        if show_root {
            builder.push_record([
                entry.root_display.clone(),
                entry.repo.clone(),
                entry.branch.clone(),
                entry.revs.to_string(),
                earliest,
                latest,
            ]);
        } else {
            builder.push_record([
                entry.repo.clone(),
                entry.branch.clone(),
                entry.revs.to_string(),
                earliest,
                latest,
            ]);
        }
    }

    let mut table = builder.build();
    apply_style(&mut table, style);
    if show_root {
        // Columns: 0 Root, 1 Repo, 2 Branch, 3 Revs, 4 Earliest, 5 Latest
        table.with(Modify::new(Columns::new(3..4)).with(Alignment::right()));
    } else {
        // Columns: 0 Repo, 1 Branch, 2 Revs, 3 Earliest, 4 Latest
        table.with(Modify::new(Columns::new(2..3)).with(Alignment::right()));
    }
    apply_title_line(&mut table, "Pushable Commits");
    table.to_string()
}

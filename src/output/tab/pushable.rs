use std::time::Duration;

use tabled::{
    builder::Builder,
    settings::{Alignment, Modify, Panel, object::Columns},
};

use crate::{PushableEntry, ReportData, humanize_age_public};

use super::{
    TabStyle,
    style::{apply_style, apply_title_line},
};

pub(crate) fn render(data: &ReportData, style: TabStyle, show_root: bool) -> String {
    if data.pushable.is_empty() {
        return render_empty(style);
    }

    let rows = sorted_rows(data, show_root);
    build_table(rows, style, show_root)
}

fn render_empty(style: TabStyle) -> String {
    let mut builder = Builder::default();
    builder.push_record(["(none)"]);
    let mut table = builder.build();
    apply_style(&mut table, style);
    table.with(Panel::header(" Pushable Commits "));
    table.to_string()
}

fn sorted_rows(data: &ReportData, show_root: bool) -> Vec<PushableEntry> {
    let mut rows = data.pushable.clone();
    rows.sort_by(|a, b| {
        let left_root = if show_root { &a.root_display } else { "" };
        let right_root = if show_root { &b.root_display } else { "" };
        (left_root, &a.repo, &a.branch).cmp(&(right_root, &b.repo, &b.branch))
    });
    rows
}

fn build_table(rows: Vec<PushableEntry>, style: TabStyle, show_root: bool) -> String {
    let mut builder = Builder::default();
    push_header(&mut builder, show_root);
    for entry in rows {
        builder.push_record(row_values(&entry, show_root));
    }

    let mut table = builder.build();
    apply_style(&mut table, style);
    if show_root {
        // Columns: 0 Root, 1 Repo, 2 Branch, 3 Commits, 4 Earliest, 5 Latest
        table.with(Modify::new(Columns::new(3..4)).with(Alignment::right()));
    } else {
        // Columns: 0 Repo, 1 Branch, 2 Commits, 3 Earliest, 4 Latest
        table.with(Modify::new(Columns::new(2..3)).with(Alignment::right()));
    }
    apply_title_line(&mut table, "Pushable Commits");
    table.to_string()
}

fn push_header(builder: &mut Builder, show_root: bool) {
    if show_root {
        builder.push_record(["Root", "Repo", "Branch", "Commits", "Earliest", "Latest"]);
    } else {
        builder.push_record(["Repo", "Branch", "Commits", "Earliest", "Latest"]);
    }
}

fn row_values(entry: &PushableEntry, show_root: bool) -> Vec<String> {
    let earliest = format_age(entry.earliest_secs);
    let latest = format_age(entry.latest_secs);

    if show_root {
        vec![
            entry.root_display.clone(),
            entry.repo.clone(),
            entry.branch.clone(),
            entry.revs.to_string(),
            earliest,
            latest,
        ]
    } else {
        vec![
            entry.repo.clone(),
            entry.branch.clone(),
            entry.revs.to_string(),
            earliest,
            latest,
        ]
    }
}

fn format_age(value: Option<u64>) -> String {
    value
        .map(|secs| humanize_age_public(Duration::from_secs(secs)))
        .unwrap_or_else(|| "n/a".to_string())
}

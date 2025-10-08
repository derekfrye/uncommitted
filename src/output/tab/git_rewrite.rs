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

pub(crate) fn render(data: &ReportData, style: TabStyle) -> String {
    let entries = data
        .git_rewrite
        .as_ref()
        .expect("git rewrite table requested without data");

    if entries.is_empty() {
        let mut builder = Builder::default();
        builder.push_record(["(none)"]);
        let mut table = builder.build();
        apply_style(&mut table, style);
        table.with(Panel::header(" Git Rewrite "));
        return table.to_string();
    }

    let mut rows = entries.clone();
    rows.sort_by(|a, b| (&a.source_repo, &a.target_repo).cmp(&(&b.source_repo, &b.target_repo)));

    let mut builder = Builder::default();
    builder.push_record(["Source", "Target", "Commits", "Earliest", "Latest"]);
    for entry in &rows {
        let earliest = entry.earliest_secs.map_or_else(
            || "n/a".to_string(),
            |secs| humanize_age_public(Duration::from_secs(secs)),
        );
        let latest = entry.latest_secs.map_or_else(
            || "n/a".to_string(),
            |secs| humanize_age_public(Duration::from_secs(secs)),
        );
        builder.push_record([
            format!("{}:{}", entry.source_repo, entry.source_branch),
            format!("{}:{}", entry.target_repo, entry.target_branch),
            entry.commits.to_string(),
            earliest,
            latest,
        ]);
    }

    let mut table = builder.build();
    apply_style(&mut table, style);
    table.with(Modify::new(Columns::new(2..3)).with(Alignment::right()));
    apply_title_line(&mut table, "Git Rewrite");
    table.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::GitRewriteEntry;

    #[test]
    fn render_appends_branch_names_to_source_and_target() {
        let entry = GitRewriteEntry {
            source_repo: "source_dir".to_string(),
            source_branch: "feature".to_string(),
            source_path: "/tmp/source_dir".to_string(),
            target_repo: "target_dir".to_string(),
            target_branch: "main".to_string(),
            target_path: "/tmp/target_dir".to_string(),
            commits: 0,
            earliest_secs: None,
            latest_secs: None,
        };

        let mut data = ReportData::default();
        data.git_rewrite = Some(vec![entry]);

        let output = render(&data, TabStyle::Empty);

        assert!(output.contains("source_dir:feature"));
        assert!(output.contains("target_dir:main"));
    }
}

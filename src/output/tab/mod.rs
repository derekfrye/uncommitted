use std::borrow::Cow;

use clap::ValueEnum;

use crate::ReportData;

mod git_rewrite;
mod other;
mod pushable;
mod staged;
mod style;
mod uncommitted;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum TabStyle {
    Rounded,
    Modern,
    ModernRounded,
    Ascii,
    AsciiRounded,
    Psql,
    Markdown,
    Extended,
    Sharp,
    Dots,
    ReStructuredText,
    Blank,
    Empty,
}

#[must_use]
pub fn format_tab(data: &ReportData, style: TabStyle, omit_non_actionable: bool) -> String {
    let render_data = if omit_non_actionable {
        apply_omit_filter(data)
    } else {
        Cow::Borrowed(data)
    };
    let render_ref: &ReportData = render_data.as_ref();

    let show_root = render_ref.multi_root;
    let mut sections = Vec::with_capacity(4);
    sections.push(uncommitted::render(render_ref, style, show_root));
    sections.push(staged::render(render_ref, style, show_root));
    sections.push(pushable::render(render_ref, style, show_root));
    if render_ref.git_rewrite.is_some() {
        sections.push(git_rewrite::render(render_ref, style));
    }
    if render_ref.untracked_enabled && !omit_non_actionable {
        sections.push(other::render(render_ref, style));
    }
    sections.join("\n")
}

fn apply_omit_filter(data: &ReportData) -> Cow<'_, ReportData> {
    let mut filtered = data.clone();

    filtered.pushable.retain(|entry| entry.revs > 0);
    filtered
        .untracked_repos
        .retain(|entry| entry.revs.map_or(true, |revs| revs > 0));
    if let Some(entries) = filtered.git_rewrite.as_mut() {
        entries.retain(|entry| entry.commits > 0);
    }

    Cow::Owned(filtered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        GitRewriteEntry, PushableEntry, ReportData, UntrackedReason, UntrackedRepoEntry,
    };

    fn pushable_entry(repo: &str, revs: u64) -> PushableEntry {
        PushableEntry {
            repo: repo.to_string(),
            branch: "main".to_string(),
            revs,
            earliest_secs: None,
            latest_secs: None,
            root_display: "~/src".to_string(),
            root_full: "/tmp/src".to_string(),
        }
    }

    #[test]
    fn format_tab_omits_zero_revs_and_commits_when_requested() {
        let mut data = ReportData::default();
        data.pushable = vec![
            pushable_entry("pushable-keep", 2),
            pushable_entry("pushable-drop", 0),
        ];
        data.git_rewrite = Some(vec![
            GitRewriteEntry {
                source_repo: "rewrite-drop-src".to_string(),
                source_branch: "main".to_string(),
                source_path: "/tmp/src1".to_string(),
                target_repo: "rewrite-drop-target".to_string(),
                target_branch: "main".to_string(),
                target_path: "/tmp/dst1".to_string(),
                commits: 0,
                earliest_secs: None,
                latest_secs: None,
            },
            GitRewriteEntry {
                source_repo: "rewrite-keep-src".to_string(),
                source_branch: "dev".to_string(),
                source_path: "/tmp/src2".to_string(),
                target_repo: "rewrite-keep-target".to_string(),
                target_branch: "main".to_string(),
                target_path: "/tmp/dst2".to_string(),
                commits: 1,
                earliest_secs: None,
                latest_secs: None,
            },
        ]);

        let output = format_tab(&data, TabStyle::Empty, true);

        assert!(output.contains("pushable-keep"));
        assert!(!output.contains("pushable-drop"));
        assert!(output.contains("rewrite-keep-src:dev"));
        assert!(!output.contains("rewrite-drop-src:main"));
    }

    #[test]
    fn format_tab_renders_other_repos_only_when_not_omitting() {
        let mut data = ReportData::default();
        data.untracked_enabled = true;
        data.untracked_repos = vec![
            UntrackedRepoEntry {
                repo: "ignored-repo".to_string(),
                branch: "main".to_string(),
                root_display: "~/src".to_string(),
                root_full: "/tmp/src".to_string(),
                revs: Some(5),
                earliest_secs: None,
                latest_secs: None,
                reason: UntrackedReason::Ignored,
            },
            UntrackedRepoEntry {
                repo: "missing-repo".to_string(),
                branch: "dev".to_string(),
                root_display: "~/src".to_string(),
                root_full: "/tmp/src".to_string(),
                revs: Some(0),
                earliest_secs: None,
                latest_secs: None,
                reason: UntrackedReason::MissingConfig,
            },
            UntrackedRepoEntry {
                repo: "/tmp/missing".to_string(),
                branch: "main".to_string(),
                root_display: "/tmp/missing".to_string(),
                root_full: "/tmp/missing".to_string(),
                revs: None,
                earliest_secs: None,
                latest_secs: None,
                reason: UntrackedReason::MissingRepo,
            },
        ];

        let omitted = format_tab(&data, TabStyle::Empty, true);
        assert!(!omitted.contains("Other Repos"));
        assert!(!omitted.contains("ignored-repo:main"));

        let rendered = format_tab(&data, TabStyle::Empty, false);
        assert!(rendered.contains("ignored-repo:main"));
        assert!(rendered.contains("missing-repo:dev"));
        assert!(rendered.contains("ignored"));
        assert!(rendered.contains("untracked"));
        assert!(rendered.contains("missing"));
    }
}

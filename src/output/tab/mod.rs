use std::borrow::Cow;

use clap::ValueEnum;

use crate::ReportData;

mod git_rewrite;
mod pushable;
mod staged;
mod style;
mod uncommitted;
mod untracked;

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
pub fn format_tab(
    data: &ReportData,
    style: TabStyle,
    omit_repos_up_to_date: bool,
) -> (String, usize) {
    let (render_data, omitted) = if omit_repos_up_to_date {
        apply_omit_filter(data)
    } else {
        (Cow::Borrowed(data), 0)
    };
    let render_ref: &ReportData = render_data.as_ref();

    let show_root = render_ref.multi_root;
    let mut sections = Vec::with_capacity(4);
    sections.push(uncommitted::render(render_ref, style, show_root));
    sections.push(staged::render(render_ref, style, show_root));
    sections.push(pushable::render(render_ref, style, show_root));
    if render_ref.untracked_enabled {
        sections.push(untracked::render(render_ref, style));
    }
    if render_ref.git_rewrite.is_some() {
        sections.push(git_rewrite::render(render_ref, style));
    }
    (sections.join("\n"), omitted)
}

fn apply_omit_filter(data: &ReportData) -> (Cow<'_, ReportData>, usize) {
    let mut filtered = data.clone();
    let mut omitted = 0usize;

    let before_pushable = filtered.pushable.len();
    filtered.pushable.retain(|entry| entry.revs > 0);
    omitted += before_pushable - filtered.pushable.len();

    let before_untracked = filtered.untracked_repos.len();
    filtered
        .untracked_repos
        .retain(|entry| entry.revs.map_or(true, |revs| revs > 0));
    omitted += before_untracked - filtered.untracked_repos.len();

    if let Some(entries) = filtered.git_rewrite.as_mut() {
        let before_git_rewrite = entries.len();
        entries.retain(|entry| entry.commits > 0);
        omitted += before_git_rewrite - entries.len();
    }

    (Cow::Owned(filtered), omitted)
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
        data.untracked_enabled = true;
        data.untracked_repos = vec![
            UntrackedRepoEntry {
                repo: "untracked-keep".to_string(),
                branch: "main".to_string(),
                root_display: "~/src".to_string(),
                root_full: "/tmp/src".to_string(),
                revs: Some(1),
                earliest_secs: None,
                latest_secs: None,
                reason: UntrackedReason::Ignored,
            },
            UntrackedRepoEntry {
                repo: "untracked-drop".to_string(),
                branch: "main".to_string(),
                root_display: "~/src".to_string(),
                root_full: "/tmp/src".to_string(),
                revs: Some(0),
                earliest_secs: None,
                latest_secs: None,
                reason: UntrackedReason::Ignored,
            },
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

        let (output, omitted) = format_tab(&data, TabStyle::Empty, true);

        assert_eq!(omitted, 3);
        assert!(output.contains("pushable-keep"));
        assert!(!output.contains("pushable-drop"));
        assert!(output.contains("untracked-keep"));
        assert!(!output.contains("untracked-drop"));
        assert!(output.contains("rewrite-keep-src:dev"));
        assert!(!output.contains("rewrite-drop-src:main"));
    }
}

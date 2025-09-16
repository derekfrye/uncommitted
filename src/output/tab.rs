use clap::ValueEnum;
use tabled::{
    builder::Builder,
    settings::{
        Alignment, Modify, Panel, Style,
        object::{Columns, Rows},
        style::LineText,
    },
};

use crate::{ReportData, humanize_age_public};

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
pub fn format_tab(data: &ReportData, style: TabStyle) -> String {
    let show_root = data.multi_root;
    let sections = [
        render_uncommitted(data, style, show_root),
        render_staged(data, style, show_root),
        render_pushable(data, style, show_root),
    ];
    sections.join("\n")
}

fn render_uncommitted(data: &ReportData, style: TabStyle, show_root: bool) -> String {
    if data.uncommitted.is_empty() {
        let mut b = Builder::default();
        b.push_record(["(none)"]);
        let mut t = b.build();
        apply_style(&mut t, style);
        t.with(Panel::header(" Uncommitted Changes "));
        return t.to_string();
    }
    let mut b = Builder::default();
    if show_root {
        b.push_record(["Repo", "Root", "Lines", "Files", "Untracked"]);
    } else {
        b.push_record(["Repo", "Lines", "Files", "Untracked"]);
    }
    for e in &data.uncommitted {
        if show_root {
            b.push_record([
                e.repo.clone(),
                e.root_display.clone(),
                e.lines.to_string(),
                e.files.to_string(),
                e.untracked.to_string(),
            ]);
        } else {
            b.push_record([
                e.repo.clone(),
                e.lines.to_string(),
                e.files.to_string(),
                e.untracked.to_string(),
            ]);
        }
    }
    let mut t = b.build();
    apply_style(&mut t, style);
    if show_root {
        // Right align numeric columns taking into account the root column
        t.with(Modify::new(Columns::new(2..3)).with(Alignment::right()));
        t.with(Modify::new(Columns::new(3..4)).with(Alignment::right()));
        t.with(Modify::new(Columns::new(4..5)).with(Alignment::right()));
    } else {
        t.with(Modify::new(Columns::new(1..2)).with(Alignment::right()));
        t.with(Modify::new(Columns::new(2..3)).with(Alignment::right()));
        t.with(Modify::new(Columns::new(3..4)).with(Alignment::right()));
    }
    apply_title_line(&mut t, "Uncommitted Changes");
    t.to_string()
}

fn render_staged(data: &ReportData, style: TabStyle, show_root: bool) -> String {
    if data.staged.is_empty() {
        let mut b = Builder::default();
        b.push_record(["(none)"]);
        let mut t = b.build();
        apply_style(&mut t, style);
        t.with(Panel::header(" Staged Changes "));
        return t.to_string();
    }
    let mut b = Builder::default();
    if show_root {
        b.push_record(["Repo", "Root", "Lines", "Files", "Untracked"]);
    } else {
        b.push_record(["Repo", "Lines", "Files", "Untracked"]);
    }
    for e in &data.staged {
        if show_root {
            b.push_record([
                e.repo.clone(),
                e.root_display.clone(),
                e.lines.to_string(),
                e.files.to_string(),
                e.untracked.to_string(),
            ]);
        } else {
            b.push_record([
                e.repo.clone(),
                e.lines.to_string(),
                e.files.to_string(),
                e.untracked.to_string(),
            ]);
        }
    }
    let mut t = b.build();
    apply_style(&mut t, style);
    if show_root {
        t.with(Modify::new(Columns::new(2..3)).with(Alignment::right()));
        t.with(Modify::new(Columns::new(3..4)).with(Alignment::right()));
        t.with(Modify::new(Columns::new(4..5)).with(Alignment::right()));
    } else {
        t.with(Modify::new(Columns::new(1..2)).with(Alignment::right()));
        t.with(Modify::new(Columns::new(2..3)).with(Alignment::right()));
        t.with(Modify::new(Columns::new(3..4)).with(Alignment::right()));
    }
    apply_title_line(&mut t, "Staged Changes");
    t.to_string()
}

fn render_pushable(data: &ReportData, style: TabStyle, show_root: bool) -> String {
    if data.pushable.is_empty() {
        let mut b = Builder::default();
        b.push_record(["(none)"]);
        let mut t = b.build();
        apply_style(&mut t, style);
        t.with(Panel::header(" Pushable Commits "));
        return t.to_string();
    }
    let mut b = Builder::default();
    if show_root {
        b.push_record(["Repo", "Root", "Revs", "Earliest", "Latest"]);
    } else {
        b.push_record(["Repo", "Revs", "Earliest", "Latest"]);
    }
    for e in &data.pushable {
        let earliest = e.earliest_secs.map_or_else(
            || "n/a".to_string(),
            |s| humanize_age_public(std::time::Duration::from_secs(s)),
        );
        let latest = e.latest_secs.map_or_else(
            || "n/a".to_string(),
            |s| humanize_age_public(std::time::Duration::from_secs(s)),
        );
        if show_root {
            b.push_record([
                e.repo.clone(),
                e.root_display.clone(),
                e.revs.to_string(),
                earliest,
                latest,
            ]);
        } else {
            b.push_record([e.repo.clone(), e.revs.to_string(), earliest, latest]);
        }
    }
    let mut t = b.build();
    apply_style(&mut t, style);
    if show_root {
        t.with(Modify::new(Columns::new(2..3)).with(Alignment::right()));
    } else {
        t.with(Modify::new(Columns::new(1..2)).with(Alignment::right()));
    }
    apply_title_line(&mut t, "Pushable Commits");
    t.to_string()
}

fn apply_style(t: &mut tabled::Table, style: TabStyle) {
    match style {
        TabStyle::Rounded => t.with(Style::rounded()),
        TabStyle::Modern => t.with(Style::modern()),
        TabStyle::ModernRounded => t.with(Style::modern_rounded()),
        TabStyle::Ascii => t.with(Style::ascii()),
        TabStyle::AsciiRounded => t.with(Style::ascii_rounded()),
        TabStyle::Psql => t.with(Style::psql()),
        TabStyle::Markdown => t.with(Style::markdown()),
        TabStyle::Extended => t.with(Style::extended()),
        TabStyle::Sharp => t.with(Style::sharp()),
        TabStyle::Dots => t.with(Style::dots()),
        TabStyle::ReStructuredText => t.with(Style::re_structured_text()),
        TabStyle::Blank => t.with(Style::blank()),
        TabStyle::Empty => t.with(Style::empty()),
    };
}

fn apply_title_line(t: &mut tabled::Table, title: &str) {
    t.with(LineText::new(format!(" {title} "), Rows::first()).offset(1));
}

use clap::ValueEnum;

use crate::ReportData;

mod git_rewrite;
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
pub fn format_tab(data: &ReportData, style: TabStyle) -> String {
    let show_root = data.multi_root;
    let mut sections = Vec::with_capacity(4);
    sections.push(uncommitted::render(data, style, show_root));
    sections.push(staged::render(data, style, show_root));
    sections.push(pushable::render(data, style, show_root));
    if data.git_rewrite.is_some() {
        sections.push(git_rewrite::render(data, style));
    }
    sections.join("\n")
}

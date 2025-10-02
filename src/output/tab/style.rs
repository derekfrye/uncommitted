use tabled::{
    Table,
    settings::{Style, object::Rows, style::LineText},
};

use super::TabStyle;

pub(crate) fn apply_style(table: &mut Table, style: TabStyle) {
    match style {
        TabStyle::Rounded => table.with(Style::rounded()),
        TabStyle::Modern => table.with(Style::modern()),
        TabStyle::ModernRounded => table.with(Style::modern_rounded()),
        TabStyle::Ascii => table.with(Style::ascii()),
        TabStyle::AsciiRounded => table.with(Style::ascii_rounded()),
        TabStyle::Psql => table.with(Style::psql()),
        TabStyle::Markdown => table.with(Style::markdown()),
        TabStyle::Extended => table.with(Style::extended()),
        TabStyle::Sharp => table.with(Style::sharp()),
        TabStyle::Dots => table.with(Style::dots()),
        TabStyle::ReStructuredText => table.with(Style::re_structured_text()),
        TabStyle::Blank => table.with(Style::blank()),
        TabStyle::Empty => table.with(Style::empty()),
    };
}

pub(crate) fn apply_title_line(table: &mut Table, title: &str) {
    table.with(LineText::new(format!(" {title} "), Rows::first()).offset(1));
}

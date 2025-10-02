use tabled::{
    builder::Builder,
    settings::{Alignment, Modify, Panel, object::Columns},
};

use crate::ReportData;

use super::{
    TabStyle,
    style::{apply_style, apply_title_line},
};

pub(crate) fn render(data: &ReportData, style: TabStyle, show_root: bool) -> String {
    if data.staged.is_empty() {
        let mut builder = Builder::default();
        builder.push_record(["(none)"]);
        let mut table = builder.build();
        apply_style(&mut table, style);
        table.with(Panel::header(" Staged Changes "));
        return table.to_string();
    }

    let mut builder = Builder::default();
    if show_root {
        builder.push_record(["Root", "Repo", "Branch", "Lines", "Files", "Untracked"]);
    } else {
        builder.push_record(["Repo", "Branch", "Lines", "Files", "Untracked"]);
    }

    for entry in &data.staged {
        if show_root {
            builder.push_record([
                entry.root_display.clone(),
                entry.repo.clone(),
                entry.branch.clone(),
                entry.lines.to_string(),
                entry.files.to_string(),
                entry.untracked.to_string(),
            ]);
        } else {
            builder.push_record([
                entry.repo.clone(),
                entry.branch.clone(),
                entry.lines.to_string(),
                entry.files.to_string(),
                entry.untracked.to_string(),
            ]);
        }
    }

    let mut table = builder.build();
    apply_style(&mut table, style);
    if show_root {
        // Columns: 0 Root, 1 Repo, 2 Branch, 3 Lines, 4 Files, 5 Untracked
        table.with(Modify::new(Columns::new(3..4)).with(Alignment::right()));
        table.with(Modify::new(Columns::new(4..5)).with(Alignment::right()));
        table.with(Modify::new(Columns::new(5..6)).with(Alignment::right()));
    } else {
        // Columns: 0 Repo, 1 Branch, 2 Lines, 3 Files, 4 Untracked
        table.with(Modify::new(Columns::new(2..3)).with(Alignment::right()));
        table.with(Modify::new(Columns::new(3..4)).with(Alignment::right()));
        table.with(Modify::new(Columns::new(4..5)).with(Alignment::right()));
    }
    apply_title_line(&mut table, "Staged Changes");
    table.to_string()
}

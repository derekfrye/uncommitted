use crate::{ReportData, types::UntrackedReason};
use std::fmt::Write as _;

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let code = c as u32;
                let _ = std::fmt::Write::write_fmt(&mut out, format_args!("\\u{code:04x}"));
            }
            other => out.push(other),
        }
    }
    out
}

#[must_use]
pub fn to_json(data: &ReportData) -> String {
    let mut out = String::new();
    out.push('{');
    write_uncommitted(&mut out, data);
    write_staged(&mut out, data);
    write_pushable(&mut out, data);
    write_untracked(&mut out, data);
    write_git_rewrite(&mut out, data);
    out.push('}');
    out
}

fn write_uncommitted(out: &mut String, data: &ReportData) {
    out.push_str("\"uncommitted\": [");
    for (i, e) in data.uncommitted.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push('{');
        let _ = write!(
            out,
            "\"repo\":\"{}\", \"branch\":\"{}\", \"lines\":{}, \"files\":{}, \"untracked\":{}, \"root\":\"{}\"",
            json_escape(&e.repo),
            json_escape(&e.branch),
            e.lines,
            e.files,
            e.untracked,
            json_escape(&e.root_full)
        );
        out.push('}');
    }
    out.push(']');
}

fn write_staged(out: &mut String, data: &ReportData) {
    out.push_str(", \"staged\": [");
    for (i, e) in data.staged.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push('{');
        let _ = write!(
            out,
            "\"repo\":\"{}\", \"branch\":\"{}\", \"lines\":{}, \"files\":{}, \"untracked\":{}, \"root\":\"{}\"",
            json_escape(&e.repo),
            json_escape(&e.branch),
            e.lines,
            e.files,
            e.untracked,
            json_escape(&e.root_full)
        );
        out.push('}');
    }
    out.push(']');
}

fn write_pushable(out: &mut String, data: &ReportData) {
    out.push_str(", \"pushable\": [");
    for (i, e) in data.pushable.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push('{');
        let _ = write!(out, "\"repo\":\"{}\", ", json_escape(&e.repo));
        let _ = write!(out, "\"branch\":\"{}\", ", json_escape(&e.branch));
        let _ = write!(out, "\"revs\":{}", e.revs);
        out.push_str(", ");
        match e.earliest_secs {
            Some(v) => {
                let _ = write!(out, "\"earliest_secs\":{v}");
            }
            None => out.push_str("\"earliest_secs\":null"),
        }
        out.push_str(", ");
        match e.latest_secs {
            Some(v) => {
                let _ = write!(out, "\"latest_secs\":{v}");
            }
            None => out.push_str("\"latest_secs\":null"),
        }
        out.push_str(", ");
        let _ = write!(out, "\"root\":\"{}\"", json_escape(&e.root_full));
        out.push('}');
    }
    out.push(']');
}

fn write_untracked(out: &mut String, data: &ReportData) {
    out.push_str(", \"untracked_repos\": ");
    if !data.untracked_enabled {
        out.push_str("null");
        return;
    }

    out.push('[');
    for (i, entry) in data.untracked_repos.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push('{');
        let _ = write!(
            out,
            "\"repo\":\"{}\", \"branch\":\"{}\", \"root\":\"{}\", \"root_display\":\"{}\", ",
            json_escape(&entry.repo),
            json_escape(&entry.branch),
            json_escape(&entry.root_full),
            json_escape(&entry.root_display)
        );
        match entry.revs {
            Some(v) => {
                let _ = write!(out, "\"revs\":{v}");
            }
            None => out.push_str("\"revs\":null"),
        }
        out.push_str(", ");
        match entry.earliest_secs {
            Some(v) => {
                let _ = write!(out, "\"earliest_secs\":{v}");
            }
            None => out.push_str("\"earliest_secs\":null"),
        }
        out.push_str(", ");
        match entry.latest_secs {
            Some(v) => {
                let _ = write!(out, "\"latest_secs\":{v}");
            }
            None => out.push_str("\"latest_secs\":null"),
        }
        out.push_str(", ");
        let reason = match entry.reason {
            UntrackedReason::Ignored => "ignored",
            UntrackedReason::MissingConfig => "missing_config",
        };
        let _ = write!(out, "\"reason\":\"{reason}\"");
        out.push('}');
    }
    out.push(']');
}

fn write_git_rewrite(out: &mut String, data: &ReportData) {
    out.push_str(", \"git_rewrite\": ");
    match data.git_rewrite.as_ref() {
        None => out.push_str("null"),
        Some(entries) => {
            out.push('[');
            for (i, entry) in entries.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push('{');
                let _ = write!(
                    out,
                    "\"source_repo\":\"{}\", \"source_branch\":\"{}\", \"source_path\":\"{}\", ",
                    json_escape(&entry.source_repo),
                    json_escape(&entry.source_branch),
                    json_escape(&entry.source_path)
                );
                let _ = write!(
                    out,
                    "\"target_repo\":\"{}\", \"target_branch\":\"{}\", \"target_path\":\"{}\", ",
                    json_escape(&entry.target_repo),
                    json_escape(&entry.target_branch),
                    json_escape(&entry.target_path)
                );
                let _ = write!(out, "\"commits\":{}", entry.commits);
                out.push_str(", ");
                match entry.earliest_secs {
                    Some(v) => {
                        let _ = write!(out, "\"earliest_secs\":{v}");
                    }
                    None => out.push_str("\"earliest_secs\":null"),
                }
                out.push_str(", ");
                match entry.latest_secs {
                    Some(v) => {
                        let _ = write!(out, "\"latest_secs\":{v}");
                    }
                    None => out.push_str("\"latest_secs\":null"),
                }
                out.push('}');
            }
            out.push(']');
        }
    }
}

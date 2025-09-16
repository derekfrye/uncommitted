#![forbid(unsafe_code)]
#![deny(warnings, clippy::all, clippy::pedantic)]

use crate::ReportData;

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
                let _ = std::fmt::Write::write_fmt(&mut out, format_args!("\\u{:04x}", code));
            }
            other => out.push(other),
        }
    }
    out
}

pub fn to_json(data: &ReportData) -> String {
    let mut out = String::new();
    out.push('{');
    write_uncommitted(&mut out, data);
    write_staged(&mut out, data);
    write_pushable(&mut out, data);
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
        let _ = std::fmt::Write::write_fmt(
            out,
            format_args!(
                "\"repo\":\"{}\", \"lines\":{}, \"files\":{}, \"untracked\":{}",
                json_escape(&e.repo),
                e.lines, e.files, e.untracked
            ),
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
        let _ = std::fmt::Write::write_fmt(
            out,
            format_args!(
                "\"repo\":\"{}\", \"lines\":{}, \"files\":{}, \"untracked\":{}",
                json_escape(&e.repo),
                e.lines, e.files, e.untracked
            ),
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
        out.push_str(&format!("\"repo\":\"{}\", ", json_escape(&e.repo)));
        out.push_str(&format!("\"revs\":{}", e.revs));
        out.push_str(", ");
        match e.earliest_secs {
            Some(v) => out.push_str(&format!("\"earliest_secs\":{}", v)),
            None => out.push_str("\"earliest_secs\":null"),
        }
        out.push_str(", ");
        match e.latest_secs {
            Some(v) => out.push_str(&format!("\"latest_secs\":{}", v)),
            None => out.push_str("\"latest_secs\":null"),
        }
        out.push('}');
    }
    out.push(']');
}

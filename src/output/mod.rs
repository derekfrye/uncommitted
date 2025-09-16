#![forbid(unsafe_code)]
#![deny(warnings, clippy::all, clippy::pedantic)]

pub mod tab;
pub mod json;

pub use tab::{format_tab, TabStyle};
pub use json::to_json;


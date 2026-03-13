mod entries;
mod scenarios;
mod support;
mod untracked;

use super::config::match_key_to_string;

#[test]
fn match_key_accepts_integer() {
    #[derive(serde::Deserialize)]
    struct Wrapper {
        #[serde(deserialize_with = "match_key_to_string")]
        value: String,
    }

    let cfg: Wrapper = toml::from_str("value = 7").expect("parse");
    assert_eq!(cfg.value, "7");
}

use std::path::PathBuf;

use tempfile::TempDir;

use super::support::{temp_pair_dirs, write_file};

mod args;
mod results;
mod validation;

struct EntryFixture {
    temp: TempDir,
    config_path: PathBuf,
    target_dir: PathBuf,
}

fn pair_config(source_extra: &str, target_extra: &str, match_key: u64) -> EntryFixture {
    let (temp, source_dir, target_dir) = temp_pair_dirs();
    let config_path = temp.path().join("config.toml");
    let config_contents = format!(
        "\
[[repo]]
repository-path = \"{src}\"
repository-branch = \"main\"
{source_extra}match-key = {match_key}
repo-type = \"source\"

[[repo]]
repository-path = \"{dst}\"
repository-branch = \"dev\"
{target_extra}match-key = {match_key}
repo-type = \"target\"
",
        src = source_dir.display(),
        dst = target_dir.display(),
    );
    write_file(&config_path, config_contents);

    EntryFixture {
        temp,
        config_path,
        target_dir,
    }
}

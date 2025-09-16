use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub trait FsOps {
    fn is_repo(&self, dir: &Path) -> bool;
    fn expand_tilde(&self, p: &Path) -> PathBuf;
}

pub struct DefaultFsOps;
impl FsOps for DefaultFsOps {
    fn is_repo(&self, dir: &Path) -> bool {
        dir.join(".git").is_dir()
    }
    fn expand_tilde(&self, p: &Path) -> PathBuf {
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            if p.starts_with("~")
                && let Ok(rest) = p.strip_prefix("~")
            {
                return home.join(rest);
            }
        }
        p.to_path_buf()
    }
}

pub trait Clock {
    fn now(&self) -> SystemTime;
}

pub struct DefaultClock;
impl Clock for DefaultClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

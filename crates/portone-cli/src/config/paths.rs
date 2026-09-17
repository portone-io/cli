use std::path::PathBuf;

use etcetera::BaseStrategy;

fn env_override(name: &str) -> Option<PathBuf> {
    let value = std::env::var(name).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

fn base_strategy() -> impl BaseStrategy {
    etcetera::choose_base_strategy().expect("unable to determine the home directory")
}

pub fn config_dir() -> PathBuf {
    env_override("PORTONE_CONFIG_DIR")
        .unwrap_or_else(|| base_strategy().config_dir().join("portone"))
}

pub(crate) fn try_config_dir() -> Option<PathBuf> {
    env_override("PORTONE_CONFIG_DIR").or_else(|| {
        etcetera::choose_base_strategy()
            .ok()
            .map(|base| base.config_dir().join("portone"))
    })
}

pub fn cache_dir() -> PathBuf {
    env_override("PORTONE_CACHE_DIR").unwrap_or_else(|| base_strategy().cache_dir().join("portone"))
}

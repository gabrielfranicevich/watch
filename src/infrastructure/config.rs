use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub quality: String,
    pub player: Option<String>,
    pub menu: Option<String>,
    pub config_dir: Option<PathBuf>,
    pub data_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            quality: "best".into(),
            player: None,
            menu: None,
            config_dir: super::paths::config_dir(),
            data_dir: super::paths::data_dir(),
        }
    }
}

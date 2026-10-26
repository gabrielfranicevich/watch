use std::path::PathBuf;

pub fn config_dir() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("ani-cli"))
    } else if cfg!(target_os = "macos") {
        home_dir().map(|path| {
            path.join("Library")
                .join("Application Support")
                .join("ani-cli")
        })
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|path| path.join(".config")))
            .map(|path| path.join("ani-cli"))
    }
}

/// Location of the tab-delimited history file used by the legacy shell CLI.
/// ANI_CLI_HIST_DIR is a directory override, not a filename.
pub fn history_file() -> Option<PathBuf> {
    if let Some(directory) = std::env::var_os("ANI_CLI_HIST_DIR") {
        return Some(PathBuf::from(directory).join("ani-hsts"));
    }

    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|path| path.join(".local").join("state")))
        .map(|path| path.join("ani-cli").join("ani-hsts"))
}

pub fn data_dir() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("ani-cli"))
    } else if cfg!(target_os = "macos") {
        home_dir().map(|path| {
            path.join("Library")
                .join("Application Support")
                .join("ani-cli")
        })
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|path| path.join(".local").join("share")))
            .map(|path| path.join("ani-cli"))
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

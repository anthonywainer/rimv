use std::path::{Path, PathBuf};

/// Resolve RimV's per-user data root while preserving its macOS location.
pub fn data_root(home: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData/Local"))
            .join("RimV")
    }

    #[cfg(target_os = "macos")]
    {
        home.join("Library/Application Support/rimv")
    }

    #[cfg(target_os = "linux")]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share"))
            .join("rimv")
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        home.join(".rimv")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_data_root_preserves_existing_location() {
        if cfg!(target_os = "macos") {
            assert_eq!(
                data_root(Path::new("/Users/test")),
                PathBuf::from("/Users/test/Library/Application Support/rimv")
            );
        }
    }
}

use std::env;
use std::ffi::OsString;
use std::io::BufRead;
use std::path::{Path, PathBuf};

fn is_valid_config(p: &Path) -> bool {
    log::debug!("looking for config at {}", p.display());
    p.exists() && p.is_file()
}

fn get_config_path(skip_default_locations: bool) -> Option<OsString> {
    // check if config env var is set and use it
    if let Some(path) = env::var_os("PBCLI_CONFIG_PATH") {
        log::debug!("using config pointed to by PBCLI_CONFIG_PATH");
        return Some(path);
    };

    log::debug!("PBCLI_CONFIG_PATH not set");

    if skip_default_locations {
        log::debug!("skip_default_locations set. not checking default config locations");
        return None;
    }

    // check user specific config location
    let project_dirs = directories::ProjectDirs::from("eu", "mydayyy", env!("CARGO_PKG_NAME"))?;
    let user_config_dir = project_dirs.config_local_dir();
    let user_config_file = user_config_dir.join("config");
    if is_valid_config(&user_config_file) {
        return Some(user_config_file.into());
    }

    // linux only: check /etc/pbcli/config
    if cfg!(unix) {
        let system_config_file = PathBuf::from("/etc/pbcli/config");
        if is_valid_config(&system_config_file) {
            return Some(system_config_file.into());
        }
    }

    None
}

pub fn get_config_args(skip_default_locations: bool) -> Vec<OsString> {
    let path = match get_config_path(skip_default_locations) {
        None => {
            log::debug!("no config found");
            return vec![];
        }
        Some(path) => path,
    };

    log::debug!("using config {}", path.to_string_lossy());

    let handle = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(_) => {
            log::debug!("failed to open config. using cli args only");
            return vec![];
        } // TODO: Raise error
    };

    let reader = std::io::BufReader::new(handle);
    let mut config_args: Vec<OsString> = vec![];
    reader.lines().for_each(|line| {
        let line = match line {
            Ok(line) => line.trim().to_owned(),
            Err(_) => return,
        };

        if line.starts_with('#') {
            return;
        }

        config_args.push(line.into());
    });

    config_args
}

pub fn get_cli_args() -> Vec<OsString> {
    std::env::args_os().collect()
}

fn has_flag(flag_name: &str) -> bool {
    get_cli_args().contains(&OsString::from(flag_name))
}
pub fn has_debug_flag() -> bool {
    has_flag("--debug")
}

pub fn has_skip_default_config_flag() -> bool {
    has_flag("--skip-default-config")
}

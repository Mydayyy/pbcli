use std::env;
use std::ffi::OsString;
use std::io::BufRead;
use std::path::{Path, PathBuf};

fn is_valid_config(p: &Path) -> bool {
    log::debug!("looking for config at {}", p.display());
    p.exists() && p.is_file()
}

// TODO: Split in early stage config parsing and late stage
fn get_config_path() -> Option<OsString> {
    // check if config env var is set and use it
    match env::var_os("PBCLI_CONFIG_PATH") {
        Some(path) => return {
            log::debug!("using config pointed to by PBCLI_CONFIG_PATH");
            Some(path)
        },
        None => {}
    };

    log::debug!("PBCLI_CONFIG_PATH not set");

    // check user specific config location
    let project_dirs = directories::ProjectDirs::from("eu", "mydayyy", env!("CARGO_PKG_NAME"))?;
    let user_config_dir = project_dirs.config_local_dir();
    let user_config_file = user_config_dir.join("config");
    if is_valid_config(&user_config_file) { return Some(user_config_file.into()); }

    // linux only: check /etc/pbcli/config
    if cfg!(unix) {
        let system_config_file = PathBuf::from("/etc/pbcli/config");
        if is_valid_config(&system_config_file) { return Some(system_config_file.into()); }
    }

    None
}

pub fn get_args() -> Vec<OsString> {
    let mut env_args = std::env::args_os();

    let path = match get_config_path() {
        None => {
            log::debug!("no config found");
            return env_args.collect();
        }
        Some(path) => path,
    };

    log::debug!("using config {}", path.to_string_lossy());

    let handle = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(_) => {
            log::debug!("failed to open config. using cli args only");
            return env_args.collect();
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

    if let Some(binary_path) = env_args.next() {
        config_args.insert(0, binary_path);
    }

    config_args.extend(env_args);

    config_args
}

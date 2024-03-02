use std::env;
use std::ffi::{ OsString};
use std::io::BufRead;

pub fn get_args() -> Vec<OsString> {
    let mut env_args = std::env::args_os();


    let path = match env::var_os("PBCLI_CONFIG_PATH") {
        None => return env_args.collect(),
        Some(path) => path
    };

    let handle = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return env_args.collect() // TODO: Raise error
    };

    let reader = std::io::BufReader::new(handle);
    let mut config_args: Vec<OsString> = vec![];
    reader.lines().for_each(|line| {
        let line = match line {
            Ok(line) => line.trim().to_owned(),
            Err(_) => return
        };

        if line.starts_with("#") {
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
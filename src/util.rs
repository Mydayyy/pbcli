use std::io::IsTerminal;
use std::process::exit;
use std::sync::Mutex;

pub fn check_filesize(len: u64, opts_size_limt: Option<u64>) {
    static MUTEX_IS_CONFIRMED: Mutex<bool> = Mutex::new(false);
    let mut user_confirmed_size = MUTEX_IS_CONFIRMED.lock().unwrap();

    if *user_confirmed_size {
        return;
    }

    if let Some(size_limit) = opts_size_limt {
        if len > size_limit {
            if !std::io::stdin().is_terminal() {
                exit(1)
            }

            let confirmation = dialoguer::Confirm::new()
                .with_prompt("This paste exceeds your defined size limit. Continue?")
                .interact()
                .unwrap();

            if !confirmation {
                exit(1)
            }
            *user_confirmed_size = true;
        }
    }
}

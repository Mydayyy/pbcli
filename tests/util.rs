use assert_cmd::Command;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{env, fs};

static TEST_DIR: &str = "pbcli-tests";
static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

pub fn setup(test_name: &str) -> (TestHarness, Command) {
    let test_harness = TestHarness::new(test_name);
    let pbcli = test_harness.cmd();
    // let cmd = dir.command();
    println!("Test harness created: {:?}", test_harness);
    (test_harness, pbcli)
}

#[derive(Debug)]
pub struct TestHarness {
    root: PathBuf,
    dir: PathBuf,
}

impl TestHarness {
    pub fn new(name: &str) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let root1 = env::current_exe().unwrap();
        let root = env::current_exe().unwrap().parent().unwrap().to_path_buf();
        let dir = env::temp_dir()
            .join(TEST_DIR)
            .join(name)
            .join(&format!("{}", id));
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }
        fs::create_dir_all(&dir).unwrap();
        TestHarness { root, dir }
    }

    pub fn cmd(&self) -> Command {
        let mut pbcli = Command::cargo_bin("pbcli").expect("pbcli not found");
        pbcli.current_dir(&self.dir);

        pbcli.arg("--host").arg("https://paste.mydayyy.eu");

        pbcli
    }

    pub fn create_file(&self, name: &str, content: &str) -> PathBuf {
        let path = self.dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }
}

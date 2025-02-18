use assert_cmd::Command;
use std::path::PathBuf;

pub struct TestHarness {
    root: PathBuf,
    cmd: Command,
}

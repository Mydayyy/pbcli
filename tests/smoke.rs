use crate::util::{setup, TestHarness};
use assert_cmd::Command;
use pbcli::{api, Opts};
use predicates::prelude::*;
use url::Url;

mod util;
#[macro_use]
mod macros;

#[test]
fn test_add() {
    assert_eq!(true, true);
}

const INSTANCE: &str = "https://paste.mydayyy.eu";

#[test]
fn harness() {
    // let _harness = TestHarness::new("harness");
    let (harness, mut pbcli) = setup("harness");
    harness.create_file("test.txt", "test content");
    let binding = pbcli.arg("--upload").arg("test.txt").assert().success();
    let o = binding.get_output().clone();
    println!("{:?}", String::from_utf8(o.stdout));
}

#[test]
fn upload_download() {
    let (harness, mut pbcli) = setup("upload");
    harness.create_file("in.txt", "test content");
    pbcli.arg("--upload").arg("in.txt").assert().success();
    let url: String = String::from_utf8(pbcli.output().unwrap().stdout.clone()).unwrap();

    println!("URL: {}", url);

    let (harness, mut pbcli) = setup("download");
    pbcli.arg("--download").arg("out.txt").assert().success();
}

#[test]
fn post_get() {
    let mut pbcli = Command::cargo_bin("pbcli").unwrap();
    const POST_CONTENT: &str = "integration test";

    let post_result = pbcli
        .arg("--host")
        .arg(INSTANCE)
        .write_stdin(POST_CONTENT)
        .assert()
        .success()
        .stdout(predicate::str::contains(INSTANCE));

    let url = post_result.get_output().stdout.as_slice();

    pbcli.arg(std::str::from_utf8(url).unwrap());
    pbcli
        .assert()
        .success()
        .stdout(predicate::str::is_match(format!("^{}$", POST_CONTENT)).unwrap());
}

#[test]
fn file_size_limit_pass() {
    let mut pbcli = Command::cargo_bin("pbcli").unwrap();
    const POST_CONTENT: &str = "integration test";

    pbcli
        .arg("--host")
        .arg(INSTANCE)
        .arg("--size-limit")
        .arg("1000b")
        .write_stdin(POST_CONTENT)
        .assert()
        .success()
        .stdout(predicate::str::contains(INSTANCE));
}

#[test]
fn file_size_limit_fail() {
    let mut pbcli = Command::cargo_bin("pbcli").unwrap();
    const POST_CONTENT: &str = "integration test";

    pbcli
        .arg("--host")
        .arg(INSTANCE)
        .arg("--size-limit")
        .arg("10b")
        .write_stdin(POST_CONTENT)
        .assert()
        .failure();
}

#[test]
fn burn() {
    let mut pbcli = Command::cargo_bin("pbcli").unwrap();
    const POST_CONTENT: &str = "integration test";

    let post_result = pbcli
        .arg("--host")
        .arg(INSTANCE)
        .arg("--burn")
        .assert()
        .success();

    // TODO: Verify that paste has burn
}

#[test]
fn discussion() {
    let mut pbcli = Command::cargo_bin("pbcli").unwrap();
    const POST_CONTENT: &str = "integration test";

    let post_result = pbcli
        .arg("--host")
        .arg(INSTANCE)
        .arg("--discussion")
        .assert()
        .success();

    // TODO: Verify that paste has burn
}

extern crate assert_cli;
extern crate tempdir;

use assert_cli::Assert;

fn cmd_diff() -> Assert {
    Assert::main_binary().with_args(&["review-deps", "diff"])
}

#[test]
fn diff_shows_diff() {
    cmd_diff()
        .with_args(&["rand:0.6.0", "rand:0.6.1"])
        .stdout()
        .contains("< version = \"0.6.0\"")
        .unwrap();
}

#[test]
fn diff_reports_error_for_invalid_package_id() {
    cmd_diff()
        .with_args(&["rand:0.6.0", "rand-0.6.1"])
        .fails_with(101)
        .stderr()
        .contains("error: invalid package specification: \"rand-0.6.1\"; expected \"name:x.y.z\"")
        .unwrap();
}

#[test]
fn diff_copies_sources_to_dest() {
    let dir = tempdir::TempDir::new("diff-tests").unwrap();
    cmd_diff()
        .with_args(&["rand:0.6.0", "rand:0.6.1", "--destination"])
        .with_args(&[dir.path()])
        .stdout().is("")
        .unwrap();
    assert!(dir.path().join("rand:0.6.0").exists());
    assert!(dir.path().join("rand:0.6.1").exists());
}

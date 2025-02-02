use crate::helpers::constants::MULTICALL_CONFIGS_DIR;
use crate::helpers::fixtures::default_cli_args;
use crate::helpers::runner::runner;
use indoc::indoc;
use shared::test_utils::output_assert::{assert_stderr_contains, AsOutput};
use std::path::Path;

#[tokio::test]
async fn test_happy_case() {
    let mut args = default_cli_args();
    args.append(&mut vec!["--account", "user2"]);

    let path = project_root::get_project_root().expect("failed to get project root path");
    let path = Path::new(&path)
        .join(MULTICALL_CONFIGS_DIR)
        .join("deploy_invoke.toml");
    let path_str = path.to_str().expect("failed converting path to str");

    args.append(&mut vec!["multicall", "run", "--path", path_str]);

    let snapbox = runner(&args);
    let output = snapbox.assert();

    let stderr_str = output.as_stderr();
    assert!(
        stderr_str.is_empty(),
        "Multicall error, stderr: \n{stderr_str}",
    );

    output.stdout_matches(indoc! {r"
        command: multicall run
        transaction_hash: 0x[..]
    "});
}

#[tokio::test]
async fn test_calldata_ids() {
    let mut args = default_cli_args();
    args.append(&mut vec!["--account", "user5"]);

    let path = project_root::get_project_root().expect("failed to get project root path");
    let path = Path::new(&path)
        .join(MULTICALL_CONFIGS_DIR)
        .join("deploy_invoke_calldata_ids.toml");
    let path_str = path.to_str().expect("failed converting path to str");

    args.append(&mut vec!["multicall", "run", "--path", path_str]);

    let snapbox = runner(&args);
    let output = snapbox.assert();

    let stderr_str = output.as_stderr();
    assert!(
        stderr_str.is_empty(),
        "Multicall error, stderr: \n{stderr_str}",
    );

    output.stdout_matches(indoc! {r"
        command: multicall run
        transaction_hash: 0x[..]
    "});
}

#[tokio::test]
async fn test_invalid_path() {
    let mut args = default_cli_args();
    args.append(&mut vec!["--account", "user2"]);

    args.append(&mut vec!["multicall", "run", "--path", "non-existent"]);

    let snapbox = runner(&args);
    let output = snapbox.assert().success();

    assert!(output.as_stdout().is_empty());
    assert_stderr_contains(
        output,
        indoc! {r"
        command: multicall run
        error: No such file or directory [..]
        "},
    );
}

#[tokio::test]
async fn test_deploy_fail() {
    let mut args = default_cli_args();
    args.append(&mut vec!["--account", "user2"]);

    let path = project_root::get_project_root().expect("failed to get project root path");
    let path = Path::new(&path)
        .join(MULTICALL_CONFIGS_DIR)
        .join("deploy_invalid.toml");
    let path_str = path.to_str().expect("failed converting path to str");

    args.append(&mut vec!["multicall", "run", "--path", path_str]);

    let snapbox = runner(&args);
    let output = snapbox.assert().success();

    assert_stderr_contains(
        output,
        indoc! {r"
        command: multicall run
        error: An error occurred in the called contract [..]
        "},
    );
}

#[tokio::test]
async fn test_invoke_fail() {
    let mut args = default_cli_args();
    args.append(&mut vec!["--account", "user2"]);

    let path = project_root::get_project_root().expect("failed to get project root path");
    let path = Path::new(&path)
        .join(MULTICALL_CONFIGS_DIR)
        .join("invoke_invalid.toml");
    let path_str = path.to_str().expect("failed converting path to str");

    args.append(&mut vec!["multicall", "run", "--path", path_str]);

    let snapbox = runner(&args);
    let output = snapbox.assert().success();

    assert_stderr_contains(
        output,
        indoc! {r"
        command: multicall run
        error: An error occurred in the called contract [..]
        "},
    );
}

#[tokio::test]
async fn test_deploy_success_invoke_fails() {
    let mut args = default_cli_args();
    args.append(&mut vec!["--account", "user3"]);

    let path = project_root::get_project_root().expect("failed to get project root path");
    let path = Path::new(&path)
        .join(MULTICALL_CONFIGS_DIR)
        .join("deploy_succ_invoke_fail.toml");
    let path_str = path.to_str().expect("failed converting path to str");

    args.append(&mut vec!["multicall", "run", "--path", path_str]);

    let snapbox = runner(&args);

    let output = snapbox.assert().success();
    assert_stderr_contains(
        output,
        indoc! {r"
        command: multicall run
        error: An error occurred in the called contract [..]
        "},
    );
}

use crate::helpers::constants::{ACCOUNT, ACCOUNT_FILE_PATH, CONTRACTS_DIR, URL};
use crate::helpers::fixtures::{
    duplicate_contract_directory_with_salt, from_env, get_accounts_path, get_keystores_path,
};
use crate::helpers::runner::runner;
use indoc::indoc;
use shared::test_utils::output_assert::assert_stderr_contains;
use sncast::helpers::configuration::copy_config_to_tempdir;
use sncast::helpers::constants::KEYSTORE_PASSWORD_ENV_VAR;
use std::env;

#[tokio::test]
async fn test_happy_case_from_sncast_config() {
    let tempdir = copy_config_to_tempdir("tests/data/files/correct_snfoundry.toml", None);
    let args = vec![
        "--accounts-file",
        ACCOUNT_FILE_PATH,
        "call",
        "--contract-address",
        "0x0",
        "--function",
        "doesnotmatter",
    ];

    let snapbox = runner(&args).current_dir(tempdir.path());
    let output = snapbox.assert().success();

    assert_stderr_contains(
        output,
        indoc! {r"
        command: call
        error: There is no contract at the specified address
        "},
    );
}

#[tokio::test]
async fn test_happy_case_from_cli_no_scarb() {
    let args = vec![
        "--accounts-file",
        ACCOUNT_FILE_PATH,
        "--url",
        URL,
        "--account",
        ACCOUNT,
        "call",
        "--contract-address",
        "0x0",
        "--function",
        "doesnotmatter",
    ];

    let snapbox = runner(&args);
    let output = snapbox.assert().success();

    assert_stderr_contains(
        output,
        indoc! {r"
        command: call
        error: There is no contract at the specified address
        "},
    );
}

#[tokio::test]
async fn test_happy_case_from_cli_with_sncast_config() {
    let tempdir = copy_config_to_tempdir("tests/data/files/correct_snfoundry.toml", None);
    let address = from_env("CAST_MAP_ADDRESS").unwrap();
    let args = vec![
        "--accounts-file",
        ACCOUNT_FILE_PATH,
        "--profile",
        "default",
        "--url",
        URL,
        "--account",
        ACCOUNT,
        "call",
        "--contract-address",
        &address,
        "--function",
        "get",
        "--calldata",
        "0x0",
        "--block-id",
        "latest",
    ];

    let snapbox = runner(&args).current_dir(tempdir.path());

    snapbox.assert().success().stdout_eq(indoc! {r"
        command: call
        response: [0x0]
    "});
}

#[tokio::test]
async fn test_happy_case_mixed() {
    let tempdir = copy_config_to_tempdir("tests/data/files/correct_snfoundry.toml", None);
    let address = from_env("CAST_MAP_ADDRESS").unwrap();
    let args = vec![
        "--accounts-file",
        ACCOUNT_FILE_PATH,
        "--account",
        ACCOUNT,
        "call",
        "--contract-address",
        &address,
        "--function",
        "get",
        "--calldata",
        "0x0",
        "--block-id",
        "latest",
    ];

    let snapbox = runner(&args).current_dir(tempdir.path());

    snapbox.assert().success().stdout_eq(indoc! {r"
        command: call
        response: [0x0]
    "});
}

#[tokio::test]
async fn test_nonexistent_account_address() {
    let contract_path =
        duplicate_contract_directory_with_salt(CONTRACTS_DIR.to_string() + "/map", "dummy", "101");
    let accounts_json_path = get_accounts_path("tests/data/accounts/faulty_accounts.json");
    let args = vec![
        "--accounts-file",
        accounts_json_path.as_str(),
        "--account",
        "with_nonexistent_address",
        "--url",
        URL,
        "declare",
        "--contract-name",
        "Map",
    ];

    let snapbox = runner(&args).current_dir(contract_path.path());
    let output = snapbox.assert().failure();

    assert_stderr_contains(output, "Error: Invalid account address");
}

#[tokio::test]
async fn test_missing_account_flag() {
    let args = vec![
        "--accounts-file",
        ACCOUNT_FILE_PATH,
        "--url",
        URL,
        "declare",
        "--contract-name",
        "whatever",
    ];

    let snapbox = runner(&args);
    let output = snapbox.assert().failure();

    assert_stderr_contains(
        output,
        "Error: Account name not passed nor found in snfoundry.toml",
    );
}

#[tokio::test]
async fn test_missing_url() {
    let args = vec![
        "--accounts-file",
        ACCOUNT_FILE_PATH,
        "--account",
        ACCOUNT,
        "declare",
        "--contract-name",
        "whatever",
    ];

    let snapbox = runner(&args);
    let output = snapbox.assert().failure();

    assert_stderr_contains(
        output,
        "Error: RPC url not passed nor found in snfoundry.toml",
    );
}

#[tokio::test]
async fn test_inexistent_keystore() {
    let args = vec![
        "--url",
        URL,
        "--keystore",
        "inexistent_key.json",
        "declare",
        "--contract-name",
        "my_contract",
    ];

    let snapbox = runner(&args);

    let output = snapbox.assert().failure();
    assert_stderr_contains(output, "Error: Failed to find keystore file");
}

#[tokio::test]
async fn test_keystore_account_required() {
    let args = vec![
        "--url",
        URL,
        "--keystore",
        "tests/data/keystore/my_key.json",
        "declare",
        "--contract-name",
        "my_contract",
    ];

    let snapbox = runner(&args);
    let output = snapbox.assert().failure();

    assert_stderr_contains(output, "Error: Passed empty path for `--account`");
}

#[tokio::test]
async fn test_keystore_inexistent_account() {
    let args = vec![
        "--url",
        URL,
        "--keystore",
        "tests/data/keystore/my_key.json",
        "--account",
        "inexistent_account",
        "declare",
        "--contract-name",
        "my_contract",
    ];

    let snapbox = runner(&args);
    let output = snapbox.assert().failure();

    assert_stderr_contains(
        output,
        "Error: File containing the account does not exist[..]",
    );
}

#[tokio::test]
async fn test_keystore_undeployed_account() {
    let contract_path =
        duplicate_contract_directory_with_salt(CONTRACTS_DIR.to_string() + "/map", "put", "8");
    let my_key_path = get_keystores_path("tests/data/keystore/my_key.json");
    let my_account_undeployed_path =
        get_keystores_path("tests/data/keystore/my_account_undeployed.json");

    let args = vec![
        "--url",
        URL,
        "--keystore",
        my_key_path.as_str(),
        "--account",
        my_account_undeployed_path.as_str(),
        "declare",
        "--contract-name",
        "Map",
    ];

    env::set_var(KEYSTORE_PASSWORD_ENV_VAR, "123");
    let snapbox = runner(&args).current_dir(contract_path.path());
    let output = snapbox.assert().failure();

    assert_stderr_contains(output, "Error: [..] make sure the account is deployed");
}

#[tokio::test]
async fn test_keystore_declare() {
    let contract_path =
        duplicate_contract_directory_with_salt(CONTRACTS_DIR.to_string() + "/map", "put", "999");
    let my_key_path = get_keystores_path("tests/data/keystore/predeployed_key.json");
    let my_account_path = get_keystores_path("tests/data/keystore/predeployed_account.json");
    let args = vec![
        "--url",
        URL,
        "--keystore",
        my_key_path.as_str(),
        "--account",
        my_account_path.as_str(),
        "declare",
        "--contract-name",
        "Map",
    ];

    env::set_var(KEYSTORE_PASSWORD_ENV_VAR, "123");
    let snapbox = runner(&args).current_dir(contract_path.path());

    assert!(snapbox.assert().success().get_output().stderr.is_empty());
}

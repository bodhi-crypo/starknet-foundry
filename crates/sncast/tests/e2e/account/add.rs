use crate::helpers::constants::{
    DEVNET_OZ_CLASS_HASH, DEVNET_OZ_CLASS_HASH_CAIRO_1, DEVNET_PREDEPLOYED_ACCOUNT_ADDRESS, URL,
};
use crate::helpers::runner::runner;
use camino::Utf8PathBuf;
use indoc::{formatdoc, indoc};
use serde_json::json;
use shared::test_utils::output_assert::assert_stderr_contains;
use std::fs;
use tempfile::tempdir;

#[tokio::test]
pub async fn test_happy_case() {
    let tempdir = tempdir().expect("Unable to create a temporary directory");
    let accounts_file = "accounts.json";

    let args = vec![
        "--url",
        URL,
        "--accounts-file",
        accounts_file,
        "account",
        "add",
        "--name",
        "my_account_add",
        "--address",
        "0x123",
        "--private-key",
        "0x456",
    ];

    let snapbox = runner(&args).current_dir(tempdir.path());

    snapbox.assert().stdout_matches(indoc! {r"
        command: account add
        add_profile: --add-profile flag was not set. No profile added to snfoundry.toml
    "});

    let contents = fs::read_to_string(tempdir.path().join(accounts_file))
        .expect("Unable to read created file");
    let contents_json: serde_json::Value = serde_json::from_str(&contents).unwrap();
    assert_eq!(
        contents_json,
        json!(
            {
                "alpha-goerli": {
                  "my_account_add": {
                    "address": "0x123",
                    "deployed": false,
                    "private_key": "0x456",
                    "public_key": "0x5f679dacd8278105bd3b84a15548fe84079068276b0e84d6cc093eb5430f063"
                  }
                }
            }
        )
    );
}

#[tokio::test]
pub async fn test_existent_account_address() {
    let tempdir = tempdir().expect("Unable to create a temporary directory");
    let accounts_file = "accounts.json";

    let args = vec![
        "--url",
        URL,
        "--accounts-file",
        accounts_file,
        "account",
        "add",
        "--name",
        "my_account_add",
        "--address",
        DEVNET_PREDEPLOYED_ACCOUNT_ADDRESS,
        "--private-key",
        "0x456",
    ];

    runner(&args).current_dir(tempdir.path()).assert();

    let contents = fs::read_to_string(tempdir.path().join(accounts_file))
        .expect("Unable to read created file");
    let contents_json: serde_json::Value = serde_json::from_str(&contents).unwrap();
    assert_eq!(
        contents_json,
        json!(
            {
                "alpha-goerli": {
                  "my_account_add": {
                    "address": DEVNET_PREDEPLOYED_ACCOUNT_ADDRESS,
                    "class_hash": DEVNET_OZ_CLASS_HASH,
                    "deployed": true,
                    "private_key": "0x456",
                    "public_key": "0x5f679dacd8278105bd3b84a15548fe84079068276b0e84d6cc093eb5430f063"
                  }
                }
            }
        )
    );
}

#[tokio::test]
pub async fn test_existent_account_address_and_incorrect_class_hash() {
    let tempdir = tempdir().expect("Unable to create a temporary directory");
    let accounts_file = "accounts.json";

    let args = vec![
        "--url",
        URL,
        "--accounts-file",
        accounts_file,
        "account",
        "add",
        "--name",
        "my_account_add",
        "--address",
        DEVNET_PREDEPLOYED_ACCOUNT_ADDRESS,
        "--private-key",
        "0x456",
        "--class-hash",
        DEVNET_OZ_CLASS_HASH_CAIRO_1,
    ];

    let snapbox = runner(&args).current_dir(tempdir.path());

    snapbox.assert().stderr_matches(formatdoc! {r"
        command: account add
        error: Incorrect class hash {} for account address {}
    ", DEVNET_OZ_CLASS_HASH_CAIRO_1, DEVNET_PREDEPLOYED_ACCOUNT_ADDRESS});
}

#[tokio::test]
pub async fn test_nonexistent_account_address_and_nonexistent_class_hash() {
    let tempdir = tempdir().expect("Unable to create a temporary directory");
    let accounts_file = "accounts.json";

    let args = vec![
        "--url",
        URL,
        "--accounts-file",
        accounts_file,
        "account",
        "add",
        "--name",
        "my_account_add",
        "--address",
        "0x202",
        "--private-key",
        "0x456",
        "--class-hash",
        "0x101",
    ];

    let snapbox = runner(&args).current_dir(tempdir.path());

    snapbox.assert().stderr_matches(indoc! {r"
        command: account add
        error: Class with hash 0x101 is not declared, try using --class-hash with a hash of the declared class
    "});
}

#[tokio::test]
pub async fn test_happy_case_add_profile() {
    let tempdir = tempdir().expect("Failed to create a temporary directory");
    let accounts_file = "accounts.json";

    let args = vec![
        "--url",
        URL,
        "--accounts-file",
        accounts_file,
        "account",
        "add",
        "--name",
        "my_account_add",
        "--address",
        "0x1",
        "--private-key",
        "0x2",
        "--public-key",
        "0x759ca09377679ecd535a81e83039658bf40959283187c654c5416f439403cf5",
        "--salt",
        "0x3",
        "--class-hash",
        DEVNET_OZ_CLASS_HASH,
        "--add-profile",
        "my_account_add",
    ];

    let snapbox = runner(&args).current_dir(tempdir.path());

    snapbox.assert().stdout_matches(indoc! {r"
        command: account add
        add_profile: Profile my_account_add successfully added to snfoundry.toml
    "});
    let current_dir_utf8 = Utf8PathBuf::try_from(tempdir.path().to_path_buf()).unwrap();

    let contents = fs::read_to_string(current_dir_utf8.join(accounts_file))
        .expect("Unable to read created file");
    let contents_json: serde_json::Value = serde_json::from_str(&contents).unwrap();
    assert_eq!(
        contents_json,
        json!(
            {
                "alpha-goerli": {
                  "my_account_add": {
                    "address": "0x1",
                    "class_hash": DEVNET_OZ_CLASS_HASH,
                    "deployed": false,
                    "private_key": "0x2",
                    "public_key": "0x759ca09377679ecd535a81e83039658bf40959283187c654c5416f439403cf5",
                    "salt": "0x3",
                  }
                }
            }
        )
    );

    let contents = fs::read_to_string(current_dir_utf8.join("snfoundry.toml"))
        .expect("Unable to read snfoundry.toml");
    assert!(contents.contains("[sncast.my_account_add]"));
    assert!(contents.contains("account = \"my_account_add\""));
}

#[tokio::test]
pub async fn test_detect_deployed() {
    let tempdir = tempdir().expect("Unable to create a temporary directory");
    let accounts_file = "accounts.json";

    let args = vec![
        "--url",
        URL,
        "--accounts-file",
        accounts_file,
        "account",
        "add",
        "--name",
        "my_account_add",
        "--address",
        DEVNET_PREDEPLOYED_ACCOUNT_ADDRESS,
        "--private-key",
        "0x5",
    ];

    let snapbox = runner(&args).current_dir(tempdir.path());

    snapbox.assert().stdout_matches(indoc! {r"
        command: account add
        add_profile: --add-profile flag was not set. No profile added to snfoundry.toml
    "});

    let contents = fs::read_to_string(tempdir.path().join(accounts_file))
        .expect("Unable to read created file");
    let contents_json: serde_json::Value = serde_json::from_str(&contents).unwrap();
    assert_eq!(
        contents_json,
        json!(
            {
                "alpha-goerli": {
                  "my_account_add": {
                    "address": DEVNET_PREDEPLOYED_ACCOUNT_ADDRESS,
                    "class_hash": DEVNET_OZ_CLASS_HASH,
                    "deployed": true,
                    "private_key": "0x5",
                    "public_key": "0x788435d61046d3eec54d77d25bd194525f4fa26ebe6575536bc6f656656b74c"
                  }
                }
            }
        )
    );
}

#[tokio::test]
pub async fn test_invalid_public_key() {
    let args = vec![
        "--url",
        URL,
        "account",
        "add",
        "--name",
        "my_account_add",
        "--address",
        "0x123",
        "--private-key",
        "0x456",
        "--public-key",
        "0x457",
    ];

    let snapbox = runner(&args);
    let output = snapbox.assert().success();

    assert_stderr_contains(
        output,
        indoc! {r"
        command: account add
        error: The private key does not match the public key
        "},
    );
}

#[tokio::test]
pub async fn test_missing_arguments() {
    let args = vec!["--url", URL, "account", "add", "--name", "my_account_add"];

    let snapbox = runner(&args);
    let output = snapbox.assert().failure();

    assert_stderr_contains(
        output,
        indoc! {r"
        error: the following required arguments were not provided:
          --address <ADDRESS>
          <--private-key <PRIVATE_KEY>|--private-key-file <PRIVATE_KEY_FILE_PATH>>
        "},
    );
}

#[tokio::test]
pub async fn test_private_key_from_file() {
    let temp_dir = tempdir().expect("Unable to create a temporary directory");
    let accounts_file = "accounts.json";
    let private_key_file = "my_private_key";

    fs::write(temp_dir.path().join(private_key_file), "0x456").unwrap();

    let args = vec![
        "--url",
        URL,
        "--accounts-file",
        accounts_file,
        "account",
        "add",
        "--name",
        "my_account_add",
        "--address",
        "0x123",
        "--private-key-file",
        private_key_file,
    ];

    let snapbox = runner(&args).current_dir(temp_dir.path());

    snapbox.assert().stdout_matches(indoc! {r"
        command: account add
        add_profile: --add-profile flag was not set. No profile added to snfoundry.toml
    "});

    let contents = fs::read_to_string(temp_dir.path().join(accounts_file))
        .expect("Unable to read created file");
    let contents_json: serde_json::Value = serde_json::from_str(&contents).unwrap();
    assert_eq!(
        contents_json,
        json!(
            {
                "alpha-goerli": {
                  "my_account_add": {
                    "address": "0x123",
                    "deployed": false,
                    "private_key": "0x456",
                    "public_key": "0x5f679dacd8278105bd3b84a15548fe84079068276b0e84d6cc093eb5430f063"
                  }
                }
            }
        )
    );
}

#[tokio::test]
pub async fn test_accept_only_one_private_key() {
    let args = vec![
        "account",
        "add",
        "--name",
        "my_account_add",
        "--address",
        "0x123",
        "--private-key",
        "0x456",
        "--private-key-file",
        "my_private_key",
    ];

    let snapbox = runner(&args);
    let output = snapbox.assert().failure();

    assert_stderr_contains(
        output,
        "error: the argument '--private-key <PRIVATE_KEY>' cannot be used with '--private-key-file <PRIVATE_KEY_FILE_PATH>'"
    );
}

#[tokio::test]
pub async fn test_invalid_private_key_file_path() {
    let args = vec![
        "--url",
        URL,
        "account",
        "add",
        "--name",
        "my_account_add",
        "--address",
        "0x123",
        "--private-key-file",
        "my_private_key",
    ];

    let snapbox = runner(&args);
    let output = snapbox.assert().success();

    assert_stderr_contains(
        output,
        indoc! {r"
        command: account add
        error: Failed to obtain private key from the file my_private_key: No such file or directory (os error 2)
        "},
    );
}

#[tokio::test]
pub async fn test_invalid_private_key_in_file() {
    let temp_dir = tempdir().expect("Unable to create a temporary directory");
    let private_key_file = "my_private_key";

    fs::write(
        temp_dir.path().join(private_key_file),
        "invalid private key",
    )
    .unwrap();

    let args = vec![
        "--url",
        URL,
        "--accounts-file",
        "accounts.json",
        "account",
        "add",
        "--name",
        "my_account_add",
        "--address",
        "0x123",
        "--private-key-file",
        private_key_file,
    ];

    let snapbox = runner(&args).current_dir(temp_dir.path());
    let output = snapbox.assert().success();

    assert_stderr_contains(
        output,
        indoc! {r"
        command: account add
        error: Failed to obtain private key from the file my_private_key: invalid character
        "},
    );
}

#[tokio::test]
pub async fn test_private_key_as_int_in_file() {
    let temp_dir = tempdir().expect("Unable to create a temporary directory");
    let accounts_file = "accounts.json";
    let private_key_file = "my_private_key";

    fs::write(temp_dir.path().join(private_key_file), "1110").unwrap();

    let args = vec![
        "--url",
        URL,
        "--accounts-file",
        accounts_file,
        "account",
        "add",
        "--name",
        "my_account_add",
        "--address",
        "0x123",
        "--private-key-file",
        private_key_file,
    ];

    runner(&args)
        .current_dir(temp_dir.path())
        .assert()
        .success();

    let contents = fs::read_to_string(temp_dir.path().join(accounts_file))
        .expect("Unable to read created file");
    let contents_json: serde_json::Value = serde_json::from_str(&contents).unwrap();
    assert_eq!(
        contents_json,
        json!(
            {
                "alpha-goerli": {
                  "my_account_add": {
                    "address": "0x123",
                    "deployed": false,
                    "private_key": "0x456",
                    "public_key": "0x5f679dacd8278105bd3b84a15548fe84079068276b0e84d6cc093eb5430f063"
                  }
                }
            }
        )
    );
}

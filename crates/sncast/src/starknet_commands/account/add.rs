use crate::starknet_commands::account::{
    add_created_profile_to_configuration, prepare_account_json, write_account_to_accounts_file,
};
use anyhow::{ensure, Context, Result};
use camino::Utf8PathBuf;
use clap::Args;
use sncast::handle_rpc_error;
use sncast::helpers::configuration::CastConfig;
use sncast::response::structs::AccountAddResponse;
use sncast::{check_class_hash_exists, get_chain_id, parse_number};
use starknet::core::types::BlockTag::Pending;
use starknet::core::types::{BlockId, FieldElement, StarknetError};
use starknet::providers::{
    jsonrpc::{HttpTransport, JsonRpcClient},
    Provider, ProviderError,
};
use starknet::signers::SigningKey;

#[derive(Args, Debug)]
#[command(about = "Add an account to the accounts file")]
pub struct Add {
    /// Name of the account to be added
    #[clap(short, long)]
    pub name: String,

    /// Address of the account
    #[clap(short, long, requires = "private_key_input")]
    pub address: FieldElement,

    /// Class hash of the account
    #[clap(short, long)]
    pub class_hash: Option<FieldElement>,

    /// Account private key
    #[clap(long, group = "private_key_input")]
    pub private_key: Option<FieldElement>,

    /// Path to the file holding account private key
    #[clap(long = "private-key-file", group = "private_key_input")]
    pub private_key_file_path: Option<Utf8PathBuf>,

    /// Account public key
    #[clap(long)]
    pub public_key: Option<FieldElement>,

    /// Salt for the address
    #[clap(short, long)]
    pub salt: Option<FieldElement>,

    /// If passed, a profile with the provided name and corresponding data will be created in snfoundry.toml
    #[allow(clippy::struct_field_names)]
    #[clap(long)]
    pub add_profile: Option<String>,
}

pub async fn add(
    rpc_url: &str,
    account: &str,
    accounts_file: &Utf8PathBuf,
    provider: &JsonRpcClient<HttpTransport>,
    add: &Add,
) -> Result<AccountAddResponse> {
    let private_key = match &add.private_key_file_path {
        Some(file_path) => get_private_key_from_file(file_path)
            .with_context(|| format!("Failed to obtain private key from the file {file_path}"))?,
        None => add
            .private_key
            .expect("Failed to parse provided private key"),
    };
    let private_key = &SigningKey::from_secret_scalar(private_key);
    if let Some(public_key) = &add.public_key {
        ensure!(
            public_key == &private_key.verifying_key().scalar(),
            "The private key does not match the public key"
        );
    }

    let fetched_class_hash = get_class_hash_by_address(provider, add.address).await?;
    let is_deployed = fetched_class_hash.is_some();
    let class_hash = match (fetched_class_hash, add.class_hash) {
        (Some(from_provider), Some(from_user)) => {
            ensure!(
                from_provider == from_user,
                "Incorrect class hash {:#x} for account address {:#x}",
                from_user,
                add.address
            );
            fetched_class_hash
        }
        (None, Some(from_user)) => {
            check_class_hash_exists(provider, from_user).await?;
            Some(from_user)
        }
        _ => fetched_class_hash,
    };

    let account_json =
        prepare_account_json(private_key, add.address, is_deployed, class_hash, add.salt);

    let chain_id = get_chain_id(provider).await?;
    write_account_to_accounts_file(account, accounts_file, chain_id, account_json.clone())?;

    if add.add_profile.is_some() {
        let config = CastConfig {
            rpc_url: rpc_url.into(),
            account: account.into(),
            accounts_file: accounts_file.into(),
            ..Default::default()
        };
        add_created_profile_to_configuration(&add.add_profile, &config, &None)?;
    }

    Ok(AccountAddResponse {
        add_profile: if add.add_profile.is_some() {
            format!(
                "Profile {} successfully added to snfoundry.toml",
                add.add_profile.clone().expect("Failed to get profile name")
            )
        } else {
            "--add-profile flag was not set. No profile added to snfoundry.toml".to_string()
        },
    })
}

fn get_private_key_from_file(file_path: &Utf8PathBuf) -> Result<FieldElement> {
    let private_key_string = std::fs::read_to_string(file_path.clone())?;
    parse_number(&private_key_string)
}

async fn get_class_hash_by_address(
    provider: &JsonRpcClient<HttpTransport>,
    address: FieldElement,
) -> Result<Option<FieldElement>> {
    match provider
        .get_class_hash_at(BlockId::Tag(Pending), address)
        .await
    {
        Ok(class_hash) => Ok(Some(class_hash)),
        Err(ProviderError::StarknetError(StarknetError::ContractNotFound)) => Ok(None),
        Err(err) => Err(handle_rpc_error(err)),
    }
}

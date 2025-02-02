use std::collections::HashMap;
use std::sync::Arc;

use blockifier::execution::contract_class::{ContractClassV1, ContractClassV1Inner};

use blockifier::execution::contract_class::ContractClass;
use cairo_vm::types::program::Program;

use blockifier::execution::entry_point::{CallEntryPoint, CallType};
use conversions::IntoConv;
use starknet::core::utils::get_selector_from_name;
use starknet_api::deprecated_contract_class::EntryPointType;

use runtime::starknet::context::ERC20_CONTRACT_ADDRESS;
use runtime::starknet::state::DictStateReader;
use starknet_api::{
    class_hash, contract_address,
    core::{ClassHash, ContractAddress, PatriciaKey},
    hash::StarkHash,
    patricia_key,
    transaction::Calldata,
};

pub const MAX_FEE: u128 = 1_000_000 * 100_000_000_000; // 1000000 * min_gas_price.
pub const INITIAL_BALANCE: u128 = 10 * MAX_FEE;

// Mocked class hashes, those are not checked anywhere
pub const TEST_CLASS_HASH: &str = "0x110";
pub const TEST_ACCOUNT_CONTRACT_CLASS_HASH: &str = "0x111";
pub const TEST_EMPTY_CONTRACT_CLASS_HASH: &str = "0x112";
pub const TEST_FAULTY_ACCOUNT_CONTRACT_CLASS_HASH: &str = "0x113";
pub const SECURITY_TEST_CLASS_HASH: &str = "0x114";
pub const TEST_ERC20_CONTRACT_CLASS_HASH: &str = "0x1010";

pub const TEST_CONTRACT_CLASS_HASH: &str = "0x117";
pub const TEST_ENTRY_POINT_SELECTOR: &str = "TEST_CONTRACT_SELECTOR";
// snforge_std/src/cheatcodes.cairo::test_address
pub const TEST_ADDRESS: &str = "0x01724987234973219347210837402";

fn contract_class_no_entrypoints() -> ContractClass {
    let inner = ContractClassV1Inner {
        program: Program::default(),
        entry_points_by_type: HashMap::from([
            (EntryPointType::External, vec![]),
            (EntryPointType::Constructor, vec![]),
            (EntryPointType::L1Handler, vec![]),
        ]),

        hints: HashMap::new(),
    };
    ContractClass::V1(ContractClassV1(Arc::new(inner)))
}

// Creates a state with predeployed account and erc20 used to send transactions during tests.
// Deployed contracts are cairo 0 contracts
// Account does not include validations
#[must_use]
pub fn build_testing_state() -> DictStateReader {
    let test_erc20_class_hash = class_hash!(TEST_ERC20_CONTRACT_CLASS_HASH);
    let test_contract_class_hash = class_hash!(TEST_CONTRACT_CLASS_HASH);

    let class_hash_to_class = HashMap::from([
        // This is dummy put here only to satisfy blockifier
        // this class is not used and the test contract cannot be called
        (test_contract_class_hash, contract_class_no_entrypoints()),
    ]);

    let test_erc20_address = contract_address!(ERC20_CONTRACT_ADDRESS);
    let test_address = contract_address!(TEST_ADDRESS);
    let address_to_class_hash = HashMap::from([
        (test_erc20_address, test_erc20_class_hash),
        (test_address, test_contract_class_hash),
    ]);

    DictStateReader {
        address_to_class_hash,
        class_hash_to_class,
        ..Default::default()
    }
}

#[must_use]
pub fn build_test_entry_point() -> CallEntryPoint {
    let test_selector = get_selector_from_name(TEST_ENTRY_POINT_SELECTOR).unwrap();
    let entry_point_selector = test_selector.into_();
    CallEntryPoint {
        class_hash: None,
        code_address: Some(contract_address!(TEST_ADDRESS)),
        entry_point_type: EntryPointType::External,
        entry_point_selector,
        calldata: Calldata(Arc::new(vec![])),
        storage_address: contract_address!(TEST_ADDRESS),
        caller_address: ContractAddress::default(),
        call_type: CallType::Call,
        initial_gas: u64::MAX,
    }
}

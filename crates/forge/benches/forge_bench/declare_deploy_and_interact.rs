use std::fs;
use std::path::Path;
use test_utils::{
    runner::{assert_passed, Contract, TestCase},
    running_tests::run_test_case,
    test_case,
};

pub fn setup() -> TestCase {
    test_case!(
        &fs::read_to_string("tests/data/contracts/benchmarks/test_declare_deploy_interact.cairo")
            .unwrap(),
        Contract::from_code_path(
            "HelloStarknet".to_string(),
            Path::new("tests/data/contracts/benchmarks/declare_deploy_interact.cairo")
        )
        .unwrap()
    )
}

pub fn declare_deploy_and_interact(test: &TestCase) {
    let result = run_test_case(test);

    assert_passed(&result);
}

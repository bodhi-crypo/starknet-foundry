use anyhow::{Context, Result};
use cairo_lang_casm::hints::Hint;
use cairo_lang_sierra::program::Program;
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Display;
use std::io::Write;
use std::path::Path;
use std::str::from_utf8;
use tempfile::Builder;

pub use command::*;

mod command;

#[derive(Serialize, Deserialize)]
pub struct AssembledProgramWithDebugInfo {
    pub assembled_cairo_program: AssembledCairoProgramWithSerde,
    pub debug_info: Vec<(usize, usize)>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AssembledCairoProgramWithSerde {
    pub bytecode: Vec<BigInt>,
    pub hints: Vec<(usize, Vec<Hint>)>,
}

pub fn compile_sierra_to_casm(sierra_program: &Program) -> Result<AssembledProgramWithDebugInfo> {
    let assembled_with_info_raw = compile_sierra(
        &serde_json::to_value(sierra_program).unwrap(),
        None,
        &SierraType::Raw,
    )?;
    let assembled_with_info: AssembledProgramWithDebugInfo =
        serde_json::from_str(&assembled_with_info_raw)?;

    Ok(assembled_with_info)
}

pub fn compile_sierra(
    sierra_contract_class: &Value,
    current_dir: Option<&Path>,
    sierra_type: &SierraType,
) -> Result<String> {
    let mut temp_sierra_file = Builder::new().tempfile().unwrap();
    let _ = temp_sierra_file
        .write(
            serde_json::to_vec(sierra_contract_class)
                .unwrap()
                .as_slice(),
        )
        .unwrap();

    compile_sierra_at_path(
        temp_sierra_file.path().to_str().unwrap(),
        current_dir,
        sierra_type,
    )
}

pub fn compile_sierra_at_path(
    sierra_file_path: &str,
    current_dir: Option<&Path>,
    sierra_type: &SierraType,
) -> Result<String> {
    let mut usc_command = UniversalSierraCompilerCommand::new();
    if let Some(dir) = current_dir {
        usc_command.current_dir(dir);
    }

    let usc_output = usc_command
        .inherit_stderr()
        .args(vec![
            &("compile-".to_string() + &sierra_type.to_string()),
            "--sierra-path",
            sierra_file_path,
        ])
        .command()
        .output()
        .context(
            "Error while compiling Sierra. \
            Make sure you have the latest universal-sierra-compiler binary installed. \
            Contact us if it doesn't help",
        )?;

    Ok(from_utf8(&usc_output.stdout).unwrap().to_string())
}

pub enum SierraType {
    Contract,
    Raw,
}

impl Display for SierraType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SierraType::Contract => "contract",
                SierraType::Raw => "raw",
            }
        )
    }
}

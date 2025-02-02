use crate::state::CheatnetState;
use blockifier::abi::constants;
use blockifier::execution::common_hints::HintExecutionResult;
use blockifier::execution::deprecated_syscalls::hint_processor::{
    DeprecatedSyscallExecutionError, DeprecatedSyscallHintProcessor,
};
use blockifier::execution::deprecated_syscalls::{
    CallContractRequest, DeployRequest, DeployResponse, DeprecatedSyscallResult,
    DeprecatedSyscallSelector, GetBlockNumberResponse, GetBlockTimestampResponse,
    GetContractAddressResponse, LibraryCallRequest, SyscallRequest, SyscallResponse,
    WriteResponseResult,
};
use blockifier::execution::entry_point::{CallEntryPoint, CallType, ConstructorContext};
use blockifier::execution::execution_utils::{
    execute_deployment, write_maybe_relocatable, ReadOnlySegment,
};
use conversions::FromConv;

use ::runtime::SyscallHandlingResult;
use cairo_felt::Felt252;
use cairo_vm::types::relocatable::{MaybeRelocatable, Relocatable};
use cairo_vm::vm::errors::hint_errors::HintError;
use cairo_vm::vm::vm_core::VirtualMachine;
use num_traits::ToPrimitive;
use starknet_api::block::{BlockNumber, BlockTimestamp};
use starknet_api::core::{
    calculate_contract_address, ClassHash, ContractAddress, EntryPointSelector,
};
use starknet_api::deprecated_contract_class::EntryPointType;
use starknet_api::transaction::Calldata;

use self::runtime::{
    DeprecatedExtendedRuntime, DeprecatedExtensionLogic, DeprecatedStarknetRuntime,
};

use super::call_to_blockifier_runtime_extension::execution::entry_point::execute_call_entry_point;
use super::call_to_blockifier_runtime_extension::execution::syscall_hooks;
use super::call_to_blockifier_runtime_extension::RuntimeState;

pub mod runtime;

#[derive(Debug)]
// crates/blockifier/src/execution/deprecated_syscalls/mod.rs:147 (SingleSegmentResponse)
// It is created here because fields in the original structure are private
// so we cannot create it in call_contract_syscall
pub struct SingleSegmentResponse {
    pub(crate) segment: ReadOnlySegment,
}
// crates/blockifier/src/execution/deprecated_syscalls/mod.rs:151 (SyscallResponse for SingleSegmentResponse)
impl SyscallResponse for SingleSegmentResponse {
    fn write(self, vm: &mut VirtualMachine, ptr: &mut Relocatable) -> WriteResponseResult {
        write_maybe_relocatable(vm, ptr, self.segment.length)?;
        write_maybe_relocatable(vm, ptr, self.segment.start_ptr)?;
        Ok(())
    }
}

pub struct DeprecatedCheatableStarknetRuntimeExtension<'a> {
    pub cheatnet_state: &'a mut CheatnetState,
}

pub type DeprecatedCheatableStarknetRuntime<'a> =
    DeprecatedExtendedRuntime<DeprecatedCheatableStarknetRuntimeExtension<'a>>;

impl<'a> DeprecatedExtensionLogic for DeprecatedCheatableStarknetRuntimeExtension<'a> {
    type Runtime = DeprecatedStarknetRuntime<'a>;

    fn override_system_call(
        &mut self,
        selector: DeprecatedSyscallSelector,
        vm: &mut VirtualMachine,
        extended_runtime: &mut Self::Runtime,
    ) -> Result<SyscallHandlingResult, HintError> {
        let syscall_handler = &mut extended_runtime.hint_handler;
        let contract_address = syscall_handler.storage_address;
        match selector {
            DeprecatedSyscallSelector::GetCallerAddress => {
                if self.cheatnet_state.address_is_pranked(&contract_address) {
                    // Increment, since the selector was peeked into before
                    syscall_handler.syscall_ptr += 1;
                    increment_syscall_count(syscall_handler, selector);

                    let response = get_caller_address(self, contract_address).unwrap();

                    response.write(vm, &mut syscall_handler.syscall_ptr)?;
                    Ok(SyscallHandlingResult::Handled(()))
                } else {
                    Ok(SyscallHandlingResult::Forwarded)
                }
            }
            DeprecatedSyscallSelector::GetBlockNumber => {
                if self.cheatnet_state.address_is_rolled(&contract_address) {
                    syscall_handler.syscall_ptr += 1;
                    increment_syscall_count(syscall_handler, selector);

                    let response = get_block_number(self, contract_address).unwrap();

                    response.write(vm, &mut syscall_handler.syscall_ptr)?;
                    Ok(SyscallHandlingResult::Handled(()))
                } else {
                    Ok(SyscallHandlingResult::Forwarded)
                }
            }
            DeprecatedSyscallSelector::GetBlockTimestamp => {
                if self.cheatnet_state.address_is_warped(&contract_address) {
                    syscall_handler.syscall_ptr += 1;
                    increment_syscall_count(syscall_handler, selector);

                    let response = get_block_timestamp(self, contract_address).unwrap();

                    response.write(vm, &mut syscall_handler.syscall_ptr)?;
                    Ok(SyscallHandlingResult::Handled(()))
                } else {
                    Ok(SyscallHandlingResult::Forwarded)
                }
            }
            DeprecatedSyscallSelector::GetSequencerAddress => {
                if self.cheatnet_state.address_is_elected(&contract_address) {
                    syscall_handler.syscall_ptr += 1;
                    increment_syscall_count(syscall_handler, selector);

                    let response =
                        get_sequencer_address(self, syscall_handler, contract_address).unwrap();

                    response.write(vm, &mut syscall_handler.syscall_ptr)?;

                    Ok(SyscallHandlingResult::Handled(()))
                } else {
                    Ok(SyscallHandlingResult::Forwarded)
                }
            }
            DeprecatedSyscallSelector::DelegateCall => {
                syscall_handler.syscall_ptr += 1;
                increment_syscall_count(syscall_handler, selector);

                self.execute_syscall(vm, delegate_call, syscall_handler)?;
                Ok(SyscallHandlingResult::Handled(()))
            }
            DeprecatedSyscallSelector::LibraryCall => {
                syscall_handler.syscall_ptr += 1;
                increment_syscall_count(syscall_handler, selector);

                self.execute_syscall(vm, library_call, syscall_handler)?;
                Ok(SyscallHandlingResult::Handled(()))
            }
            DeprecatedSyscallSelector::CallContract => {
                syscall_handler.syscall_ptr += 1;
                increment_syscall_count(syscall_handler, selector);

                self.execute_syscall(vm, call_contract, syscall_handler)?;
                Ok(SyscallHandlingResult::Handled(()))
            }
            DeprecatedSyscallSelector::Deploy => {
                syscall_handler.syscall_ptr += 1;
                increment_syscall_count(syscall_handler, selector);

                self.execute_syscall(vm, deploy, syscall_handler)?;
                Ok(SyscallHandlingResult::Handled(()))
            }
            _ => Ok(SyscallHandlingResult::Forwarded),
        }
    }

    fn post_syscall_hook(
        &mut self,
        selector: &DeprecatedSyscallSelector,
        extended_runtime: &mut Self::Runtime,
    ) {
        let syscall_handler = &extended_runtime.hint_handler;
        if let DeprecatedSyscallSelector::EmitEvent = selector {
            syscall_hooks::emit_event_hook(syscall_handler, self.cheatnet_state);
        }
    }
}

impl<'a> DeprecatedCheatableStarknetRuntimeExtension<'a> {
    // crates/blockifier/src/execution/deprecated_syscalls/hint_processor.rs:233
    fn execute_syscall<Request, Response, ExecuteCallback>(
        &mut self,
        vm: &mut VirtualMachine,
        execute_callback: ExecuteCallback,
        syscall_handler: &mut DeprecatedSyscallHintProcessor,
    ) -> HintExecutionResult
    where
        Request: SyscallRequest,
        Response: SyscallResponse,
        ExecuteCallback: FnOnce(
            Request,
            &mut VirtualMachine,
            &mut DeprecatedSyscallHintProcessor,
            &mut CheatnetState,
        ) -> DeprecatedSyscallResult<Response>,
    {
        let request = Request::read(vm, &mut syscall_handler.syscall_ptr)?;

        let response = execute_callback(request, vm, syscall_handler, self.cheatnet_state)?;
        response.write(vm, &mut syscall_handler.syscall_ptr)?;

        Ok(())
    }
}

// crates/blockifier/src/execution/deprecated_syscalls/hint_processor.rs:264
fn increment_syscall_count(
    syscall_handler: &mut DeprecatedSyscallHintProcessor,
    selector: DeprecatedSyscallSelector,
) {
    let syscall_count = syscall_handler
        .resources
        .syscall_counter
        .entry(selector)
        .or_default();
    *syscall_count += 1;
}

//blockifier/src/execution/deprecated_syscalls/mod.rs:303 (deploy)
pub fn deploy(
    request: DeployRequest,
    _vm: &mut VirtualMachine,
    syscall_handler: &mut DeprecatedSyscallHintProcessor<'_>,
    _cheatnet_state: &mut CheatnetState,
) -> DeprecatedSyscallResult<DeployResponse> {
    let deployer_address = syscall_handler.storage_address;
    let deployer_address_for_calculation = if request.deploy_from_zero {
        ContractAddress::default()
    } else {
        deployer_address
    };
    let deployed_contract_address = calculate_contract_address(
        request.contract_address_salt,
        request.class_hash,
        &request.constructor_calldata,
        deployer_address_for_calculation,
    )?;

    let ctor_context = ConstructorContext {
        class_hash: request.class_hash,
        code_address: Some(deployed_contract_address),
        storage_address: deployed_contract_address,
        caller_address: deployer_address,
    };
    let call_info = execute_deployment(
        syscall_handler.state,
        syscall_handler.resources,
        syscall_handler.context,
        ctor_context,
        request.constructor_calldata,
        constants::INITIAL_GAS_COST,
    )?;
    syscall_handler.inner_calls.push(call_info);

    Ok(DeployResponse {
        contract_address: deployed_contract_address,
    })
}

//blockifier/src/execution/deprecated_syscalls/mod.rs:182 (call_contract)
pub fn call_contract(
    request: CallContractRequest,
    vm: &mut VirtualMachine,
    syscall_handler: &mut DeprecatedSyscallHintProcessor<'_>,
    cheatnet_state: &mut CheatnetState,
) -> DeprecatedSyscallResult<SingleSegmentResponse> {
    let storage_address = request.contract_address;
    // Check that the call is legal if in Validate execution mode.
    if syscall_handler.is_validate_mode() && syscall_handler.storage_address != storage_address {
        return Err(
            DeprecatedSyscallExecutionError::InvalidSyscallInExecutionMode {
                syscall_name: "call_contract".to_string(),
                execution_mode: syscall_handler.execution_mode(),
            },
        );
    }
    let mut entry_point = CallEntryPoint {
        class_hash: None,
        code_address: Some(storage_address),
        entry_point_type: EntryPointType::External,
        entry_point_selector: request.function_selector,
        calldata: request.calldata,
        storage_address,
        caller_address: syscall_handler.storage_address,
        call_type: CallType::Call,
        initial_gas: constants::INITIAL_GAS_COST,
    };
    let retdata_segment =
        execute_inner_call(&mut entry_point, vm, syscall_handler, cheatnet_state)?;

    Ok(SingleSegmentResponse {
        segment: retdata_segment,
    })
}

// blockifier/src/execution/deprecated_syscalls/mod.rs:209 (delegate_call)
pub fn delegate_call(
    request: CallContractRequest,
    vm: &mut VirtualMachine,
    syscall_handler: &mut DeprecatedSyscallHintProcessor<'_>,
    cheatnet_state: &mut CheatnetState,
) -> DeprecatedSyscallResult<SingleSegmentResponse> {
    let call_to_external = true;
    let storage_address = request.contract_address;
    let class_hash = syscall_handler.state.get_class_hash_at(storage_address)?;
    let retdata_segment = execute_library_call(
        syscall_handler,
        cheatnet_state,
        vm,
        class_hash,
        Some(storage_address),
        call_to_external,
        request.function_selector,
        request.calldata,
    )?;

    Ok(SingleSegmentResponse {
        segment: retdata_segment,
    })
}

// blockifier/src/execution/deprecated_syscalls/mod.rs:537 (library_call)
pub fn library_call(
    request: LibraryCallRequest,
    vm: &mut VirtualMachine,
    syscall_handler: &mut DeprecatedSyscallHintProcessor<'_>,
    cheatnet_state: &mut CheatnetState,
) -> DeprecatedSyscallResult<SingleSegmentResponse> {
    let call_to_external = true;
    let retdata_segment = execute_library_call(
        syscall_handler,
        cheatnet_state,
        vm,
        request.class_hash,
        None,
        call_to_external,
        request.function_selector,
        request.calldata,
    )?;

    Ok(SingleSegmentResponse {
        segment: retdata_segment,
    })
}

// blockifier/src/execution/deprecated_syscalls/mod.rs:426 (get_caller_address)
pub fn get_caller_address(
    syscall_handler: &mut DeprecatedCheatableStarknetRuntimeExtension<'_>,
    contract_address: ContractAddress,
) -> DeprecatedSyscallResult<GetContractAddressResponse> {
    Ok(GetContractAddressResponse {
        address: syscall_handler
            .cheatnet_state
            .get_cheated_caller_address(&contract_address)
            .unwrap(),
    })
}

// blockifier/src/execution/deprecated_syscalls/mod.rs:387 (get_block_number)
pub fn get_block_number(
    syscall_handler: &mut DeprecatedCheatableStarknetRuntimeExtension<'_>,
    contract_address: ContractAddress,
) -> DeprecatedSyscallResult<GetBlockNumberResponse> {
    Ok(GetBlockNumberResponse {
        block_number: BlockNumber(
            syscall_handler
                .cheatnet_state
                .get_cheated_block_number(&contract_address)
                .unwrap()
                .to_u64()
                .unwrap(),
        ),
    })
}

// blockifier/src/execution/deprecated_syscalls/mod.rs:411 (get_block_timestamp)
pub fn get_block_timestamp(
    syscall_handler: &mut DeprecatedCheatableStarknetRuntimeExtension<'_>,
    contract_address: ContractAddress,
) -> DeprecatedSyscallResult<GetBlockTimestampResponse> {
    Ok(GetBlockTimestampResponse {
        block_timestamp: BlockTimestamp(
            syscall_handler
                .cheatnet_state
                .get_cheated_block_timestamp(&contract_address)
                .unwrap()
                .to_u64()
                .unwrap(),
        ),
    })
}

// blockifier/src/execution/deprecated_syscalls/mod.rs:470 (get_sequencer_address)
type GetSequencerAddressResponse = GetContractAddressResponse;

pub fn get_sequencer_address(
    cheatable_syscall_handler: &mut DeprecatedCheatableStarknetRuntimeExtension<'_>,
    syscall_handler: &mut DeprecatedSyscallHintProcessor<'_>,
    contract_address: ContractAddress,
) -> DeprecatedSyscallResult<GetSequencerAddressResponse> {
    syscall_handler.verify_not_in_validate_mode("get_sequencer_address")?;

    Ok(GetSequencerAddressResponse {
        address: cheatable_syscall_handler
            .cheatnet_state
            .get_cheated_sequencer_address(&contract_address)
            .unwrap(),
    })
}

// blockifier/src/execution/deprecated_syscalls/hint_processor.rs:393 (execute_inner_call)
pub fn execute_inner_call(
    call: &mut CallEntryPoint,
    vm: &mut VirtualMachine,
    syscall_handler: &mut DeprecatedSyscallHintProcessor<'_>,
    cheatnet_state: &mut CheatnetState,
) -> DeprecatedSyscallResult<ReadOnlySegment> {
    let mut runtime_state = RuntimeState { cheatnet_state };
    // region: Modified blockifier code
    let call_info = execute_call_entry_point(
        call,
        syscall_handler.state,
        &mut runtime_state,
        syscall_handler.resources,
        syscall_handler.context,
    )?;
    // endregion

    let retdata = &call_info.execution.retdata.0;
    let retdata: Vec<MaybeRelocatable> = retdata
        .iter()
        .map(|&x| MaybeRelocatable::from(Felt252::from_(x)))
        .collect();
    let retdata_segment_start_ptr = syscall_handler.read_only_segments.allocate(vm, &retdata)?;

    syscall_handler.inner_calls.push(call_info);
    Ok(ReadOnlySegment {
        start_ptr: retdata_segment_start_ptr,
        length: retdata.len(),
    })
}

// blockifier/src/execution/deprecated_syscalls/hint_processor.rs:409 (execute_library_call)
#[allow(clippy::too_many_arguments)]
pub fn execute_library_call(
    syscall_handler: &mut DeprecatedSyscallHintProcessor<'_>,
    cheatnet_state: &mut CheatnetState,
    vm: &mut VirtualMachine,
    class_hash: ClassHash,
    code_address: Option<ContractAddress>,
    call_to_external: bool,
    entry_point_selector: EntryPointSelector,
    calldata: Calldata,
) -> DeprecatedSyscallResult<ReadOnlySegment> {
    let entry_point_type = if call_to_external {
        EntryPointType::External
    } else {
        EntryPointType::L1Handler
    };
    let mut entry_point = CallEntryPoint {
        class_hash: Some(class_hash),
        code_address,
        entry_point_type,
        entry_point_selector,
        calldata,
        // The call context remains the same in a library call.
        storage_address: syscall_handler.storage_address,
        caller_address: syscall_handler.caller_address,
        call_type: CallType::Delegate,
        initial_gas: constants::INITIAL_GAS_COST,
    };

    execute_inner_call(&mut entry_point, vm, syscall_handler, cheatnet_state)
}

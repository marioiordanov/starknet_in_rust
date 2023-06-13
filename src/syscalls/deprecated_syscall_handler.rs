use super::{
    deprecated_business_logic_syscall_handler::DeprecatedBLSyscallHandler, hint_code::*,
    other_syscalls, syscall_handler::HintProcessorPostRun,
};
use crate::{
    business_logic::state::state_api::{State, StateReader},
    syscalls::syscall_handler_errors::SyscallHandlerError,
};
use cairo_vm::felt::Felt252;
use cairo_vm::{
    hint_processor::{
        builtin_hint_processor::{
            builtin_hint_processor_definition::{BuiltinHintProcessor, HintProcessorData},
            hint_utils::get_relocatable_from_var_name,
        },
        hint_processor_definition::{HintProcessor, HintReference},
    },
    serde::deserialize_program::ApTracking,
    types::{exec_scope::ExecutionScopes, relocatable::Relocatable},
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};
use std::{any::Any, collections::HashMap};

pub(crate) struct DeprecatedSyscallHintProcessor<'a, T: State + StateReader> {
    pub(crate) builtin_hint_processor: BuiltinHintProcessor,
    pub(crate) syscall_handler: DeprecatedBLSyscallHandler<'a, T>,
}

impl<'a, T: State + StateReader> DeprecatedSyscallHintProcessor<'a, T> {
    pub fn new(syscall_handler: DeprecatedBLSyscallHandler<'a, T>) -> Self {
        DeprecatedSyscallHintProcessor {
            builtin_hint_processor: BuiltinHintProcessor::new_empty(),
            syscall_handler,
        }
    }

    pub fn should_run_syscall_hint(
        &mut self,
        vm: &mut VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
        hint_data: &Box<dyn Any>,
        constants: &HashMap<String, Felt252>,
    ) -> Result<bool, HintError> {
        match self
            .builtin_hint_processor
            .execute_hint(vm, exec_scopes, hint_data, constants)
        {
            Ok(()) => Ok(false),
            Err(HintError::UnknownHint(_)) => Ok(true),
            Err(e) => Err(e),
        }
    }

    fn execute_syscall_hint(
        &mut self,
        vm: &mut VirtualMachine,
        _exec_scopes: &mut ExecutionScopes,
        hint_data: &Box<dyn Any>,
        constants: &HashMap<String, Felt252>,
    ) -> Result<(), SyscallHandlerError> {
        let hint_data = hint_data
            .downcast_ref::<HintProcessorData>()
            .ok_or(SyscallHandlerError::WrongHintData)?;

        match hint_data.code.as_str() {
            ADDR_BOUND_PRIME => other_syscalls::addr_bound_prime(vm, hint_data, constants),
            ADDR_IS_250 => other_syscalls::addr_is_250(vm, hint_data),
            DEPLOY => {
                let syscall_ptr = get_syscall_ptr(vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
                self.syscall_handler.deploy(vm, syscall_ptr)
            }
            EMIT_EVENT_CODE => {
                let syscall_ptr = get_syscall_ptr(vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
                self.syscall_handler.emit_event(vm, syscall_ptr)
            }
            GET_BLOCK_NUMBER => {
                let syscall_ptr = get_syscall_ptr(vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
                self.syscall_handler.get_block_number(vm, syscall_ptr)
            }
            GET_BLOCK_TIMESTAMP => {
                let syscall_ptr = get_syscall_ptr(vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
                self.syscall_handler.get_block_timestamp(vm, syscall_ptr)
            }
            GET_CALLER_ADDRESS => {
                let syscall_ptr = get_syscall_ptr(vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
                self.syscall_handler.get_caller_address(vm, syscall_ptr)
            }
            GET_SEQUENCER_ADDRESS => {
                let syscall_ptr = get_syscall_ptr(vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
                self.syscall_handler.get_sequencer_address(vm, syscall_ptr)
            }
            LIBRARY_CALL => {
                let syscall_ptr = get_syscall_ptr(vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
                self.syscall_handler.library_call(vm, syscall_ptr)
            }
            LIBRARY_CALL_L1_HANDLER => {
                let syscall_ptr = get_syscall_ptr(vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
                self.syscall_handler
                    .library_call_l1_handler(vm, syscall_ptr)
            }
            CALL_CONTRACT => {
                let syscall_ptr = get_syscall_ptr(vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
                self.syscall_handler.call_contract(vm, syscall_ptr)
            }
            STORAGE_READ => {
                let syscall_ptr = get_syscall_ptr(vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
                self.syscall_handler.storage_read(vm, syscall_ptr)
            }
            STORAGE_WRITE => {
                let syscall_ptr = get_syscall_ptr(vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
                self.syscall_handler.storage_write(vm, syscall_ptr)
            }
            SEND_MESSAGE_TO_L1 => {
                let syscall_ptr = get_syscall_ptr(vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
                self.syscall_handler.send_message_to_l1(vm, syscall_ptr)
            }
            GET_TX_SIGNATURE => {
                let syscall_ptr = get_syscall_ptr(vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
                self.syscall_handler.get_tx_signature(vm, syscall_ptr)
            }
            GET_TX_INFO => {
                let syscall_ptr = get_syscall_ptr(vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
                self.syscall_handler.get_tx_info(vm, syscall_ptr)
            }
            GET_CONTRACT_ADDRESS => {
                let syscall_ptr = get_syscall_ptr(vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
                self.syscall_handler.get_contract_address(vm, syscall_ptr)
            }
            DELEGATE_CALL => {
                let syscall_ptr = get_syscall_ptr(vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
                self.syscall_handler.delegate_call(vm, syscall_ptr)
            }
            REPLACE_CLASS => {
                let syscall_ptr = get_syscall_ptr(vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
                self.syscall_handler.replace_class(vm, syscall_ptr)
            }
            _ => Err(SyscallHandlerError::NotImplemented(hint_data.code.clone())),
        }
    }
}

impl<'a, T: State + StateReader> HintProcessor for DeprecatedSyscallHintProcessor<'a, T> {
    fn execute_hint(
        &mut self,
        vm: &mut VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
        hint_data: &Box<dyn Any>,
        constants: &HashMap<String, Felt252>,
    ) -> Result<(), HintError> {
        if self.should_run_syscall_hint(vm, exec_scopes, hint_data, constants)? {
            self.execute_syscall_hint(vm, exec_scopes, hint_data, constants)
                .map_err(|e| match e {
                    SyscallHandlerError::NotImplemented(hint_code) => {
                        HintError::UnknownHint(hint_code.into_boxed_str())
                    }

                    e => HintError::CustomHint(e.to_string().into_boxed_str()),
                })?;
        }
        Ok(())
    }
}

impl<'a, T: State + StateReader> HintProcessorPostRun for DeprecatedSyscallHintProcessor<'a, T> {
    fn post_run(
        &self,
        runner: &mut VirtualMachine,
        syscall_stop_ptr: Relocatable,
    ) -> Result<(), crate::business_logic::transaction::error::TransactionError> {
        self.syscall_handler.post_run(runner, syscall_stop_ptr)
    }
}

fn get_syscall_ptr(
    vm: &VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<Relocatable, SyscallHandlerError> {
    let location = get_relocatable_from_var_name("syscall_ptr", vm, ids_data, ap_tracking)?;
    let syscall_ptr = vm.get_relocatable(location)?;
    Ok(syscall_ptr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        add_segments, allocate_selector, any_box,
        business_logic::{
            execution::{OrderedEvent, OrderedL2ToL1Message, TransactionExecutionContext},
            state::in_memory_state_reader::InMemoryStateReader,
            state::{
                cached_state::CachedState,
                state_api::{State, StateReader},
            },
            transaction::InvokeFunction,
        },
        definitions::{
            constants::TRANSACTION_VERSION, general_config::TransactionContext,
            transaction_type::TransactionType,
        },
        memory_insert,
        services::api::contract_classes::deprecated_contract_class::ContractClass,
        syscalls::deprecated_syscall_request::{
            DeprecatedDeployRequest, DeprecatedSendMessageToL1SysCallRequest,
            DeprecatedSyscallRequest,
        },
        utils::{
            felt_to_hash, get_big_int, get_integer, get_relocatable,
            test_utils::{ids_data, vm},
            Address,
        },
    };
    use cairo_vm::relocatable;
    use num_traits::Num;
    use starknet_contract_class::EntryPointType;
    use std::path::PathBuf;

    type DeprecatedBLSyscallHandler<'a> =
        crate::syscalls::deprecated_business_logic_syscall_handler::DeprecatedBLSyscallHandler<
            'a,
            CachedState<InMemoryStateReader>,
        >;
    type SyscallHintProcessor<'a, T> = super::DeprecatedSyscallHintProcessor<'a, T>;

    #[test]
    fn read_send_message_to_l1_request() {
        let mut state = CachedState::<InMemoryStateReader>::default();
        let syscall = DeprecatedBLSyscallHandler::default_with(&mut state);
        let mut vm = vm!();
        add_segments!(vm, 3);

        memory_insert!(
            vm,
            [((1, 0), 0), ((1, 1), 1), ((1, 2), 2), ((1, 3), (2, 0))]
        );
        assert_matches!(
            syscall.read_syscall_request("send_message_to_l1", &vm, relocatable!(1, 0)),
            Ok(request) if request == DeprecatedSyscallRequest::SendMessageToL1(DeprecatedSendMessageToL1SysCallRequest {
                _selector: 0.into(),
                to_address: Address(1.into()),
                payload_size: 2,
                payload_ptr: relocatable!(2, 0)
            })
        )
    }

    #[test]
    fn read_deploy_syscall_request() {
        let mut state = CachedState::<InMemoryStateReader>::default();
        let syscall = DeprecatedBLSyscallHandler::default_with(&mut state);
        let mut vm = vm!();
        add_segments!(vm, 2);

        memory_insert!(
            vm,
            [
                ((1, 0), 0),
                ((1, 1), 1),
                ((1, 2), 2),
                ((1, 3), 3),
                ((1, 4), (1, 20)),
                ((1, 5), 4)
            ]
        );

        assert_matches!(
            syscall.read_syscall_request("deploy", &vm, relocatable!(1, 0)),
            Ok(request) if request == DeprecatedSyscallRequest::Deploy(DeprecatedDeployRequest {
                _selector: 0.into(),
                class_hash: 1.into(),
                contract_address_salt: 2.into(),
                constructor_calldata_size: 3.into(),
                constructor_calldata: relocatable!(1, 20),
                deploy_from_zero: 4,
            })
        )
    }

    #[test]
    fn get_block_timestamp_for_business_logic() {
        let mut state = CachedState::<InMemoryStateReader>::default();
        let syscall = DeprecatedBLSyscallHandler::default_with(&mut state);
        let mut vm = vm!();
        add_segments!(vm, 2);

        memory_insert!(
            vm,
            [
                ((1, 0), (1, 1)), // syscall_ptr
                ((1, 1), 18)
            ]
        );

        let ids_data = ids_data!["syscall_ptr"];

        let hint_data = HintProcessorData::new_default(GET_BLOCK_TIMESTAMP.to_string(), ids_data);

        // invoke syscall
        let mut state = CachedState::<InMemoryStateReader>::default();
        let mut syscall_handler =
            SyscallHintProcessor::new(DeprecatedBLSyscallHandler::default_with(&mut state));
        syscall_handler
            .execute_hint(
                &mut vm,
                &mut ExecutionScopes::new(),
                &any_box!(hint_data),
                &HashMap::new(),
            )
            .unwrap();

        // Check that syscall.get_block_timestamp insert syscall.get_block_info().block_timestamp in the (1,2) position
        assert_eq!(
            get_big_int(&vm, relocatable!(1, 2)).unwrap(),
            syscall.get_block_info().block_timestamp.into()
        );
    }

    #[test]
    fn get_sequencer_address_for_business_logic() {
        let mut vm = vm!();
        add_segments!(vm, 2);

        memory_insert!(vm, [((1, 0), (1, 1)), ((1, 1), 18)]);

        let ids_data = ids_data!["syscall_ptr"];

        let hint_data = HintProcessorData::new_default(GET_SEQUENCER_ADDRESS.to_string(), ids_data);

        // invoke syscall
        let mut state = CachedState::<InMemoryStateReader>::default();
        let mut syscall_handler =
            SyscallHintProcessor::new(DeprecatedBLSyscallHandler::default_with(&mut state));
        syscall_handler
            .execute_hint(
                &mut vm,
                &mut ExecutionScopes::new(),
                &any_box!(hint_data),
                &HashMap::new(),
            )
            .unwrap();

        // Check that syscall.get_sequencer insert syscall.get_block_info().sequencer_address in the (1,1) position
        assert_eq!(get_big_int(&vm, relocatable!(1, 2)).unwrap(), 0.into())
    }

    #[test]
    fn emit_event_test() {
        // create data and variables to execute hint

        let mut vm = vm!();
        add_segments!(vm, 4);

        // insert keys and data to generate the event
        // keys ptr points to (3,0)
        // data ptr points to (3,3)

        // selector of syscall
        let selector = "1280709301550335749748";

        allocate_selector!(vm, ((2, 0), selector.as_bytes()));
        memory_insert!(
            vm,
            [
                ((1, 0), (2, 0)), // syscall ptr
                ((2, 1), 2),      // keys len
                ((2, 2), (3, 0)), // keys ptr
                ((2, 3), 2),      // data len
                ((2, 4), (3, 3)), // data ptr
                ((3, 0), 1),      // keys pointed by key ptr
                ((3, 1), 1),
                ((3, 3), 1), // data pointed by data ptr
                ((3, 4), 1)
            ]
        );
        // syscall_ptr
        let ids_data = ids_data!["syscall_ptr"];

        let hint_data = HintProcessorData::new_default(EMIT_EVENT_CODE.to_string(), ids_data);

        // invoke syscall
        let mut state = CachedState::<InMemoryStateReader>::default();
        let mut syscall_handler =
            SyscallHintProcessor::new(DeprecatedBLSyscallHandler::default_with(&mut state));
        syscall_handler
            .execute_hint(
                &mut vm,
                &mut ExecutionScopes::new(),
                &any_box!(hint_data),
                &HashMap::new(),
            )
            .unwrap();

        let event = syscall_handler
            .syscall_handler
            .events
            .get(0)
            .unwrap()
            .clone();

        assert_eq!(
            OrderedEvent::new(
                0,
                Vec::from([1.into(), 1.into()]),
                Vec::from([1.into(), 1.into()])
            ),
            event
        );
        assert_eq!(
            syscall_handler
                .syscall_handler
                .tx_execution_context
                .n_emitted_events,
            1
        );
    }

    #[test]
    fn get_tx_info_for_business_logic_test() {
        let mut vm = vm!();
        add_segments!(vm, 3);

        // insert data to form the request
        memory_insert!(
            vm,
            [
                ((1, 0), (2, 0)), //  syscall_ptr
                ((2, 0), 8)       //  GetTxInfoRequest.selector
            ]
        );

        // syscall_ptr
        let ids_data = ids_data!["syscall_ptr"];

        let hint_data = HintProcessorData::new_default(GET_TX_INFO.to_string(), ids_data);

        // invoke syscall
        let mut state = CachedState::<InMemoryStateReader>::default();
        let mut syscall_handler_hint_processor =
            SyscallHintProcessor::new(DeprecatedBLSyscallHandler::default_with(&mut state));

        let tx_execution_context = TransactionExecutionContext {
            n_emitted_events: 50,
            version: 51.into(),
            account_contract_address: Address(260.into()),
            max_fee: 261,
            transaction_hash: 262.into(),
            signature: vec![300.into(), 301.into()],
            nonce: 263.into(),
            n_sent_messages: 52,
            _n_steps: 100000,
        };
        syscall_handler_hint_processor
            .syscall_handler
            .tx_execution_context = tx_execution_context.clone();

        let result = syscall_handler_hint_processor.execute_hint(
            &mut vm,
            &mut ExecutionScopes::new(),
            &any_box!(hint_data),
            &HashMap::new(),
        );

        assert_matches!(result, Ok(()));

        // Check VM inserts

        // TransactionExecutionContext.signature
        assert_eq!(
            vm.get_integer(relocatable!(3, 0)).unwrap().into_owned(),
            tx_execution_context.signature[0]
        );
        assert_eq!(
            vm.get_integer(relocatable!(3, 1)).unwrap().into_owned(),
            tx_execution_context.signature[1]
        );

        // TxInfoStruct
        assert_matches!(
            get_big_int(&vm, relocatable!(4, 0)),
            Ok(field) if field == tx_execution_context.version
        );
        assert_matches!(
            get_big_int(&vm, relocatable!(4, 1)),
            Ok(field) if field == tx_execution_context.account_contract_address.0
        );
        assert_matches!(
            get_integer(&vm, relocatable!(4, 2)),
            Ok(field) if field == tx_execution_context.max_fee as usize
        );
        assert_matches!(
            get_integer(&vm, relocatable!(4, 3)),
            Ok(field) if field == tx_execution_context.signature.len()
        );
        assert_matches!(
            get_relocatable(&vm, relocatable!(4, 4)),
            Ok(field) if field == relocatable!(3, 0)
        );
        assert_matches!(
            get_big_int(&vm, relocatable!(4, 5)),
            Ok(field) if field == tx_execution_context.transaction_hash
        );
        assert_matches!(
            get_big_int(&vm, relocatable!(4, 6)),
            Ok(field) if field == syscall_handler_hint_processor
                .syscall_handler
                .general_config
                .starknet_os_config
                .chain_id
                .to_felt());

        assert_matches!(
            get_big_int(&vm, relocatable!(4, 7)),
            Ok(field) if field == tx_execution_context.nonce
        );

        // DeprecatedGetTxInfoResponse
        assert_eq!(
            vm.get_relocatable(relocatable!(2, 1)),
            Ok(relocatable!(4, 0))
        );
    }

    #[test]
    fn get_tx_info_for_business_logic_with_tx_info_ptr() {
        let mut vm = vm!();
        add_segments!(vm, 3);

        // insert data to form the request
        memory_insert!(
            vm,
            [
                ((1, 0), (2, 0)), //  syscall_ptr
                ((2, 0), 8)       //  GetTxInfoRequest.selector
            ]
        );

        // syscall_ptr
        let ids_data = ids_data!["syscall_ptr"];

        let hint_data = HintProcessorData::new_default(GET_TX_INFO.to_string(), ids_data);

        // invoke syscall
        let mut state = CachedState::<InMemoryStateReader>::default();
        let mut syscall_handler_hint_processor =
            SyscallHintProcessor::new(DeprecatedBLSyscallHandler::default_with(&mut state));

        syscall_handler_hint_processor.syscall_handler.tx_info_ptr =
            Some(relocatable!(7, 0).into());

        let result = syscall_handler_hint_processor.execute_hint(
            &mut vm,
            &mut ExecutionScopes::new(),
            &any_box!(hint_data),
            &HashMap::new(),
        );

        assert_matches!(result, Ok(()));

        // DeprecatedGetTxInfoResponse
        assert_matches!(
            vm.get_relocatable(relocatable!(2, 1)),
            Ok(relocatable!(7, 0))
        );
    }

    #[test]
    fn test_get_caller_address_ok() {
        let mut vm = vm!();

        add_segments!(vm, 2);

        // direction (1,0) is the sycall_ptr
        memory_insert!(vm, [((1, 0), (1, 1)), ((1, 1), 0)]);

        // syscall_ptr
        let ids_data = ids_data!["syscall_ptr"];

        let hint_data = HintProcessorData::new_default(GET_CALLER_ADDRESS.to_string(), ids_data);

        // invoke syscall
        let mut state = CachedState::<InMemoryStateReader>::default();
        let mut hint_processor =
            SyscallHintProcessor::new(DeprecatedBLSyscallHandler::default_with(&mut state));
        hint_processor
            .execute_hint(
                &mut vm,
                &mut ExecutionScopes::new(),
                &any_box!(hint_data),
                &HashMap::new(),
            )
            .unwrap();

        // response is written in direction (1,2)
        assert_eq!(
            get_big_int(&vm, relocatable!(1, 2)).unwrap(),
            hint_processor.syscall_handler.caller_address.0
        )
    }

    #[test]
    fn test_send_message_to_l1_ok() {
        let mut vm = vm!();

        add_segments!(vm, 3);

        // parameters are read from memory location (1,1)
        memory_insert!(
            vm,
            [
                ((1, 0), (1, 1)), // syscall_ptr
                ((1, 1), 0),
                ((1, 2), 1),
                ((1, 3), 2),
                ((1, 4), (2, 0)),
                ((2, 0), 18),
                ((2, 1), 12)
            ]
        );

        // syscall_ptr
        let ids_data = ids_data!["syscall_ptr"];

        let hint_data = HintProcessorData::new_default(SEND_MESSAGE_TO_L1.to_string(), ids_data);

        // invoke syscall
        let mut state = CachedState::<InMemoryStateReader>::default();
        let mut hint_processor =
            SyscallHintProcessor::new(DeprecatedBLSyscallHandler::default_with(&mut state));
        hint_processor
            .execute_hint(
                &mut vm,
                &mut ExecutionScopes::new(),
                &any_box!(hint_data),
                &HashMap::new(),
            )
            .unwrap();

        assert_eq!(
            hint_processor
                .syscall_handler
                .tx_execution_context
                .n_sent_messages,
            1
        );
        assert_eq!(
            hint_processor.syscall_handler.l2_to_l1_messages,
            vec![OrderedL2ToL1Message::new(
                hint_processor
                    .syscall_handler
                    .tx_execution_context
                    .n_sent_messages
                    - 1,
                Address(1.into()),
                vec![18.into(), 12.into()],
            )]
        );
    }

    #[test]
    fn test_get_block_number() {
        let mut vm = vm!();

        add_segments!(vm, 4);
        memory_insert!(
            vm,
            [
                ((1, 0), (2, 0)), // Syscall pointer.
                ((2, 0), 0)       // selector
            ]
        );

        let mut state = CachedState::<InMemoryStateReader>::default();
        let mut hint_processor =
            SyscallHintProcessor::new(DeprecatedBLSyscallHandler::default_with(&mut state));

        let hint_data =
            HintProcessorData::new_default(GET_BLOCK_NUMBER.to_string(), ids_data!["syscall_ptr"]);
        assert_matches!(
            hint_processor.execute_hint(
                &mut vm,
                &mut ExecutionScopes::new(),
                &any_box!(hint_data),
                &HashMap::new(),
            ),
            Ok(())
        );
        assert_matches!(get_integer(&vm, relocatable!(2, 1)), Ok(0));
    }

    #[test]
    fn test_get_contract_address_ok() {
        let mut vm = vm!();

        add_segments!(vm, 2);

        // direction (1,0) is the sycall_ptr
        memory_insert!(vm, [((1, 0), (1, 1)), ((1, 1), 0)]);

        // syscall_ptr
        let ids_data = ids_data!["syscall_ptr"];

        let hint_data = HintProcessorData::new_default(GET_CONTRACT_ADDRESS.to_string(), ids_data);

        // invoke syscall
        let mut state = CachedState::<InMemoryStateReader>::default();
        let mut hint_processor =
            SyscallHintProcessor::new(DeprecatedBLSyscallHandler::default_with(&mut state));
        hint_processor
            .execute_hint(
                &mut vm,
                &mut ExecutionScopes::new(),
                &any_box!(hint_data),
                &HashMap::new(),
            )
            .unwrap();

        // response is written in direction (1,2)
        assert_eq!(
            get_big_int(&vm, relocatable!(1, 2)).unwrap(),
            hint_processor.syscall_handler.contract_address.0
        )
    }

    #[test]
    fn test_gt_tx_signature() {
        let mut vm = vm!();

        add_segments!(vm, 3);

        memory_insert!(
            vm,
            [
                ((1, 0), (2, 0)), //  syscall_ptr
                ((2, 0), 8)       //  GetTxInfoRequest.selector
            ]
        );

        // syscall_ptr
        let ids_data = ids_data!["syscall_ptr"];

        let hint_data = HintProcessorData::new_default(GET_TX_SIGNATURE.to_string(), ids_data);

        // invoke syscall
        let mut state = CachedState::<InMemoryStateReader>::default();
        let mut syscall_handler_hint_processor =
            SyscallHintProcessor::new(DeprecatedBLSyscallHandler::default_with(&mut state));

        let tx_execution_context = TransactionExecutionContext {
            n_emitted_events: 50,
            version: 51.into(),
            account_contract_address: Address(260.into()),
            max_fee: 261,
            transaction_hash: 262.into(),
            signature: vec![300.into(), 301.into()],
            nonce: 263.into(),
            n_sent_messages: 52,
            _n_steps: 10000,
        };
        syscall_handler_hint_processor
            .syscall_handler
            .tx_execution_context = tx_execution_context.clone();

        let result = syscall_handler_hint_processor.execute_hint(
            &mut vm,
            &mut ExecutionScopes::new(),
            &any_box!(hint_data),
            &HashMap::new(),
        );

        assert!(result.is_ok());
        assert_eq!(
            get_integer(&vm, relocatable!(2, 1)).unwrap(),
            tx_execution_context.signature.len()
        );
        assert_eq!(
            vm.get_relocatable(relocatable!(2, 2)).unwrap(),
            relocatable!(3, 0)
        );
    }

    #[test]
    fn test_bl_storage_read_hint_ok() {
        let mut vm = vm!();
        add_segments!(vm, 3);

        let address = Felt252::from_str_radix(
            "2151680050850558576753658069693146429350618838199373217695410689374331200218",
            10,
        )
        .unwrap();
        // insert data to form the request
        memory_insert!(
            vm,
            [
                ((1, 0), (2, 0)), //  syscall_ptr
                ((2, 0), 10)      //  StorageReadRequest.selector
            ]
        );

        // StorageReadRequest.address
        vm.insert_value(relocatable!(2, 1), address.clone())
            .unwrap();

        // syscall_ptr
        let ids_data = ids_data!["syscall_ptr"];

        let hint_data = HintProcessorData::new_default(STORAGE_READ.to_string(), ids_data);

        let mut state = CachedState::<InMemoryStateReader>::default();
        let mut syscall_handler_hint_processor =
            SyscallHintProcessor::new(DeprecatedBLSyscallHandler::default_with(&mut state));

        let storage_value = Felt252::new(3);
        syscall_handler_hint_processor
            .syscall_handler
            .starknet_storage_state
            .state
            .set_storage_at(
                &(
                    syscall_handler_hint_processor
                        .syscall_handler
                        .starknet_storage_state
                        .contract_address
                        .clone(),
                    address.to_bytes_be().try_into().unwrap(),
                ),
                storage_value.clone(),
            );
        assert!(syscall_handler_hint_processor
            .execute_hint(
                &mut vm,
                &mut ExecutionScopes::new(),
                &any_box!(hint_data),
                &HashMap::new(),
            )
            .is_ok());

        // Check DeprecatedStorageReadResponse insert
        assert_matches!(get_big_int(&vm, relocatable!(2, 2)), Ok(response) if response == storage_value );
    }

    #[test]
    fn test_bl_storage_write_hint_ok() {
        let mut vm = vm!();
        add_segments!(vm, 3);

        let address = Felt252::from_str_radix(
            "2151680050850558576753658069693146429350618838199373217695410689374331200218",
            10,
        )
        .unwrap();

        memory_insert!(
            vm,
            [
                ((1, 0), (2, 0)), //  syscall_ptr
                ((2, 0), 10),     //  StorageWriteRequest.selector
                ((2, 2), 45)      //  StorageWriteRequest.value
            ]
        );

        // StorageWriteRequest.address
        vm.insert_value(relocatable!(2, 1), address.clone())
            .unwrap();

        // syscall_ptr
        let ids_data = ids_data!["syscall_ptr"];

        let hint_data = HintProcessorData::new_default(STORAGE_WRITE.to_string(), ids_data);

        let mut state = CachedState::<InMemoryStateReader>::default();
        let mut syscall_handler_hint_processor =
            SyscallHintProcessor::new(DeprecatedBLSyscallHandler::default_with(&mut state));

        syscall_handler_hint_processor
            .syscall_handler
            .starknet_storage_state
            .state
            .set_storage_at(
                &(
                    syscall_handler_hint_processor
                        .syscall_handler
                        .starknet_storage_state
                        .contract_address
                        .clone(),
                    address.to_bytes_be().try_into().unwrap(),
                ),
                Felt252::new(3),
            );
        assert!(syscall_handler_hint_processor
            .execute_hint(
                &mut vm,
                &mut ExecutionScopes::new(),
                &any_box!(hint_data),
                &HashMap::new(),
            )
            .is_ok());

        let write = syscall_handler_hint_processor
            .syscall_handler
            .starknet_storage_state
            .read(&felt_to_hash(&address));

        assert_eq!(write, Ok(Felt252::new(45)));
    }

    #[test]
    fn test_bl_deploy_ok() {
        let mut vm = vm!();
        add_segments!(vm, 4);

        // insert data to form the request
        memory_insert!(
            vm,
            [
                ((1, 0), (2, 0)), //  syscall_ptr
                ((2, 0), 10),     // DeployRequestStruct._selector
                // ((2, 1), class_hash),     // DeployRequestStruct.class_hash
                ((2, 2), 12),     // DeployRequestStruct.contract_address_salt
                ((2, 3), 0),      // DeployRequestStruct.constructor_calldata_size
                ((2, 4), (3, 0)), // DeployRequestStruct.constructor_calldata
                ((2, 5), 0)       // DeployRequestStruct.deploy_from_zero
            ]
        );

        let class_hash_felt = Felt252::from_str_radix(
            "284536ad7de8852cc9101133f7f7670834084d568610335c94da1c4d9ce4be6",
            16,
        )
        .unwrap();
        let class_hash: [u8; 32] = class_hash_felt.to_bytes_be().try_into().unwrap();

        vm.insert_value(relocatable!(2, 1), class_hash_felt)
            .unwrap();

        // Hinta data
        let ids_data = ids_data!["syscall_ptr"];
        let hint_data = HintProcessorData::new_default(DEPLOY.to_string(), ids_data);

        // Create SyscallHintProcessor
        let mut state = CachedState::<InMemoryStateReader>::default();
        let mut syscall_handler_hint_processor =
            SyscallHintProcessor::new(DeprecatedBLSyscallHandler::default_with(&mut state));
        // Initialize state.set_contract_classes
        syscall_handler_hint_processor
            .syscall_handler
            .starknet_storage_state
            .state
            .set_contract_classes(HashMap::new())
            .unwrap();

        // Set contract class
        let contract_class =
            ContractClass::try_from(PathBuf::from("starknet_programs/fibonacci.json")).unwrap();
        syscall_handler_hint_processor
            .syscall_handler
            .starknet_storage_state
            .state
            .set_contract_class(&class_hash, &contract_class)
            .unwrap();

        // Execute Deploy hint
        assert_matches!(
            syscall_handler_hint_processor.execute_hint(
                &mut vm,
                &mut ExecutionScopes::new(),
                &any_box!(hint_data),
                &HashMap::new(),
            ),
            Ok(())
        );

        // Check VM inserts
        // DeprecatedDeployResponse.contract_address
        let deployed_address = get_big_int(&vm, relocatable!(2, 6)).unwrap();
        // DeprecatedDeployResponse.constructor_retdata_size
        assert_matches!(get_big_int(&vm, relocatable!(2, 7)), Ok(constructor_retdata_size) if constructor_retdata_size == 0.into());
        // DeprecatedDeployResponse.constructor_retdata
        assert_matches!(
            get_relocatable(&vm, relocatable!(2, 8)),
            Ok(relocatable!(0, 0))
        );

        // Check State diff
        assert_eq!(
            syscall_handler_hint_processor
                .syscall_handler
                .starknet_storage_state
                .state
                .get_class_hash_at(&Address(deployed_address)),
            Ok(class_hash)
        );
    }

    #[test]
    fn test_deploy_and_invoke() {
        /*
        DEPLOY
        */
        let mut vm = vm!();
        add_segments!(vm, 4);

        // insert data to form the request
        memory_insert!(
            vm,
            [
                ((1, 0), (2, 0)), //  syscall_ptr
                ((2, 0), 10),     // DeployRequestStruct._selector
                // ((2, 1), class_hash),     // DeployRequestStruct.class_hash
                ((2, 2), 12),     // DeployRequestStruct.contract_address_salt
                ((2, 3), 1),      // DeployRequestStruct.constructor_calldata_size
                ((2, 4), (3, 0)), // DeployRequestStruct.constructor_calldata
                ((2, 5), 0),      // DeployRequestStruct.deploy_from_zero
                ((3, 0), 250)     // constructor
            ]
        );

        let class_hash_felt = Felt252::from_str_radix(
            "284536ad7de8852cc9101133f7f7670834084d568610335c94da1c4d9ce4be6",
            16,
        )
        .unwrap();
        let class_hash: [u8; 32] = class_hash_felt.to_bytes_be().try_into().unwrap();

        vm.insert_value(relocatable!(2, 1), class_hash_felt)
            .unwrap();

        // Hinta data
        let ids_data = ids_data!["syscall_ptr"];
        let hint_data = HintProcessorData::new_default(
            "syscall_handler.deploy(segments=segments, syscall_ptr=ids.syscall_ptr)".to_string(),
            ids_data,
        );

        // Create SyscallHintProcessor
        let mut state = CachedState::<InMemoryStateReader>::default();
        let mut syscall_handler_hint_processor =
            SyscallHintProcessor::new(DeprecatedBLSyscallHandler::default_with(&mut state));
        // Initialize state.set_contract_classes
        syscall_handler_hint_processor
            .syscall_handler
            .starknet_storage_state
            .state
            .set_contract_classes(HashMap::new())
            .unwrap();

        // Set contract class
        let contract_class = ContractClass::try_from(PathBuf::from(
            "starknet_programs/storage_var_and_constructor.json",
        ))
        .unwrap();
        syscall_handler_hint_processor
            .syscall_handler
            .starknet_storage_state
            .state
            .set_contract_class(&class_hash, &contract_class)
            .unwrap();

        // Execute Deploy hint
        assert_matches!(
            syscall_handler_hint_processor.execute_hint(
                &mut vm,
                &mut ExecutionScopes::new(),
                &any_box!(hint_data),
                &HashMap::new(),
            ),
            Ok(())
        );

        // Check VM inserts
        // DeprecatedDeployResponse.contract_address
        let deployed_address = get_big_int(&vm, relocatable!(2, 6)).unwrap();
        // DeprecatedDeployResponse.constructor_retdata_size
        assert_matches!(get_big_int(&vm, relocatable!(2, 7)), Ok(constructor_retdata_size) if constructor_retdata_size == 0.into());
        // DeprecatedDeployResponse.constructor_retdata
        assert_matches!(
            get_relocatable(&vm, relocatable!(2, 8)),
            Ok(relocatable!(0, 0))
        );

        // Check State diff
        assert_eq!(
            syscall_handler_hint_processor
                .syscall_handler
                .starknet_storage_state
                .state
                .get_class_hash_at(&Address(deployed_address.clone())),
            Ok(class_hash)
        );

        /*
        INVOKE
        */
        let internal_invoke_function = InvokeFunction::new(
            Address(deployed_address.clone()),
            Felt252::from_str_radix(
                "283e8c15029ea364bfb37203d91b698bc75838eaddc4f375f1ff83c2d67395c",
                16,
            )
            .unwrap(),
            0,
            TRANSACTION_VERSION.clone(),
            vec![10.into()],
            Vec::new(),
            0.into(),
            Some(0.into()),
            None,
        )
        .unwrap();

        // Invoke result
        let result = internal_invoke_function
            .apply(&mut state, &TransactionContext::default())
            .unwrap();

        let result_call_info = result.call_info.unwrap();

        assert_eq!(result.tx_type, Some(TransactionType::InvokeFunction));
        assert_eq!(result_call_info.contract_address, Address(deployed_address));
        assert_eq!(result_call_info.class_hash, Some(class_hash));
        assert_eq!(
            result_call_info.entry_point_type,
            Some(EntryPointType::External)
        );
        assert_eq!(result_call_info.calldata, vec![10.into()]);
        assert_eq!(result_call_info.retdata, vec![260.into()]);
        assert_eq!(result_call_info.storage_read_values, vec![250.into()]);
    }
}

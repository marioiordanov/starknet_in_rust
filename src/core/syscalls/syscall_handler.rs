use super::business_logic_syscall_handler::BusinessLogicSyscallHandler;
use crate::business_logic::state::state_api::{State, StateReader};
use crate::business_logic::transaction::error::TransactionError;
use cairo_lang_casm::{
    hints::{Hint, StarknetHint},
    operand::{CellRef, DerefOrImmediate, Register, ResOperand},
};
use cairo_vm::{
    felt::Felt252,
    hint_processor::{
        cairo_1_hint_processor::hint_processor::Cairo1HintProcessor,
        hint_processor_definition::{HintProcessor, HintReference},
    },
    types::{
        errors::math_errors::MathError, exec_scope::ExecutionScopes, relocatable::Relocatable,
    },
    vm::{
        errors::{hint_errors::HintError, vm_errors::VirtualMachineError},
        vm_core::VirtualMachine,
    },
};
use std::{any::Any, boxed::Box, collections::HashMap};

pub(crate) trait HintProcessorPostRun {
    /// Performs post run syscall related tasks (if any).
    fn post_run(
        &self,
        _runner: &mut VirtualMachine,
        _syscall_stop_ptr: Relocatable,
    ) -> Result<(), TransactionError>;
}

#[allow(unused)]
pub(crate) struct SyscallHintProcessor<'a, T: State + StateReader> {
    pub(crate) cairo1_hint_processor: Cairo1HintProcessor,
    pub(crate) syscall_handler: BusinessLogicSyscallHandler<'a, T>,
}

impl<'a, T: State + StateReader> SyscallHintProcessor<'a, T> {
    pub fn new(
        syscall_handler: BusinessLogicSyscallHandler<'a, T>,
        hints: &[(usize, Vec<Hint>)],
    ) -> Self {
        SyscallHintProcessor {
            cairo1_hint_processor: Cairo1HintProcessor::new(hints),
            syscall_handler,
        }
    }
}

impl<'a, T: State + StateReader> HintProcessor for SyscallHintProcessor<'a, T> {
    fn execute_hint(
        &mut self,
        vm: &mut VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
        hint_data: &Box<dyn Any>,
        _constants: &HashMap<String, Felt252>,
    ) -> Result<(), HintError> {
        let hints: &Vec<Hint> = hint_data.downcast_ref().ok_or(HintError::WrongHintData)?;
        for hint in hints {
            match hint {
                Hint::Core(_core_hint) => {
                    self.cairo1_hint_processor.execute(vm, exec_scopes, hint)?
                }
                Hint::Starknet(starknet_hint) => match starknet_hint {
                    StarknetHint::SystemCall { system } => {
                        let syscall_ptr = as_relocatable(vm, system)?;
                        self.syscall_handler
                            .syscall(vm, syscall_ptr)
                            .map_err(|err| {
                                HintError::CustomHint(format!(
                                    "Syscall handler invocation error: {err}"
                                ))
                            })?;
                    }
                    other => return Err(HintError::UnknownHint(other.to_string())),
                },
            };
        }
        Ok(())
    }

    // Ignores all data except for the code that should contain
    fn compile_hint(
        &self,
        //Block of hint code as String
        hint_code: &str,
        //Ap Tracking Data corresponding to the Hint
        ap_tracking_data: &cairo_vm::serde::deserialize_program::ApTracking,
        //Map from variable name to reference id number
        //(may contain other variables aside from those used by the hint)
        reference_ids: &HashMap<String, usize>,
        //List of all references (key corresponds to element of the previous dictionary)
        references: &HashMap<usize, HintReference>,
    ) -> Result<Box<dyn Any>, VirtualMachineError> {
        self.cairo1_hint_processor.compile_hint(
            hint_code,
            ap_tracking_data,
            reference_ids,
            references,
        )
    }
}

impl<'a, T: State + StateReader> HintProcessorPostRun for SyscallHintProcessor<'a, T> {
    fn post_run(
        &self,
        runner: &mut VirtualMachine,
        syscall_stop_ptr: Relocatable,
    ) -> Result<(), crate::business_logic::transaction::error::TransactionError> {
        self.syscall_handler.post_run(runner, syscall_stop_ptr)
    }
}

// TODO: These four functions were copied from cairo-rs in
// hint_processor/cairo-1-hint-processor/hint_processor_utils.rs as these functions are private.
// They will became public soon and then we have to remove this ones and use the ones in cairo-rs instead
fn as_relocatable(vm: &mut VirtualMachine, value: &ResOperand) -> Result<Relocatable, HintError> {
    let (base, offset) = extract_buffer(value)?;
    get_ptr(vm, base, &offset).map_err(HintError::from)
}

fn extract_buffer(buffer: &ResOperand) -> Result<(&CellRef, Felt252), HintError> {
    let (cell, base_offset) = match buffer {
        ResOperand::Deref(cell) => (cell, 0.into()),
        ResOperand::BinOp(bin_op) => {
            if let DerefOrImmediate::Immediate(val) = &bin_op.b {
                (&bin_op.a, val.clone().value.into())
            } else {
                return Err(HintError::CustomHint("Failed to extract buffer, expected ResOperand of BinOp type to have Inmediate b value".to_owned()));
            }
        }
        _ => {
            return Err(HintError::CustomHint(
                "Illegal argument for a buffer.".to_string(),
            ))
        }
    };
    Ok((cell, base_offset))
}

fn get_ptr(
    vm: &VirtualMachine,
    cell: &CellRef,
    offset: &Felt252,
) -> Result<Relocatable, VirtualMachineError> {
    Ok((vm.get_relocatable(cell_ref_to_relocatable(cell, vm)?)? + offset)?)
}

fn cell_ref_to_relocatable(
    cell_ref: &CellRef,
    vm: &VirtualMachine,
) -> Result<Relocatable, MathError> {
    let base = match cell_ref.register {
        Register::AP => vm.get_ap(),
        Register::FP => vm.get_fp(),
    };
    base + (cell_ref.offset as i32)
}

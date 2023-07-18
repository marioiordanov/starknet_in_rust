use super::{CallInfo, TransactionExecutionContext};
use crate::{
    definitions::block_context::BlockContext,
    state::{
        state_api::{State, StateReader},
        ExecutionResourcesManager,
    },
    transaction::error::TransactionError,
    utils::Address,
};
use cairo_lang_sierra::extensions::core::{CoreLibfunc, CoreType};
use cairo_lang_sierra::program::Program as SierraProgram;
use cairo_native::easy::compile_and_execute;
use cairo_vm::{felt::Felt252, vm::runners::cairo_runner::ExecutionResources};
use serde_json::json;
use num_traits::One;
use num_traits::Zero;

#[derive(Debug)]
struct CairoNativeParams(Option<u64>, u64, Vec<u64>);

impl CairoNativeParams {
    pub fn to_json(&self) -> serde_json::Value {
        json!([
            self.0,
            self.1,
            ..self.2
        ])
    }
}

pub struct NativeEntryPoint {
    pub contract_address: Address,
    pub calldata: Vec<Felt252>,
    pub entrypoint_selector: Felt252,
    pub caller_address: Address,
    pub initial_gas: u128,
}

impl NativeEntryPoint {
    pub fn new(
        contract_address: Address,
        calldata: Vec<Felt252>,
        entrypoint_selector: Felt252,
        caller_address: Address,
        initial_gas: u128,
    ) -> Self {
        Self {
            contract_address,
            calldata,
            entrypoint_selector,
            caller_address,
            initial_gas,
        }
    }

    pub fn execute<T>(
        &self,
        state: &mut T,
        _block_context: &BlockContext,
        _resources_manager: &mut ExecutionResourcesManager,
        _tx_execution_context: &mut TransactionExecutionContext,
        _support_reverted: bool,
    ) -> Result<CallInfo, TransactionError>
    where
        T: State + StateReader,
    {
        // get the sierra class from the state
        let class_hash = state.get_class_hash_at(&self.contract_address)?;
        let program: SierraProgram = state.get_sierra_class(&class_hash)?;
        let mut writer: Vec<u8> = Vec::new();
        let mut res = serde_json::Serializer::new(&mut writer);
        // only range_check and gas (TODO: ask RJ about range_check)
        let mut calldata = [null, 9000];
        calldata.to_vec().extend(self.calldata);
        let function_id = &program
            .funcs
            .iter()
            .find(|x| {
                x.id.debug_name.as_deref() == function_id_by_selector(self.entrypoint_selector).as_deref()
            })
            .unwrap()
            .id;
        compile_and_execute::<CoreType, CoreLibfunc, _, _>(
            &program,
            function_id,
            json!(calldata),
            &mut res,
        )
        .unwrap();

        // The output expected as a string will be a json that looks like this:
        //   [
        // 0  null,
        // 1  9000,
        // 2  [
        // 2[0]   0,
        // 2[1]   [
        // 2[1][0]   [
        // 2[1][0][0]  55,
        // 2[1][0][1]  0,
        // 2[1][0][2]  0,
        // 2[1][0][3]  0,
        // 2[1][0][4]  0,
        // 2[1][0][5]  0,
        // 2[1][0][6]  0,
        // 2[1][0][7]  0]]]]
        let deserialized_result: String = String::from_utf8(writer).unwrap();
        let deserialized_value = serde_json::from_str::<serde_json::Value>(&deserialized_result)
            .expect("Failed to deserialize result");
        // TODO: Ask cairo_native team to return an array of 32 u8s
        let result = deserialized_value[2][1][0][0].as_u64().unwrap();
        let result_felt: Vec<Felt252> = vec![result.into()];
        // Create a CallInfo using the result from cairo_native
        Ok(CallInfo {
            caller_address: self.caller_address,
            call_type: None,
            contract_address: self.contract_address,
            code_address: None,
            class_hash: Some(class_hash.clone()),
            entry_point_selector: Some(self.entrypoint_selector),
            entry_point_type: None,
            calldata: self.calldata.clone(),
            retdata: result_felt,
            execution_resources: ExecutionResources::default(),
            events: Default::default(),
            l2_to_l1_messages: Default::default(),
            storage_read_values: Default::default(),
            accessed_storage_keys: Default::default(),
            internal_calls: Default::default(),
            gas_consumed: 0,
            failure_flag: false,
        })
    }
}

/// TODO: we should fix this
fn function_id_by_selector(selector: Felt252) -> Option<String> {
    let zero = Felt252::zero();
    let one = Felt252::one();
    match selector {
        zero => Some("fib_contract::fib_contract::Fibonacci::fib".into()),
        one => Some("".into()),
        _ => None
    }
    
}

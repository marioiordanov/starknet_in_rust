use std::collections::HashMap;

use crate::{
    business_logic::{
        execution::{
            execution_entry_point::ExecutionEntryPoint, CallInfo, TransactionExecutionContext,
            TransactionExecutionInfo,
        },
        state::state_api::{State, StateReader},
        state::ExecutionResourcesManager,
        transaction::{
            error::TransactionError,
            fee::{calculate_tx_fee, execute_fee_transfer, FeeInfo},
        },
    },
    core::transaction_hash::{calculate_transaction_hash_common, TransactionHashPrefix},
    definitions::{
        constants::{EXECUTE_ENTRY_POINT_SELECTOR, VALIDATE_ENTRY_POINT_SELECTOR},
        general_config::TransactionContext,
        transaction_type::TransactionType,
    },
    utils::{calculate_tx_resources, Address},
};
use cairo_vm::felt::Felt252;
use getset::Getters;
use num_traits::Zero;
use starknet_contract_class::EntryPointType;

#[derive(Debug, Getters)]
pub struct InvokeFunction {
    #[getset(get = "pub")]
    contract_address: Address,
    entry_point_selector: Felt252,
    #[allow(dead_code)]
    entry_point_type: EntryPointType,
    calldata: Vec<Felt252>,
    tx_type: TransactionType,
    version: Felt252,
    validate_entry_point_selector: Felt252,
    #[getset(get = "pub")]
    hash_value: Felt252,
    #[getset(get = "pub")]
    signature: Vec<Felt252>,
    max_fee: u128,
    nonce: Option<Felt252>,
}

impl InvokeFunction {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        contract_address: Address,
        entry_point_selector: Felt252,
        max_fee: u128,
        version: Felt252,
        calldata: Vec<Felt252>,
        signature: Vec<Felt252>,
        chain_id: Felt252,
        nonce: Option<Felt252>,
        hash_value: Option<Felt252>,
    ) -> Result<Self, TransactionError> {
        let (entry_point_selector_field, additional_data) = preprocess_invoke_function_fields(
            entry_point_selector.clone(),
            nonce.clone(),
            version.clone(),
        )?;
        let hash_value = match hash_value {
            Some(hash) => hash,
            None => calculate_transaction_hash_common(
                TransactionHashPrefix::Invoke,
                version.clone(),
                &contract_address,
                entry_point_selector_field,
                &calldata,
                max_fee,
                chain_id,
                &additional_data,
            )?,
        };
        let validate_entry_point_selector = VALIDATE_ENTRY_POINT_SELECTOR.clone();

        Ok(InvokeFunction {
            contract_address,
            entry_point_selector,
            entry_point_type: EntryPointType::External,
            calldata,
            tx_type: TransactionType::InvokeFunction,
            version,
            max_fee,
            signature,
            validate_entry_point_selector,
            nonce,
            hash_value,
        })
    }

    fn get_execution_context(
        &self,
        n_steps: u64,
    ) -> Result<TransactionExecutionContext, TransactionError> {
        Ok(TransactionExecutionContext::new(
            self.contract_address.clone(),
            self.hash_value.clone(),
            self.signature.clone(),
            self.max_fee,
            if self.version.is_zero() {
                Felt252::zero()
            } else {
                self.nonce.clone().ok_or(TransactionError::MissingNonce)?
            },
            n_steps,
            self.version.clone(),
        ))
    }

    fn run_validate_entrypoint<T>(
        &self,
        state: &mut T,
        resources_manager: &mut ExecutionResourcesManager,
        general_config: &TransactionContext,
    ) -> Result<Option<CallInfo>, TransactionError>
    where
        T: State + StateReader,
    {
        if self.entry_point_selector != *EXECUTE_ENTRY_POINT_SELECTOR {
            return Ok(None);
        }
        if self.version.is_zero() {
            return Ok(None);
        }

        let call = ExecutionEntryPoint::new(
            self.contract_address.clone(),
            self.calldata.clone(),
            self.validate_entry_point_selector.clone(),
            Address(0.into()),
            EntryPointType::External,
            None,
            None,
            0,
        );

        let call_info = call.execute(
            state,
            general_config,
            resources_manager,
            &self.get_execution_context(general_config.validate_max_n_steps)?,
            false,
        )?;

        verify_no_calls_to_other_contracts(&call_info)
            .map_err(|_| TransactionError::InvalidContractCall)?;

        Ok(Some(call_info))
    }

    /// Builds the transaction execution context and executes the entry point.
    /// Returns the CallInfo.
    fn run_execute_entrypoint<T>(
        &self,
        state: &mut T,
        general_config: &TransactionContext,
        resources_manager: &mut ExecutionResourcesManager,
    ) -> Result<CallInfo, TransactionError>
    where
        T: State + StateReader,
    {
        let call = ExecutionEntryPoint::new(
            self.contract_address.clone(),
            self.calldata.clone(),
            self.entry_point_selector.clone(),
            Address(0.into()),
            EntryPointType::External,
            None,
            None,
            0,
        );

        call.execute(
            state,
            general_config,
            resources_manager,
            &self.get_execution_context(general_config.invoke_tx_max_n_steps)?,
            false,
        )
    }

    /// Execute a call to the cairo-vm using the accounts_validation.cairo contract to validate
    /// the contract that is being declared. Then it returns the transaction execution info of the run.
    pub fn apply<S>(
        &self,
        state: &mut S,
        general_config: &TransactionContext,
    ) -> Result<TransactionExecutionInfo, TransactionError>
    where
        S: State + StateReader,
    {
        let mut resources_manager = ExecutionResourcesManager::default();

        let validate_info =
            self.run_validate_entrypoint(state, &mut resources_manager, general_config)?;
        // Execute transaction
        let call_info =
            self.run_execute_entrypoint(state, general_config, &mut resources_manager)?;
        let changes = state.count_actual_storage_changes();
        let actual_resources = calculate_tx_resources(
            resources_manager,
            &vec![Some(call_info.clone()), validate_info.clone()],
            self.tx_type,
            changes,
            None,
        )?;

        let transaction_execution_info =
            TransactionExecutionInfo::create_concurrent_stage_execution_info(
                validate_info,
                Some(call_info),
                actual_resources,
                Some(self.tx_type),
            );
        Ok(transaction_execution_info)
    }

    fn charge_fee<S>(
        &self,
        state: &mut S,
        resources: &HashMap<String, usize>,
        general_config: &TransactionContext,
    ) -> Result<FeeInfo, TransactionError>
    where
        S: State + StateReader,
    {
        if self.max_fee.is_zero() {
            return Ok((None, 0));
        }

        let actual_fee = calculate_tx_fee(
            resources,
            general_config.starknet_os_config.gas_price,
            general_config,
        )?;

        let tx_context = self.get_execution_context(general_config.invoke_tx_max_n_steps)?;
        let fee_transfer_info =
            execute_fee_transfer(state, general_config, &tx_context, actual_fee)?;

        Ok((Some(fee_transfer_info), actual_fee))
    }

    /// Calculates actual fee used by the transaction using the execution info returned by apply(),
    /// then updates the transaction execution info with the data of the fee.
    pub fn execute<S: State + StateReader>(
        &self,
        state: &mut S,
        general_config: &TransactionContext,
    ) -> Result<TransactionExecutionInfo, TransactionError> {
        let concurrent_exec_info = self.apply(state, general_config)?;
        self.handle_nonce(state)?;

        let (fee_transfer_info, actual_fee) = self.charge_fee(
            state,
            &concurrent_exec_info.actual_resources,
            general_config,
        )?;

        Ok(
            TransactionExecutionInfo::from_concurrent_state_execution_info(
                concurrent_exec_info,
                actual_fee,
                fee_transfer_info,
            ),
        )
    }

    fn handle_nonce<S: State + StateReader>(&self, state: &mut S) -> Result<(), TransactionError> {
        if self.version.is_zero() {
            return Ok(());
        }

        let contract_address = self.contract_address();

        let current_nonce = state.get_nonce_at(contract_address)?;

        match &self.nonce {
            None => {
                // TODO: Remove this once we have a better way to handle the nonce.
                Ok(())
            }
            Some(nonce) => {
                if nonce != &current_nonce {
                    return Err(TransactionError::InvalidTransactionNonce(
                        current_nonce.to_string(),
                        nonce.to_string(),
                    ));
                }
                state.increment_nonce(contract_address)?;
                Ok(())
            }
        }
    }
}

// ------------------------------------
//  Invoke internal functions utils
// ------------------------------------

pub fn verify_no_calls_to_other_contracts(call_info: &CallInfo) -> Result<(), TransactionError> {
    let invoked_contract_address = call_info.contract_address.clone();
    for internal_call in call_info.gen_call_topology() {
        if internal_call.contract_address != invoked_contract_address {
            return Err(TransactionError::UnauthorizedActionOnValidate);
        }
    }
    Ok(())
}

// Performs validation on fields related to function invocation transaction.
// InvokeFunction transaction.
// Deduces and returns fields required for hash calculation of

pub(crate) fn preprocess_invoke_function_fields(
    entry_point_selector: Felt252,
    nonce: Option<Felt252>,
    version: Felt252,
) -> Result<(Felt252, Vec<Felt252>), TransactionError> {
    if version.is_zero() {
        match nonce {
            Some(_) => Err(TransactionError::InvokeFunctionZeroHasNonce),
            None => {
                let additional_data = Vec::new();
                let entry_point_selector_field = entry_point_selector;
                Ok((entry_point_selector_field, additional_data))
            }
        }
    } else {
        match nonce {
            Some(n) => {
                let additional_data = vec![n];
                let entry_point_selector_field = Felt252::zero();
                Ok((entry_point_selector_field, additional_data))
            }
            None => Err(TransactionError::InvokeFunctionNonZeroMissingNonce),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        business_logic::{
            state::cached_state::CachedState, state::in_memory_state_reader::InMemoryStateReader,
        },
        services::api::contract_classes::deprecated_contract_class::ContractClass,
    };
    use num_traits::Num;
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn test_apply_specific_concurrent_changes() {
        let internal_invoke_function = InvokeFunction {
            contract_address: Address(0.into()),
            entry_point_selector: Felt252::from_str_radix(
                "112e35f48499939272000bd72eb840e502ca4c3aefa8800992e8defb746e0c9",
                16,
            )
            .unwrap(),
            entry_point_type: EntryPointType::External,
            calldata: vec![1.into(), 1.into(), 10.into()],
            tx_type: TransactionType::InvokeFunction,
            version: 0.into(),
            validate_entry_point_selector: 0.into(),
            hash_value: 0.into(),
            signature: Vec::new(),
            max_fee: 0,
            nonce: Some(0.into()),
        };

        // Instantiate CachedState
        let mut state_reader = InMemoryStateReader::default();
        // Set contract_class
        let class_hash = [1; 32];
        let contract_class =
            ContractClass::try_from(PathBuf::from("starknet_programs/fibonacci.json")).unwrap();
        // Set contact_state
        let contract_address = Address(0.into());
        let nonce = Felt252::zero();

        state_reader
            .address_to_class_hash_mut()
            .insert(contract_address.clone(), class_hash);
        state_reader
            .address_to_nonce
            .insert(contract_address, nonce);

        let mut state = CachedState::new(state_reader.clone(), None, None);

        // Initialize state.contract_classes
        state.set_contract_classes(HashMap::new()).unwrap();

        state
            .set_contract_class(&class_hash, &contract_class)
            .unwrap();

        let result = internal_invoke_function
            .apply(&mut state, &TransactionContext::default())
            .unwrap();

        assert_eq!(result.tx_type, Some(TransactionType::InvokeFunction));
        assert_eq!(
            result.call_info.as_ref().unwrap().class_hash,
            Some(class_hash)
        );
        assert_eq!(
            result.call_info.as_ref().unwrap().entry_point_selector,
            Some(internal_invoke_function.entry_point_selector)
        );
        assert_eq!(
            result.call_info.as_ref().unwrap().calldata,
            internal_invoke_function.calldata
        );
        assert_eq!(result.call_info.unwrap().retdata, vec![Felt252::new(144)]);
    }

    #[test]
    fn test_execute_specific_concurrent_changes() {
        let internal_invoke_function = InvokeFunction {
            contract_address: Address(0.into()),
            entry_point_selector: Felt252::from_str_radix(
                "112e35f48499939272000bd72eb840e502ca4c3aefa8800992e8defb746e0c9",
                16,
            )
            .unwrap(),
            entry_point_type: EntryPointType::External,
            calldata: vec![1.into(), 1.into(), 10.into()],
            tx_type: TransactionType::InvokeFunction,
            version: 0.into(),
            validate_entry_point_selector: 0.into(),
            hash_value: 0.into(),
            signature: Vec::new(),
            max_fee: 0,
            nonce: Some(0.into()),
        };

        // Instantiate CachedState
        let mut state_reader = InMemoryStateReader::default();
        // Set contract_class
        let class_hash = [1; 32];
        let contract_class =
            ContractClass::try_from(PathBuf::from("starknet_programs/fibonacci.json")).unwrap();
        // Set contact_state
        let contract_address = Address(0.into());
        let nonce = Felt252::zero();

        state_reader
            .address_to_class_hash_mut()
            .insert(contract_address.clone(), class_hash);
        state_reader
            .address_to_nonce
            .insert(contract_address, nonce);

        let mut state = CachedState::new(state_reader.clone(), None, None);

        // Initialize state.contract_classes
        state.set_contract_classes(HashMap::new()).unwrap();

        state
            .set_contract_class(&class_hash, &contract_class)
            .unwrap();

        let result = internal_invoke_function
            .execute(&mut state, &TransactionContext::default())
            .unwrap();

        assert_eq!(result.tx_type, Some(TransactionType::InvokeFunction));
        assert_eq!(
            result.call_info.as_ref().unwrap().class_hash,
            Some(class_hash)
        );
        assert_eq!(
            result.call_info.as_ref().unwrap().entry_point_selector,
            Some(internal_invoke_function.entry_point_selector)
        );
        assert_eq!(
            result.call_info.as_ref().unwrap().calldata,
            internal_invoke_function.calldata
        );
        assert_eq!(result.call_info.unwrap().retdata, vec![Felt252::new(144)]);
    }

    #[test]
    fn test_apply_invoke_entrypoint_not_found_should_fail() {
        let internal_invoke_function = InvokeFunction {
            contract_address: Address(0.into()),
            entry_point_selector: (*EXECUTE_ENTRY_POINT_SELECTOR).clone(),
            entry_point_type: EntryPointType::External,
            calldata: Vec::new(),
            tx_type: TransactionType::InvokeFunction,
            version: 0.into(),
            validate_entry_point_selector: 0.into(),
            hash_value: 0.into(),
            signature: Vec::new(),
            max_fee: 0,
            nonce: Some(0.into()),
        };

        // Instantiate CachedState
        let mut state_reader = InMemoryStateReader::default();
        // Set contract_class
        let class_hash = [1; 32];
        let contract_class =
            ContractClass::try_from(PathBuf::from("starknet_programs/amm.json")).unwrap();
        // Set contact_state
        let contract_address = Address(0.into());
        let nonce = Felt252::zero();

        state_reader
            .address_to_class_hash_mut()
            .insert(contract_address.clone(), class_hash);
        state_reader
            .address_to_nonce
            .insert(contract_address, nonce);

        let mut state = CachedState::new(state_reader.clone(), None, None);

        // Initialize state.contract_classes
        state.set_contract_classes(HashMap::new()).unwrap();

        state
            .set_contract_class(&class_hash, &contract_class)
            .unwrap();

        let expected_error =
            internal_invoke_function.apply(&mut state, &TransactionContext::default());

        assert!(expected_error.is_err());
        assert_matches!(
            expected_error.unwrap_err(),
            TransactionError::EntryPointNotFound
        );
    }

    #[test]
    fn test_apply_v0_with_no_nonce() {
        let internal_invoke_function = InvokeFunction {
            contract_address: Address(0.into()),
            entry_point_selector: Felt252::from_str_radix(
                "112e35f48499939272000bd72eb840e502ca4c3aefa8800992e8defb746e0c9",
                16,
            )
            .unwrap(),
            entry_point_type: EntryPointType::External,
            calldata: vec![1.into(), 1.into(), 10.into()],
            tx_type: TransactionType::InvokeFunction,
            version: 0.into(),
            validate_entry_point_selector: 0.into(),
            hash_value: 0.into(),
            signature: Vec::new(),
            max_fee: 0,
            nonce: None,
        };

        // Instantiate CachedState
        let mut state_reader = InMemoryStateReader::default();
        // Set contract_class
        let class_hash = [1; 32];
        let contract_class =
            ContractClass::try_from(PathBuf::from("starknet_programs/fibonacci.json")).unwrap();
        // Set contact_state
        let contract_address = Address(0.into());
        let nonce = Felt252::zero();

        state_reader
            .address_to_class_hash_mut()
            .insert(contract_address.clone(), class_hash);
        state_reader
            .address_to_nonce
            .insert(contract_address, nonce);

        let mut state = CachedState::new(state_reader.clone(), None, None);

        // Initialize state.contract_classes
        state.set_contract_classes(HashMap::new()).unwrap();

        state
            .set_contract_class(&class_hash, &contract_class)
            .unwrap();

        let result = internal_invoke_function
            .apply(&mut state, &TransactionContext::default())
            .unwrap();

        assert_eq!(result.tx_type, Some(TransactionType::InvokeFunction));
        assert_eq!(
            result.call_info.as_ref().unwrap().class_hash,
            Some(class_hash)
        );
        assert_eq!(
            result.call_info.as_ref().unwrap().entry_point_selector,
            Some(internal_invoke_function.entry_point_selector)
        );
        assert_eq!(
            result.call_info.as_ref().unwrap().calldata,
            internal_invoke_function.calldata
        );
        assert_eq!(result.call_info.unwrap().retdata, vec![Felt252::new(144)]);
    }

    #[test]
    fn test_run_validate_entrypoint_nonce_is_none_should_fail() {
        let internal_invoke_function = InvokeFunction {
            contract_address: Address(0.into()),
            entry_point_selector: (*EXECUTE_ENTRY_POINT_SELECTOR).clone(),
            entry_point_type: EntryPointType::External,
            calldata: Vec::new(),
            tx_type: TransactionType::InvokeFunction,
            version: 1.into(),
            validate_entry_point_selector: 0.into(),
            hash_value: 0.into(),
            signature: Vec::new(),
            max_fee: 0,
            nonce: None,
        };

        // Instantiate CachedState
        let mut state_reader = InMemoryStateReader::default();
        // Set contract_class
        let class_hash = [1; 32];
        let contract_class =
            ContractClass::try_from(PathBuf::from("starknet_programs/amm.json")).unwrap();
        // Set contact_state
        let contract_address = Address(0.into());
        let nonce = Felt252::zero();

        state_reader
            .address_to_class_hash_mut()
            .insert(contract_address.clone(), class_hash);
        state_reader
            .address_to_nonce
            .insert(contract_address, nonce);

        let mut state = CachedState::new(state_reader.clone(), None, None);

        // Initialize state.contract_classes
        state.set_contract_classes(HashMap::new()).unwrap();

        state
            .set_contract_class(&class_hash, &contract_class)
            .unwrap();

        let expected_error =
            internal_invoke_function.apply(&mut state, &TransactionContext::default());

        assert!(expected_error.is_err());
        assert_matches!(expected_error.unwrap_err(), TransactionError::MissingNonce);
    }

    #[test]
    // Test fee calculation is done correctly but payment to sequencer fails due to been WIP.
    fn test_execute_invoke_fee_payment_to_sequencer_should_fail() {
        let internal_invoke_function = InvokeFunction {
            contract_address: Address(0.into()),
            entry_point_selector: Felt252::from_str_radix(
                "112e35f48499939272000bd72eb840e502ca4c3aefa8800992e8defb746e0c9",
                16,
            )
            .unwrap(),
            entry_point_type: EntryPointType::External,
            calldata: vec![1.into(), 1.into(), 10.into()],
            tx_type: TransactionType::InvokeFunction,
            version: 1.into(),
            validate_entry_point_selector: 0.into(),
            hash_value: 0.into(),
            signature: Vec::new(),
            max_fee: 1000,
            nonce: Some(0.into()),
        };

        // Instantiate CachedState
        let mut state_reader = InMemoryStateReader::default();
        // Set contract_class
        let class_hash = [1; 32];
        let contract_class =
            ContractClass::try_from(PathBuf::from("starknet_programs/fibonacci.json")).unwrap();
        // Set contact_state
        let contract_address = Address(0.into());
        let nonce = Felt252::zero();

        state_reader
            .address_to_class_hash_mut()
            .insert(contract_address.clone(), class_hash);
        state_reader
            .address_to_nonce
            .insert(contract_address, nonce);

        let mut state = CachedState::new(state_reader.clone(), None, None);

        // Initialize state.contract_classes
        state.set_contract_classes(HashMap::new()).unwrap();

        state
            .set_contract_class(&class_hash, &contract_class)
            .unwrap();

        let mut config = TransactionContext::default();
        config.cairo_resource_fee_weights = HashMap::from([
            (String::from("l1_gas_usage"), 0.into()),
            (String::from("pedersen_builtin"), 16.into()),
            (String::from("range_check_builtin"), 70.into()),
        ]);

        let expected_error = internal_invoke_function.execute(&mut state, &config);
        let error_msg = "Fee transfer failure".to_string();
        assert!(expected_error.is_err());
        assert_matches!(expected_error.unwrap_err(), TransactionError::FeeError(msg) if msg == error_msg);
    }

    #[test]
    fn test_execute_invoke_actual_fee_exceeded_max_fee_should_fail() {
        let internal_invoke_function = InvokeFunction {
            contract_address: Address(0.into()),
            entry_point_selector: Felt252::from_str_radix(
                "112e35f48499939272000bd72eb840e502ca4c3aefa8800992e8defb746e0c9",
                16,
            )
            .unwrap(),
            entry_point_type: EntryPointType::External,
            calldata: vec![1.into(), 1.into(), 10.into()],
            tx_type: TransactionType::InvokeFunction,
            version: 1.into(),
            validate_entry_point_selector: 0.into(),
            hash_value: 0.into(),
            signature: Vec::new(),
            max_fee: 1000,
            nonce: Some(0.into()),
        };

        // Instantiate CachedState
        let mut state_reader = InMemoryStateReader::default();
        // Set contract_class
        let class_hash = [1; 32];
        let contract_class =
            ContractClass::try_from(PathBuf::from("starknet_programs/fibonacci.json")).unwrap();
        // Set contact_state
        let contract_address = Address(0.into());
        let nonce = Felt252::zero();

        state_reader
            .address_to_class_hash_mut()
            .insert(contract_address.clone(), class_hash);
        state_reader
            .address_to_nonce
            .insert(contract_address, nonce);

        let mut state = CachedState::new(state_reader.clone(), None, None);

        // Initialize state.contract_classes
        state.set_contract_classes(HashMap::new()).unwrap();

        state
            .set_contract_class(&class_hash, &contract_class)
            .unwrap();

        let mut config = TransactionContext::default();
        config.cairo_resource_fee_weights = HashMap::from([
            (String::from("l1_gas_usage"), 0.into()),
            (String::from("pedersen_builtin"), 16.into()),
            (String::from("range_check_builtin"), 70.into()),
        ]);
        config.starknet_os_config.gas_price = 1;

        let expected_error = internal_invoke_function.execute(&mut state, &config);
        let error_msg = "Actual fee exceeded max fee.".to_string();
        assert!(expected_error.is_err());
        assert_matches!(expected_error.unwrap_err(), TransactionError::FeeError(actual_error_msg) if actual_error_msg == error_msg);
    }

    #[test]
    fn test_execute_invoke_twice_should_fail() {
        let internal_invoke_function = InvokeFunction {
            contract_address: Address(0.into()),
            entry_point_selector: Felt252::from_str_radix(
                "112e35f48499939272000bd72eb840e502ca4c3aefa8800992e8defb746e0c9",
                16,
            )
            .unwrap(),
            entry_point_type: EntryPointType::External,
            calldata: vec![1.into(), 1.into(), 10.into()],
            tx_type: TransactionType::InvokeFunction,
            version: 1.into(),
            validate_entry_point_selector: 0.into(),
            hash_value: 0.into(),
            signature: Vec::new(),
            max_fee: 0,
            nonce: Some(0.into()),
        };

        // Instantiate CachedState
        let mut state_reader = InMemoryStateReader::default();
        // Set contract_class
        let class_hash = [1; 32];
        let contract_class =
            ContractClass::try_from(PathBuf::from("starknet_programs/fibonacci.json")).unwrap();
        // Set contact_state
        let contract_address = Address(0.into());
        let nonce = Felt252::zero();

        state_reader
            .address_to_class_hash_mut()
            .insert(contract_address.clone(), class_hash);
        state_reader
            .address_to_nonce
            .insert(contract_address, nonce);

        let mut state = CachedState::new(state_reader.clone(), None, None);

        // Initialize state.contract_classes
        state.set_contract_classes(HashMap::new()).unwrap();

        state
            .set_contract_class(&class_hash, &contract_class)
            .unwrap();

        internal_invoke_function
            .execute(&mut state, &TransactionContext::default())
            .unwrap();

        let expected_error =
            internal_invoke_function.execute(&mut state, &TransactionContext::default());

        assert!(expected_error.is_err());
        assert_matches!(
            expected_error.unwrap_err(),
            TransactionError::InvalidTransactionNonce(..)
        )
    }

    #[test]
    fn test_execute_inovoke_nonce_missing_should_fail() {
        let internal_invoke_function = InvokeFunction {
            contract_address: Address(0.into()),
            entry_point_selector: Felt252::from_str_radix(
                "112e35f48499939272000bd72eb840e502ca4c3aefa8800992e8defb746e0c9",
                16,
            )
            .unwrap(),
            entry_point_type: EntryPointType::External,
            calldata: vec![1.into(), 1.into(), 10.into()],
            tx_type: TransactionType::InvokeFunction,
            version: 1.into(),
            validate_entry_point_selector: 0.into(),
            hash_value: 0.into(),
            signature: Vec::new(),
            max_fee: 0,
            nonce: None,
        };

        // Instantiate CachedState
        let mut state_reader = InMemoryStateReader::default();
        // Set contract_class
        let class_hash = [1; 32];
        let contract_class =
            ContractClass::try_from(PathBuf::from("starknet_programs/fibonacci.json")).unwrap();
        // Set contact_state
        let contract_address = Address(0.into());
        let nonce = Felt252::zero();

        state_reader
            .address_to_class_hash_mut()
            .insert(contract_address.clone(), class_hash);
        state_reader
            .address_to_nonce
            .insert(contract_address, nonce);

        let mut state = CachedState::new(state_reader.clone(), None, None);

        // Initialize state.contract_classes
        state.set_contract_classes(HashMap::new()).unwrap();

        state
            .set_contract_class(&class_hash, &contract_class)
            .unwrap();

        let expected_error =
            internal_invoke_function.execute(&mut state, &TransactionContext::default());

        assert!(expected_error.is_err());
        assert_matches!(expected_error.unwrap_err(), TransactionError::MissingNonce)
    }

    #[test]
    fn invoke_version_zero_with_non_zero_nonce_should_fail() {
        let expected_error = preprocess_invoke_function_fields(
            Felt252::from_str_radix(
                "112e35f48499939272000bd72eb840e502ca4c3aefa8800992e8defb746e0c9",
                16,
            )
            .unwrap(),
            Some(1.into()),
            0.into(),
        )
        .unwrap_err();
        assert_matches!(expected_error, TransactionError::InvokeFunctionZeroHasNonce)
    }

    #[test]
    // the test should try to make verify_no_calls_to_other_contracts fail
    fn verify_no_calls_to_other_contracts_should_fail() {
        let mut call_info = CallInfo::default();
        let mut internal_calls = Vec::new();
        let internal_call = CallInfo {
            contract_address: Address(1.into()),
            ..Default::default()
        };
        internal_calls.push(internal_call);
        call_info.internal_calls = internal_calls;

        let expected_error = verify_no_calls_to_other_contracts(&call_info);

        assert!(expected_error.is_err());
        assert_matches!(
            expected_error.unwrap_err(),
            TransactionError::UnauthorizedActionOnValidate
        );
    }

    #[test]
    fn preprocess_invoke_function_fields_nonce_is_none() {
        let entry_point_selector = Felt252::from_str_radix(
            "112e35f48499939272000bd72eb840e502ca4c3aefa8800992e8defb746e0c9",
            16,
        )
        .unwrap();
        let result =
            preprocess_invoke_function_fields(entry_point_selector.clone(), None, 0.into());

        let expected_additional_data: Vec<Felt252> = Vec::new();
        let expected_entry_point_selector_field = entry_point_selector;
        assert_eq!(
            result.unwrap(),
            (
                expected_entry_point_selector_field,
                expected_additional_data
            )
        )
    }

    #[test]
    fn invoke_version_one_with_no_nonce_should_fail() {
        let expected_error = preprocess_invoke_function_fields(
            Felt252::from_str_radix(
                "112e35f48499939272000bd72eb840e502ca4c3aefa8800992e8defb746e0c9",
                16,
            )
            .unwrap(),
            None,
            1.into(),
        );
        assert!(expected_error.is_err());
        assert_matches!(
            expected_error.unwrap_err(),
            TransactionError::InvokeFunctionNonZeroMissingNonce
        )
    }
}

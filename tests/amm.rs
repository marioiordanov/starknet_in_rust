#![deny(warnings)]

use cairo_rs::vm::runners::cairo_runner::ExecutionResources;
use felt::Felt;
use num_traits::Zero;
use starknet_crypto::{pedersen_hash, FieldElement};
use starknet_rs::{
    business_logic::{
        execution::{
            execution_entry_point::ExecutionEntryPoint,
            objects::{CallInfo, CallType, TransactionExecutionContext},
        },
        fact_state::{
            contract_state::ContractState, in_memory_state_reader::InMemoryStateReader,
            state::ExecutionResourcesManager,
        },
        state::cached_state::CachedState,
    },
    definitions::{constants::TRANSACTION_VERSION, general_config::StarknetGeneralConfig},
    services::api::contract_class::{ContractClass, EntryPointType},
    utils::{calculate_sn_keccak, Address},
};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

#[test]
fn amm_init_pool_test() {
    // ---------------------------------------------------------
    //  Create program and entry point types for contract class
    // ---------------------------------------------------------

    let path = PathBuf::from("starknet_programs/amm.json");
    let contract_class = ContractClass::try_from(path).unwrap();
    let entry_points_by_type = contract_class.entry_points_by_type().clone();

    let amm_entrypoint_selector = entry_points_by_type
        .get(&EntryPointType::External)
        .unwrap()
        .get(4)
        .unwrap()
        .selector()
        .clone();

    //* --------------------------------------------
    //*    Create state reader with class hash data
    //* --------------------------------------------

    let mut contract_class_cache = HashMap::new();

    //  ------------ contract data --------------------

    let address = Address(1111.into());
    let class_hash = [1; 32];
    let contract_state = ContractState::new(class_hash, 3.into(), HashMap::new());

    contract_class_cache.insert(class_hash, contract_class);
    let mut state_reader = InMemoryStateReader::new(HashMap::new(), HashMap::new());
    state_reader
        .contract_states_mut()
        .insert(address.clone(), contract_state);

    //* ---------------------------------------
    //*    Create state with previous data
    //* ---------------------------------------

    let mut state = CachedState::new(state_reader, Some(contract_class_cache));

    //* ------------------------------------
    //*    Create execution entry point
    //* ------------------------------------

    let calldata = [10000.into(), 10000.into()].to_vec();
    let caller_address = Address(0000.into());
    let entry_point_type = EntryPointType::External;

    let exec_entry_point = ExecutionEntryPoint::new(
        address.clone(),
        calldata.clone(),
        amm_entrypoint_selector.clone(),
        caller_address.clone(),
        entry_point_type,
        Some(CallType::Delegate),
        Some(class_hash),
    );

    //* --------------------
    //*   Execute contract
    //* ---------------------
    let general_config = StarknetGeneralConfig::default();
    let tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt::zero(),
        Vec::new(),
        0,
        10.into(),
        general_config.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION,
    );
    let mut resources_manager = ExecutionResourcesManager::default();

    let pool_balance_hash = calculate_sn_keccak("pool_balance".as_bytes());
    let pool_balance_hash = FieldElement::from_bytes_be(&pool_balance_hash).unwrap();

    let variable_name_1: FieldElement = FieldElement::from(1u8);
    let storage_hash_1 = pedersen_hash(&pool_balance_hash, &variable_name_1);

    let variable_name_2: FieldElement = FieldElement::from(2u8);
    let storage_hash_2 = pedersen_hash(&pool_balance_hash, &variable_name_2);

    let mut accessed_storage_keys: HashSet<[u8; 32], _> = HashSet::new();

    accessed_storage_keys.insert(storage_hash_1.to_bytes_be());
    accessed_storage_keys.insert(storage_hash_2.to_bytes_be());

    let expected_call_info = CallInfo {
        caller_address: Address(0.into()),
        call_type: Some(CallType::Delegate),
        contract_address: Address(1111.into()),
        entry_point_selector: Some(amm_entrypoint_selector.clone()),
        entry_point_type: Some(EntryPointType::External),
        calldata: calldata.clone(),
        retdata: [].to_vec(),
        execution_resources: ExecutionResources::default(),
        class_hash: Some(class_hash),
        accessed_storage_keys,
        ..Default::default()
    };

    assert_eq!(
        exec_entry_point
            .execute(
                &mut state,
                &general_config,
                &mut resources_manager,
                &tx_execution_context
            )
            .unwrap(),
        expected_call_info
    );
    //GETTER ENTRY POINT SELECTOR
    let getter_entrypoint_selector = entry_points_by_type
        .get(&EntryPointType::External)
        .unwrap()
        .get(1)
        .unwrap()
        .selector()
        .clone();
    println!(
        "getter_entrypoint_selector: {:?}",
        getter_entrypoint_selector
    );

    //NEW ENTRY POINT TO GET POOL BALANCE
    let calldata_getter: Vec<Felt> = [1.into()].to_vec();
    let caller_getter_address = Address(1111.into());
    let _getter_exec_entry_point = ExecutionEntryPoint::new(
        address,
        calldata_getter.clone(),
        getter_entrypoint_selector.clone(),
        caller_getter_address.clone(),
        entry_point_type,
        Some(CallType::Delegate),
        Some(class_hash),
    );

    //EXECUTE GETTER
    let general_config = StarknetGeneralConfig::default();
    let _tx_getter_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt::zero(),
        Vec::new(),
        0,
        10.into(),
        general_config.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION,
    );
}

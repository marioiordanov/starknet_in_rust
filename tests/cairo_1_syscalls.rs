use starknet_rs::{utils::calculate_sn_keccak, SierraContractClass};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    vec,
};

use cairo_lang_starknet::casm_contract_class::CasmContractClass;
use cairo_vm::{
    felt::{felt_str, Felt252},
    vm::runners::{builtin_runner::RANGE_CHECK_BUILTIN_NAME, cairo_runner::ExecutionResources},
};
use num_bigint::BigUint;
use num_traits::{Num, One, Zero};
use starknet_contract_class::EntryPointType;
use starknet_rs::{
    definitions::{block_context::BlockContext, constants::TRANSACTION_VERSION},
    execution::{
        execution_entry_point::ExecutionEntryPoint, CallInfo, CallType, OrderedEvent,
        OrderedL2ToL1Message, TransactionExecutionContext,
    },
    services::api::contract_classes::{
        compiled_class::CompiledClass, deprecated_contract_class::ContractClass,
    },
    state::{cached_state::CachedState, state_api::StateReader},
    state::{in_memory_state_reader::InMemoryStateReader, ExecutionResourcesManager},
    utils::{Address, ClassHash},
};

#[test]
fn storage_write_read() {
    //  Create program and entry point types for contract class
    let program_data = include_bytes!("../starknet_programs/cairo1/simple_wallet.casm");
    let contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = contract_class.clone().entry_points_by_type;
    let constructor_entrypoint_selector = &entrypoints.constructor.get(0).unwrap().selector;
    let get_balance_entrypoint_selector = &entrypoints.external.get(1).unwrap().selector;
    let increase_balance_entrypoint_selector = &entrypoints.external.get(0).unwrap().selector;

    // Create state reader with class hash data
    let mut contract_class_cache = HashMap::new();

    let address = Address(1111.into());
    let class_hash: ClassHash = [1; 32];
    let nonce = Felt252::zero();

    contract_class_cache.insert(class_hash, contract_class);
    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(state_reader, None, Some(contract_class_cache));

    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );

    let mut resources_manager = ExecutionResourcesManager::default();

    let create_execute_extrypoint = |selector: &BigUint,
                                     calldata: Vec<Felt252>,
                                     entry_point_type: EntryPointType|
     -> ExecutionEntryPoint {
        ExecutionEntryPoint::new(
            address.clone(),
            calldata,
            Felt252::new(selector.clone()),
            Address(0000.into()),
            entry_point_type,
            Some(CallType::Delegate),
            Some(class_hash),
            100000,
        )
    };

    // RUN CONSTRUCTOR
    // Create an execution entry point
    let calldata = [25.into()].to_vec();
    let constructor_exec_entry_point = create_execute_extrypoint(
        constructor_entrypoint_selector,
        calldata,
        EntryPointType::Constructor,
    );

    // Run constructor entrypoint
    constructor_exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();

    // RUN GET_BALANCE
    // Create an execution entry point
    let calldata = [].to_vec();
    let get_balance_exec_entry_point = create_execute_extrypoint(
        get_balance_entrypoint_selector,
        calldata,
        EntryPointType::External,
    );

    // Run get_balance entrypoint
    let call_info = get_balance_exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();
    assert_eq!(call_info.retdata, [25.into()]);

    // RUN INCREASE_BALANCE
    // Create an execution entry point
    let calldata = [100.into()].to_vec();
    let increase_balance_entry_point = create_execute_extrypoint(
        increase_balance_entrypoint_selector,
        calldata,
        EntryPointType::External,
    );

    // Run increase_balance entrypoint
    increase_balance_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();

    // RUN GET_BALANCE
    // Create an execution entry point
    let calldata = [].to_vec();
    let get_balance_exec_entry_point = create_execute_extrypoint(
        get_balance_entrypoint_selector,
        calldata,
        EntryPointType::External,
    );

    // Run get_balance entrypoint
    let call_info = get_balance_exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();
    assert_eq!(call_info.retdata, [125.into()])
}

#[test]
fn library_call() {
    //  Create program and entry point types for contract class
    let program_data = include_bytes!("../starknet_programs/cairo1/square_root.casm");
    let contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = contract_class.clone().entry_points_by_type;
    let entrypoint_selector = &entrypoints.external.get(0).unwrap().selector;

    // Create state reader with class hash data
    let mut contract_class_cache = HashMap::new();

    let address = Address(1111.into());
    let class_hash: ClassHash = [1; 32];
    let nonce = Felt252::zero();

    contract_class_cache.insert(class_hash, contract_class);
    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce);

    // Add lib contract to the state

    let lib_program_data = include_bytes!("../starknet_programs/cairo1/math_lib.casm");
    let lib_contract_class: CasmContractClass = serde_json::from_slice(lib_program_data).unwrap();

    let lib_address = Address(1112.into());
    let lib_class_hash: ClassHash = [2; 32];
    let lib_nonce = Felt252::zero();

    contract_class_cache.insert(lib_class_hash, lib_contract_class);
    state_reader
        .address_to_class_hash_mut()
        .insert(lib_address.clone(), lib_class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(lib_address, lib_nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(state_reader, None, Some(contract_class_cache));

    // Create an execution entry point
    let calldata = [25.into(), Felt252::from_bytes_be(&lib_class_hash)].to_vec();
    let caller_address = Address(0000.into());
    let entry_point_type = EntryPointType::External;

    let exec_entry_point = ExecutionEntryPoint::new(
        address,
        calldata.clone(),
        Felt252::new(entrypoint_selector.clone()),
        caller_address,
        entry_point_type,
        Some(CallType::Delegate),
        Some(class_hash),
        100000,
    );

    // Execute the entrypoint
    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );
    let mut resources_manager = ExecutionResourcesManager::default();
    let expected_execution_resources = ExecutionResources {
        n_steps: 259,
        n_memory_holes: 10,
        builtin_instance_counter: HashMap::from([(RANGE_CHECK_BUILTIN_NAME.to_string(), 12)]),
    };
    let expected_execution_resources_internal_call = ExecutionResources {
        n_steps: 85,
        n_memory_holes: 6,
        builtin_instance_counter: HashMap::from([(RANGE_CHECK_BUILTIN_NAME.to_string(), 7)]),
    };

    // expected results
    let expected_call_info = CallInfo {
        caller_address: Address(0.into()),
        call_type: Some(CallType::Delegate),
        contract_address: Address(1111.into()),
        entry_point_selector: Some(Felt252::new(entrypoint_selector)),
        entry_point_type: Some(EntryPointType::External),
        calldata,
        retdata: [5.into()].to_vec(),
        execution_resources: expected_execution_resources,
        class_hash: Some(class_hash),
        internal_calls: vec![CallInfo {
            caller_address: Address(0.into()),
            call_type: Some(CallType::Delegate),
            contract_address: Address(1111.into()),
            entry_point_selector: Some(
                Felt252::from_str_radix(
                    "544923964202674311881044083303061611121949089655923191939299897061511784662",
                    10,
                )
                .unwrap(),
            ),
            entry_point_type: Some(EntryPointType::External),
            calldata: vec![25.into()],
            retdata: [5.into()].to_vec(),
            execution_resources: expected_execution_resources_internal_call,
            class_hash: Some(lib_class_hash),
            gas_consumed: 0,
            ..Default::default()
        }],
        code_address: None,
        events: vec![],
        l2_to_l1_messages: vec![],
        storage_read_values: vec![],
        accessed_storage_keys: HashSet::new(),
        gas_consumed: 78980,
        ..Default::default()
    };

    assert_eq!(
        exec_entry_point
            .execute(
                &mut state,
                &block_context,
                &mut resources_manager,
                &mut tx_execution_context,
                false,
            )
            .unwrap(),
        expected_call_info
    );
}

#[test]
fn call_contract_storage_write_read() {
    //  Create program and entry point types for contract class
    let program_data = include_bytes!("../starknet_programs/cairo1/wallet_wrapper.casm");
    let contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = contract_class.clone().entry_points_by_type;
    let get_balance_entrypoint_selector = &entrypoints.external.get(1).unwrap().selector;
    let increase_balance_entrypoint_selector = &entrypoints.external.get(0).unwrap().selector;

    // Create state reader with class hash data
    let mut contract_class_cache = HashMap::new();

    let address = Address(1111.into());
    let class_hash: ClassHash = [1; 32];
    let nonce = Felt252::zero();

    contract_class_cache.insert(class_hash, contract_class);
    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce);

    // Add simple_wallet contract to the state

    let simple_wallet_program_data =
        include_bytes!("../starknet_programs/cairo1/simple_wallet.casm");
    let simple_wallet_contract_class: CasmContractClass =
        serde_json::from_slice(simple_wallet_program_data).unwrap();
    let simple_wallet_constructor_entrypoint_selector = simple_wallet_contract_class
        .entry_points_by_type
        .constructor
        .get(0)
        .unwrap()
        .selector
        .clone();

    let simple_wallet_address = Address(1112.into());
    let simple_wallet_class_hash: ClassHash = [2; 32];
    let simple_wallet_nonce = Felt252::zero();

    contract_class_cache.insert(simple_wallet_class_hash, simple_wallet_contract_class);
    state_reader
        .address_to_class_hash_mut()
        .insert(simple_wallet_address.clone(), simple_wallet_class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(simple_wallet_address.clone(), simple_wallet_nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(state_reader, None, Some(contract_class_cache));

    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );

    let mut resources_manager = ExecutionResourcesManager::default();

    let create_execute_extrypoint = |selector: &BigUint,
                                     calldata: Vec<Felt252>,
                                     entry_point_type: EntryPointType,
                                     class_hash: [u8; 32],
                                     address: Address|
     -> ExecutionEntryPoint {
        ExecutionEntryPoint::new(
            address,
            calldata,
            Felt252::new(selector.clone()),
            Address(0000.into()),
            entry_point_type,
            Some(CallType::Delegate),
            Some(class_hash),
            u64::MAX.into(),
        )
    };

    // RUN SIMPLE_WALLET CONSTRUCTOR
    // Create an execution entry point
    let calldata = [25.into()].to_vec();
    let constructor_exec_entry_point = create_execute_extrypoint(
        &simple_wallet_constructor_entrypoint_selector,
        calldata,
        EntryPointType::Constructor,
        simple_wallet_class_hash,
        simple_wallet_address.clone(),
    );

    // Run constructor entrypoint
    constructor_exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();

    // RUN GET_BALANCE
    // Create an execution entry point
    let calldata = [simple_wallet_address.0.clone()].to_vec();
    let get_balance_exec_entry_point = create_execute_extrypoint(
        get_balance_entrypoint_selector,
        calldata,
        EntryPointType::External,
        class_hash,
        address.clone(),
    );

    // Run get_balance entrypoint
    let call_info = get_balance_exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();
    assert_eq!(call_info.retdata, [25.into()]);

    // RUN INCREASE_BALANCE
    // Create an execution entry point
    let calldata = [100.into(), simple_wallet_address.0.clone()].to_vec();
    let increase_balance_entry_point = create_execute_extrypoint(
        increase_balance_entrypoint_selector,
        calldata,
        EntryPointType::External,
        class_hash,
        address.clone(),
    );

    // Run increase_balance entrypoint
    increase_balance_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();

    // RUN GET_BALANCE
    // Create an execution entry point
    let calldata = [simple_wallet_address.0].to_vec();
    let get_balance_exec_entry_point = create_execute_extrypoint(
        get_balance_entrypoint_selector,
        calldata,
        EntryPointType::External,
        class_hash,
        address,
    );

    // Run get_balance entrypoint
    let call_info = get_balance_exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();
    assert_eq!(call_info.retdata, [125.into()])
}

#[test]
fn emit_event() {
    //  Create program and entry point types for contract class
    let program_data = include_bytes!("../starknet_programs/cairo1/emit_event.casm");
    let contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = contract_class.clone().entry_points_by_type;
    let entrypoint_selector = &entrypoints.external.get(0).unwrap().selector;

    // Create state reader with class hash data
    let mut contract_class_cache = HashMap::new();

    let address = Address(1111.into());
    let class_hash: ClassHash = [1; 32];
    let nonce = Felt252::zero();

    contract_class_cache.insert(class_hash, contract_class);
    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(state_reader, None, Some(contract_class_cache));

    // Create an execution entry point
    let calldata = [].to_vec();
    let caller_address = Address(0000.into());
    let entry_point_type = EntryPointType::External;

    let exec_entry_point = ExecutionEntryPoint::new(
        address,
        calldata,
        Felt252::new(entrypoint_selector.clone()),
        caller_address,
        entry_point_type,
        Some(CallType::Delegate),
        Some(class_hash),
        100000,
    );

    // Execute the entrypoint
    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );
    let mut resources_manager = ExecutionResourcesManager::default();
    let call_info = exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();
    assert_eq!(
        call_info.events,
        vec![
            OrderedEvent {
                order: 0,
                keys: vec![Felt252::from_str_radix(
                    "1533133552972353850845856330693290141476612241335297758062928121906575244541",
                    10
                )
                .unwrap()],
                data: vec![1.into()]
            },
            OrderedEvent {
                order: 1,
                keys: vec![Felt252::from_str_radix(
                    "1533133552972353850845856330693290141476612241335297758062928121906575244541",
                    10
                )
                .unwrap()],
                data: vec![2.into()]
            },
            OrderedEvent {
                order: 2,
                keys: vec![Felt252::from_str_radix(
                    "1533133552972353850845856330693290141476612241335297758062928121906575244541",
                    10
                )
                .unwrap()],
                data: vec![3.into()]
            }
        ]
    )
}

#[test]
fn deploy_cairo1_from_cairo1() {
    // data to deploy
    let test_class_hash: ClassHash = [2; 32];
    let test_felt_hash = Felt252::from_bytes_be(&test_class_hash);
    let salt = Felt252::zero();
    let test_data = include_bytes!("../starknet_programs/cairo1/contract_a.casm");
    let test_contract_class: CasmContractClass = serde_json::from_slice(test_data).unwrap();

    // Create the deploy contract class
    let program_data = include_bytes!("../starknet_programs/cairo1/deploy.casm");
    let contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = contract_class.clone().entry_points_by_type;
    let entrypoint_selector = &entrypoints.external.get(0).unwrap().selector;

    // Create state reader with class hash data
    let mut contract_class_cache = HashMap::new();

    let address = Address(1111.into());
    let class_hash: ClassHash = [1; 32];
    let nonce = Felt252::zero();

    contract_class_cache.insert(class_hash, contract_class);
    contract_class_cache.insert(test_class_hash, test_contract_class.clone());

    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(state_reader, None, Some(contract_class_cache));

    // arguments of deploy contract
    let calldata: Vec<_> = [test_felt_hash, salt].to_vec();

    // set up remaining structures

    let caller_address = Address(0000.into());
    let entry_point_type = EntryPointType::External;

    let exec_entry_point = ExecutionEntryPoint::new(
        address,
        calldata,
        Felt252::new(entrypoint_selector.clone()),
        caller_address,
        entry_point_type,
        Some(CallType::Delegate),
        Some(class_hash),
        100_000_000,
    );

    // Execute the entrypoint
    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );
    let mut resources_manager = ExecutionResourcesManager::default();

    let call_info = exec_entry_point.execute(
        &mut state,
        &block_context,
        &mut resources_manager,
        &mut tx_execution_context,
        false,
    );

    assert!(call_info.is_ok());

    let ret_address = Address(felt_str!(
        "619464431559909356793718633071398796109800070568878623926447195121629120356"
    ));

    let ret_class_hash = state.get_class_hash_at(&ret_address).unwrap();
    let ret_casm_class = match state.get_contract_class(&ret_class_hash).unwrap() {
        CompiledClass::Casm(class) => *class,
        CompiledClass::Deprecated(_) => unreachable!(),
    };

    assert_eq!(ret_casm_class, test_contract_class);
}

#[test]
fn deploy_cairo0_from_cairo1_without_constructor() {
    // data to deploy
    let test_class_hash: ClassHash = [2; 32];
    let test_felt_hash = Felt252::from_bytes_be(&test_class_hash);
    let salt = Felt252::zero();
    let contract_path = Path::new("starknet_programs/fibonacci.json");
    let test_contract_class: ContractClass =
        ContractClass::try_from(contract_path.to_path_buf()).unwrap();

    // Create the deploy contract class
    let program_data =
        include_bytes!("../starknet_programs/cairo1/deploy_without_constructor.casm");
    let contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = contract_class.clone().entry_points_by_type;
    let entrypoint_selector = &entrypoints.external.get(0).unwrap().selector;

    // Create state reader with class hash data
    let mut casm_contract_class_cache = HashMap::new();
    let mut contract_class_cache = HashMap::new();

    let address = Address(1111.into());
    let class_hash: ClassHash = [1; 32];
    let nonce = Felt252::zero();

    casm_contract_class_cache.insert(class_hash, contract_class);
    contract_class_cache.insert(test_class_hash, test_contract_class.clone());

    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(
        state_reader,
        Some(contract_class_cache),
        Some(casm_contract_class_cache),
    );

    // arguments of deploy contract
    let calldata: Vec<_> = [test_felt_hash, salt].to_vec();

    // set up remaining structures

    let caller_address = Address(0000.into());
    let entry_point_type = EntryPointType::External;

    let exec_entry_point = ExecutionEntryPoint::new(
        address,
        calldata,
        Felt252::new(entrypoint_selector.clone()),
        caller_address,
        entry_point_type,
        Some(CallType::Delegate),
        Some(class_hash),
        100_000_000,
    );

    // Execute the entrypoint
    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );
    let mut resources_manager = ExecutionResourcesManager::default();

    let call_info = exec_entry_point.execute(
        &mut state,
        &block_context,
        &mut resources_manager,
        &mut tx_execution_context,
        false,
    );

    assert!(call_info.is_ok());

    let ret_address = Address(felt_str!(
        "3326516449409112130211257005742850249535379011750934837578774621442000311202"
    ));

    let ret_class_hash = state.get_class_hash_at(&ret_address).unwrap();
    let ret_casm_class = match state.get_contract_class(&ret_class_hash).unwrap() {
        CompiledClass::Deprecated(class) => *class,
        CompiledClass::Casm(_) => unreachable!(),
    };

    assert_eq!(ret_casm_class, test_contract_class);
}

#[test]
fn deploy_cairo0_from_cairo1_with_constructor() {
    // data to deploy
    let test_class_hash: ClassHash = [2; 32];
    let test_felt_hash = Felt252::from_bytes_be(&test_class_hash);
    let salt = Felt252::zero();
    let contract_path = Path::new("starknet_programs/test_contract.json");
    let test_contract_class: ContractClass =
        ContractClass::try_from(contract_path.to_path_buf()).unwrap();

    // Create the deploy contract class
    let program_data = include_bytes!("../starknet_programs/cairo1/deploy_with_constructor.casm");
    let contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = contract_class.clone().entry_points_by_type;
    let entrypoint_selector = &entrypoints.external.get(0).unwrap().selector;

    // Create state reader with class hash data
    let mut casm_contract_class_cache = HashMap::new();
    let mut contract_class_cache = HashMap::new();

    let address = Address(1111.into());
    let class_hash: ClassHash = [1; 32];
    let nonce = Felt252::zero();

    // simulate contract declare
    casm_contract_class_cache.insert(class_hash, contract_class);
    contract_class_cache.insert(test_class_hash, test_contract_class.clone());

    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(
        state_reader,
        Some(contract_class_cache),
        Some(casm_contract_class_cache),
    );

    // arguments of deploy contract
    let calldata: Vec<_> = [test_felt_hash, salt, address.0.clone(), Felt252::zero()].to_vec();

    // set up remaining structures

    let caller_address = Address(0000.into());
    let entry_point_type = EntryPointType::External;

    let exec_entry_point = ExecutionEntryPoint::new(
        address,
        calldata,
        Felt252::new(entrypoint_selector.clone()),
        caller_address,
        entry_point_type,
        Some(CallType::Delegate),
        Some(class_hash),
        100_000_000,
    );

    // Execute the entrypoint
    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );
    let mut resources_manager = ExecutionResourcesManager::default();

    let call_info = exec_entry_point.execute(
        &mut state,
        &block_context,
        &mut resources_manager,
        &mut tx_execution_context,
        false,
    );

    assert!(call_info.is_ok());

    let ret_address = Address(felt_str!(
        "2981367321579044137695643605491580626686793431687828656373743652416610344312"
    ));

    let ret_class_hash = state.get_class_hash_at(&ret_address).unwrap();
    let ret_casm_class = match state.get_contract_class(&ret_class_hash).unwrap() {
        CompiledClass::Deprecated(class) => *class,
        CompiledClass::Casm(_) => unreachable!(),
    };

    assert_eq!(ret_casm_class, test_contract_class);
}

#[test]
fn deploy_cairo0_and_invoke() {
    // data to deploy
    let test_class_hash: ClassHash = [2; 32];
    let test_felt_hash = Felt252::from_bytes_be(&test_class_hash);
    let salt = Felt252::zero();
    let contract_path = Path::new("starknet_programs/factorial.json");
    let test_contract_class: ContractClass =
        ContractClass::try_from(contract_path.to_path_buf()).unwrap();

    // Create the deploy contract class
    let program_data =
        include_bytes!("../starknet_programs/cairo1/deploy_without_constructor.casm");
    let contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = contract_class.clone().entry_points_by_type;
    let entrypoint_selector = &entrypoints.external.get(0).unwrap().selector;

    // Create state reader with class hash data
    let mut casm_contract_class_cache = HashMap::new();
    let mut contract_class_cache = HashMap::new();

    let address = Address(1111.into());
    let class_hash: ClassHash = [1; 32];
    let nonce = Felt252::zero();

    casm_contract_class_cache.insert(class_hash, contract_class);
    contract_class_cache.insert(test_class_hash, test_contract_class.clone());

    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(
        state_reader,
        Some(contract_class_cache),
        Some(casm_contract_class_cache),
    );

    // arguments of deploy contract
    let calldata: Vec<_> = [test_felt_hash, salt].to_vec();

    // set up remaining structures

    let caller_address = Address(0000.into());
    let entry_point_type = EntryPointType::External;

    let exec_entry_point = ExecutionEntryPoint::new(
        address,
        calldata,
        Felt252::new(entrypoint_selector.clone()),
        caller_address.clone(),
        entry_point_type,
        Some(CallType::Delegate),
        Some(class_hash),
        100_000_000,
    );

    // Execute the entrypoint
    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );
    let mut resources_manager = ExecutionResourcesManager::default();

    let call_info = exec_entry_point.execute(
        &mut state,
        &block_context,
        &mut resources_manager,
        &mut tx_execution_context,
        false,
    );

    assert!(call_info.is_ok());

    let ret_address = Address(felt_str!(
        "3326516449409112130211257005742850249535379011750934837578774621442000311202"
    ));

    let ret_class_hash = state.get_class_hash_at(&ret_address).unwrap();
    let ret_casm_class = match state.get_contract_class(&ret_class_hash).unwrap() {
        CompiledClass::Deprecated(class) => *class,
        CompiledClass::Casm(_) => unreachable!(),
    };

    assert_eq!(ret_casm_class, test_contract_class);

    // invoke factorial

    let calldata = [3.into()].to_vec();
    let selector =
        felt_str!("1554360238305724106620514039016755337737024783182305317707426109255385571750");

    let exec_entry_point = ExecutionEntryPoint::new(
        ret_address,
        calldata,
        selector,
        caller_address,
        entry_point_type,
        Some(CallType::Delegate),
        Some(test_class_hash),
        100_000_000,
    );

    let call_info = exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();

    let retdata = call_info.retdata;

    // expected result 3! = 6
    assert_eq!(retdata, [6.into()].to_vec());
}

#[test]
fn test_send_message_to_l1_syscall() {
    //  Create program and entry point types for contract class
    let program_data = include_bytes!("../starknet_programs/cairo1/send_message_to_l1.casm");
    let contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = contract_class.clone().entry_points_by_type;
    let external_entrypoint_selector = &entrypoints.external.get(0).unwrap().selector;

    // Create state reader with class hash data
    let mut contract_class_cache = HashMap::new();

    let address = Address(1111.into());
    let class_hash: ClassHash = [1; 32];
    let nonce = Felt252::zero();

    contract_class_cache.insert(class_hash, contract_class);

    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(state_reader, None, Some(contract_class_cache));

    let create_execute_extrypoint = |selector: &BigUint,
                                     calldata: Vec<Felt252>,
                                     entry_point_type: EntryPointType|
     -> ExecutionEntryPoint {
        ExecutionEntryPoint::new(
            address.clone(),
            calldata,
            Felt252::new(selector.clone()),
            Address(0000.into()),
            entry_point_type,
            Some(CallType::Delegate),
            Some(class_hash),
            100000,
        )
    };

    // RUN SEND_MSG
    // Create an execution entry point
    let send_message_exec_entry_point = create_execute_extrypoint(
        external_entrypoint_selector,
        vec![],
        EntryPointType::External,
    );

    // Execute the entrypoint
    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );
    let mut resources_manager = ExecutionResourcesManager::default();

    // Run send_msg entrypoint
    let call_info = send_message_exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();

    let l2_to_l1_messages = vec![OrderedL2ToL1Message {
        order: 0,
        to_address: Address(444.into()),
        payload: vec![555.into(), 666.into()],
    }];

    let expected_execution_resources = ExecutionResources {
        n_steps: 50,
        n_memory_holes: 1,
        builtin_instance_counter: HashMap::from([(RANGE_CHECK_BUILTIN_NAME.to_string(), 2)]),
    };

    let expected_call_info = CallInfo {
        caller_address: Address(0.into()),
        call_type: Some(CallType::Delegate),
        contract_address: address.clone(),
        class_hash: Some(class_hash),
        entry_point_selector: Some(external_entrypoint_selector.into()),
        entry_point_type: Some(EntryPointType::External),
        l2_to_l1_messages,
        execution_resources: expected_execution_resources,
        gas_consumed: 10040,
        ..Default::default()
    };

    assert_eq!(call_info, expected_call_info);
}

#[test]
fn test_get_execution_info() {
    //  Create program and entry point types for contract class
    let program_data = include_bytes!("../starknet_programs/cairo1/get_execution_info.casm");
    let contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = contract_class.clone().entry_points_by_type;
    let external_entrypoint_selector = &entrypoints.external.get(0).unwrap().selector;

    // Create state reader with class hash data
    let mut contract_class_cache = HashMap::new();

    let address = Address(1111.into());
    let class_hash: ClassHash = [1; 32];
    let nonce = Felt252::zero();

    contract_class_cache.insert(class_hash, contract_class);
    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(state_reader, None, Some(contract_class_cache));

    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        vec![22.into(), 33.into()],
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );

    let mut resources_manager = ExecutionResourcesManager::default();

    let create_execute_extrypoint = |selector: &BigUint,
                                     calldata: Vec<Felt252>,
                                     entry_point_type: EntryPointType|
     -> ExecutionEntryPoint {
        ExecutionEntryPoint::new(
            address.clone(),
            calldata,
            Felt252::new(selector.clone()),
            Address(0000.into()),
            entry_point_type,
            Some(CallType::Delegate),
            Some(class_hash),
            100000,
        )
    };

    // RUN GET_INFO
    // Create an execution entry point
    let get_info_exec_entry_point = create_execute_extrypoint(
        external_entrypoint_selector,
        vec![],
        EntryPointType::External,
    );

    // Run send_msg entrypoint
    let call_info = get_info_exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();

    let expected_ret_data = vec![
        block_context.block_info().sequencer_address.0.clone(),
        0.into(),
        0.into(),
        address.0.clone(),
    ];

    let expected_execution_resources = ExecutionResources {
        n_steps: 355,
        n_memory_holes: 14,
        builtin_instance_counter: HashMap::from([(RANGE_CHECK_BUILTIN_NAME.to_string(), 4)]),
    };

    let expected_call_info = CallInfo {
        caller_address: Address(0.into()),
        call_type: Some(CallType::Delegate),
        contract_address: address.clone(),
        class_hash: Some(class_hash),
        entry_point_selector: Some(external_entrypoint_selector.into()),
        entry_point_type: Some(EntryPointType::External),
        retdata: expected_ret_data,
        execution_resources: expected_execution_resources,
        gas_consumed: 38180,
        ..Default::default()
    };

    assert_eq!(call_info, expected_call_info);
}

#[test]
fn replace_class_internal() {
    // This test only checks that the contract is updated in the storage, see `replace_class_contract_call`
    //  Create program and entry point types for contract class
    let program_data_a = include_bytes!("../starknet_programs/cairo1/get_number_a.casm");
    let contract_class_a: CasmContractClass = serde_json::from_slice(program_data_a).unwrap();
    let entrypoints_a = contract_class_a.clone().entry_points_by_type;
    let upgrade_selector = &entrypoints_a.external.get(0).unwrap().selector;

    // Create state reader with class hash data
    let mut contract_class_cache = HashMap::new();

    let address = Address(1111.into());
    let class_hash_a: ClassHash = [1; 32];
    let nonce = Felt252::zero();

    contract_class_cache.insert(class_hash_a, contract_class_a);
    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash_a);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce);

    // Add get_number_b contract to the state (only its contract_class)

    let program_data_b = include_bytes!("../starknet_programs/cairo1/get_number_b.casm");
    let contract_class_b: CasmContractClass = serde_json::from_slice(program_data_b).unwrap();

    let class_hash_b: ClassHash = [2; 32];

    contract_class_cache.insert(class_hash_b, contract_class_b.clone());

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(state_reader, None, Some(contract_class_cache));

    // Run upgrade entrypoint and check that the storage was updated with the new contract class
    // Create an execution entry point
    let calldata = [Felt252::from_bytes_be(&class_hash_b)].to_vec();
    let caller_address = Address(0000.into());
    let entry_point_type = EntryPointType::External;

    let exec_entry_point = ExecutionEntryPoint::new(
        address.clone(),
        calldata,
        Felt252::new(upgrade_selector.clone()),
        caller_address,
        entry_point_type,
        Some(CallType::Delegate),
        Some(class_hash_a),
        100000,
    );

    // Execute the entrypoint
    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );
    let mut resources_manager = ExecutionResourcesManager::default();

    exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();
    // Check that the class was indeed replaced in storage
    assert_eq!(state.get_class_hash_at(&address).unwrap(), class_hash_b);
    // Check that the class_hash_b leads to contract_class_b for soundness
    assert_eq!(
        state.get_contract_class(&class_hash_b).unwrap(),
        CompiledClass::Casm(Box::new(contract_class_b))
    );
}

#[test]
fn replace_class_contract_call() {
    /* Test Outline:
       - Add `get_number_a.cairo` contract at address 2 and `get_number_b.cairo` contract without an address
       - Call `get_number` function of `get_number_wrapper.cairo` and expect to get an answer from `get_number_a` (25)
       - Call `upgrade` function of `get_number_wrapper.cairo` with `get_number_b.cairo`'s class_hash
       - Call `get_number` function of `get_number_wrapper.cairo` and expect to get an answer from `get_number_b` (17)
    */

    // SET GET_NUMBER_A
    // Add get_number_a.cairo to storage
    let program_data = include_bytes!("../starknet_programs/cairo1/get_number_a.casm");
    let contract_class_a: CasmContractClass = serde_json::from_slice(program_data).unwrap();

    // Create state reader with class hash data
    let mut contract_class_cache = HashMap::new();

    let address = Address(Felt252::one());
    let class_hash_a: ClassHash = [1; 32];
    let nonce = Felt252::zero();

    contract_class_cache.insert(class_hash_a, contract_class_a);
    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash_a);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce.clone());

    // SET GET_NUMBER_B

    // Add get_number_b contract to the state (only its contract_class)

    let program_data = include_bytes!("../starknet_programs/cairo1/get_number_b.casm");
    let contract_class_b: CasmContractClass = serde_json::from_slice(program_data).unwrap();

    let class_hash_b: ClassHash = [2; 32];

    contract_class_cache.insert(class_hash_b, contract_class_b);

    // SET GET_NUMBER_WRAPPER

    //  Create program and entry point types for contract class
    let program_data = include_bytes!("../starknet_programs/cairo1/get_number_wrapper.casm");
    let wrapper_contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = wrapper_contract_class.clone().entry_points_by_type;
    let get_number_entrypoint_selector = &entrypoints.external.get(1).unwrap().selector;
    let upgrade_entrypoint_selector = &entrypoints.external.get(0).unwrap().selector;

    let wrapper_address = Address(Felt252::from(2));
    let wrapper_class_hash: ClassHash = [3; 32];

    contract_class_cache.insert(wrapper_class_hash, wrapper_contract_class);
    state_reader
        .address_to_class_hash_mut()
        .insert(wrapper_address.clone(), wrapper_class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(wrapper_address, nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(state_reader, None, Some(contract_class_cache));

    // INITIALIZE STARKNET CONFIG
    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );
    let mut resources_manager = ExecutionResourcesManager::default();

    // CALL GET_NUMBER BEFORE REPLACE_CLASS

    let calldata = [].to_vec();
    let caller_address = Address(0000.into());
    let entry_point_type = EntryPointType::External;

    let exec_entry_point = ExecutionEntryPoint::new(
        address.clone(),
        calldata,
        Felt252::new(get_number_entrypoint_selector.clone()),
        caller_address.clone(),
        entry_point_type,
        Some(CallType::Delegate),
        Some(wrapper_class_hash),
        100000,
    );

    let result = exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();
    assert_eq!(result.retdata, vec![25.into()]);

    // REPLACE_CLASS

    let calldata = [Felt252::from_bytes_be(&class_hash_b)].to_vec();

    let exec_entry_point = ExecutionEntryPoint::new(
        address.clone(),
        calldata,
        Felt252::new(upgrade_entrypoint_selector.clone()),
        caller_address.clone(),
        entry_point_type,
        Some(CallType::Delegate),
        Some(wrapper_class_hash),
        100000,
    );

    exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();

    // CALL GET_NUMBER AFTER REPLACE_CLASS

    let calldata = [].to_vec();

    let exec_entry_point = ExecutionEntryPoint::new(
        address,
        calldata,
        Felt252::new(get_number_entrypoint_selector.clone()),
        caller_address,
        entry_point_type,
        Some(CallType::Delegate),
        Some(wrapper_class_hash),
        100000,
    );

    let result = exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();
    assert_eq!(result.retdata, vec![17.into()]);
}

#[test]
fn replace_class_contract_call_same_transaction() {
    /* Test Outline:
       - Add `get_number_a.cairo` contract at address 2 and `get_number_b.cairo` contract without an address
       - Call `get_numbers_old_new` function of `get_number_wrapper.cairo` and expect to get both answers from `get_number_a`, and 'get_number_b' (25, 17)
    */

    // SET GET_NUMBER_A
    // Add get_number_a.cairo to storage
    let program_data = include_bytes!("../starknet_programs/cairo1/get_number_a.casm");
    let contract_class_a: CasmContractClass = serde_json::from_slice(program_data).unwrap();

    // Create state reader with class hash data
    let mut contract_class_cache = HashMap::new();

    let address = Address(Felt252::one());
    let class_hash_a: ClassHash = [1; 32];
    let nonce = Felt252::zero();

    contract_class_cache.insert(class_hash_a, contract_class_a);
    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash_a);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce.clone());

    // SET GET_NUMBER_B

    // Add get_number_b contract to the state (only its contract_class)

    let program_data = include_bytes!("../starknet_programs/cairo1/get_number_b.casm");
    let contract_class_b: CasmContractClass = serde_json::from_slice(program_data).unwrap();

    let class_hash_b: ClassHash = [2; 32];

    contract_class_cache.insert(class_hash_b, contract_class_b);

    // SET GET_NUMBER_WRAPPER

    //  Create program and entry point types for contract class
    let program_data = include_bytes!("../starknet_programs/cairo1/get_number_wrapper.casm");
    let wrapper_contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = wrapper_contract_class.clone().entry_points_by_type;
    let get_numbers_entrypoint_selector = &entrypoints.external.get(2).unwrap().selector;

    let wrapper_address = Address(Felt252::from(2));
    let wrapper_class_hash: ClassHash = [3; 32];

    contract_class_cache.insert(wrapper_class_hash, wrapper_contract_class);
    state_reader
        .address_to_class_hash_mut()
        .insert(wrapper_address.clone(), wrapper_class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(wrapper_address, nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(state_reader, None, Some(contract_class_cache));

    // INITIALIZE STARKNET CONFIG
    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );
    let mut resources_manager = ExecutionResourcesManager::default();

    // CALL GET_NUMBERS_OLD_NEW

    let calldata = [Felt252::from_bytes_be(&class_hash_b)].to_vec();
    let caller_address = Address(0000.into());
    let entry_point_type = EntryPointType::External;

    let exec_entry_point = ExecutionEntryPoint::new(
        address,
        calldata,
        Felt252::new(get_numbers_entrypoint_selector.clone()),
        caller_address,
        entry_point_type,
        Some(CallType::Delegate),
        Some(wrapper_class_hash),
        u64::MAX.into(),
    );

    let result = exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();
    assert_eq!(result.retdata, vec![25.into(), 17.into()]);
}

#[test]
fn call_contract_upgrade_cairo_0_to_cairo_1_same_transaction() {
    /* Test Outline:
       - Add `get_number_c.cairo` contract at address 2 and `get_number_b.cairo` contract without an address
       - Call `get_numbers_old_new` function of `get_number_wrapper.cairo` and expect to get both answers from `get_number_c`, and 'get_number_b' (33, 17)
    */

    // SET GET_NUMBER_C

    // Add get_number_a.cairo to storage

    let path = PathBuf::from("starknet_programs/get_number_c.json");
    let contract_class_c = ContractClass::try_from(path).unwrap();

    // Create state reader with class hash data
    let mut casm_contract_class_cache = HashMap::new();
    let mut deprecated_contract_class_cache = HashMap::new();

    let address = Address(Felt252::one());
    let class_hash_c: ClassHash = Felt252::one().to_be_bytes();
    let nonce = Felt252::zero();

    deprecated_contract_class_cache.insert(class_hash_c, contract_class_c);
    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash_c);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce.clone());

    // SET GET_NUMBER_B

    // Add get_number_b contract to the state (only its contract_class)

    let program_data = include_bytes!("../starknet_programs/cairo1/get_number_b.casm");
    let contract_class_b: CasmContractClass = serde_json::from_slice(program_data).unwrap();

    let class_hash_b: ClassHash = Felt252::from(2).to_be_bytes();

    casm_contract_class_cache.insert(class_hash_b, contract_class_b);

    // SET GET_NUMBER_WRAPPER

    //  Create program and entry point types for contract class
    let program_data = include_bytes!("../starknet_programs/cairo1/get_number_wrapper.casm");
    let wrapper_contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = wrapper_contract_class.clone().entry_points_by_type;
    let get_numbers_entrypoint_selector = &entrypoints.external.get(2).unwrap().selector;

    let wrapper_address = Address(Felt252::from(2));
    let wrapper_class_hash: ClassHash = [3; 32];

    casm_contract_class_cache.insert(wrapper_class_hash, wrapper_contract_class);
    state_reader
        .address_to_class_hash_mut()
        .insert(wrapper_address.clone(), wrapper_class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(wrapper_address, nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(
        state_reader,
        Some(deprecated_contract_class_cache),
        Some(casm_contract_class_cache),
    );

    // INITIALIZE STARKNET CONFIG
    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );
    let mut resources_manager = ExecutionResourcesManager::default();

    // CALL GET_NUMBERS_OLD_NEW

    let calldata = [Felt252::from_bytes_be(&class_hash_b)].to_vec();
    let caller_address = Address(0000.into());
    let entry_point_type = EntryPointType::External;

    let exec_entry_point = ExecutionEntryPoint::new(
        address,
        calldata,
        Felt252::new(get_numbers_entrypoint_selector.clone()),
        caller_address,
        entry_point_type,
        Some(CallType::Delegate),
        Some(wrapper_class_hash),
        u64::MAX.into(),
    );

    let result = exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();
    assert_eq!(result.retdata, vec![33.into(), 17.into()]);
}

#[test]
fn call_contract_downgrade_cairo_1_to_cairo_0_same_transaction() {
    /* Test Outline:
       - Add `get_number_b.cairo` contract at address 2 and `get_number_c.cairo` contract without an address
       - Call `get_numbers_old_new` function of `get_number_wrapper.cairo` and expect to get both answers from `get_number_b`, and 'get_number_c' (17, 33)
    */

    // SET GET_NUMBER_C
    // Add get_number_a.cairo to the state (only its contract_class)
    let path = PathBuf::from("starknet_programs/get_number_c.json");
    let contract_class_c = ContractClass::try_from(path).unwrap();

    // Create state reader with class hash data
    let mut casm_contract_class_cache = HashMap::new();
    let mut deprecated_contract_class_cache = HashMap::new();

    let address = Address(Felt252::one());
    let class_hash_c: ClassHash = Felt252::one().to_be_bytes();
    let nonce = Felt252::zero();

    deprecated_contract_class_cache.insert(class_hash_c, contract_class_c);

    // SET GET_NUMBER_B

    // Add get_number_b contract to the state

    let program_data = include_bytes!("../starknet_programs/cairo1/get_number_b.casm");
    let contract_class_b: CasmContractClass = serde_json::from_slice(program_data).unwrap();

    let class_hash_b: ClassHash = Felt252::from(2).to_be_bytes();

    casm_contract_class_cache.insert(class_hash_b, contract_class_b);
    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash_b);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce.clone());

    // SET GET_NUMBER_WRAPPER

    //  Create program and entry point types for contract class
    let program_data = include_bytes!("../starknet_programs/cairo1/get_number_wrapper.casm");
    let wrapper_contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = wrapper_contract_class.clone().entry_points_by_type;
    let get_numbers_entrypoint_selector = &entrypoints.external.get(2).unwrap().selector;

    let wrapper_address = Address(Felt252::from(2));
    let wrapper_class_hash: ClassHash = [3; 32];

    casm_contract_class_cache.insert(wrapper_class_hash, wrapper_contract_class);
    state_reader
        .address_to_class_hash_mut()
        .insert(wrapper_address.clone(), wrapper_class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(wrapper_address, nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(
        state_reader,
        Some(deprecated_contract_class_cache),
        Some(casm_contract_class_cache),
    );

    // INITIALIZE STARKNET CONFIG
    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );
    let mut resources_manager = ExecutionResourcesManager::default();

    // CALL GET_NUMBERS_OLD_NEW

    let calldata = [Felt252::from_bytes_be(&class_hash_c)].to_vec();
    let caller_address = Address(0000.into());
    let entry_point_type = EntryPointType::External;

    let exec_entry_point = ExecutionEntryPoint::new(
        address,
        calldata,
        Felt252::new(get_numbers_entrypoint_selector.clone()),
        caller_address,
        entry_point_type,
        Some(CallType::Delegate),
        Some(wrapper_class_hash),
        u64::MAX.into(),
    );

    let result = exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();
    assert_eq!(result.retdata, vec![17.into(), 33.into()]);
}

#[test]
fn call_contract_replace_class_cairo_0() {
    /* Test Outline:
       - Add `get_number_d.cairo` contract at address 2 and `get_number_c.cairo` contract without an address
       - Call `get_numbers_old_new` function of `get_number_wrapper.cairo` and expect to get both answers from `get_number_d`, and 'get_number_c' (64, 33)
    */

    // SET GET_NUMBER_C
    // Add get_number_a.cairo to the state (only its contract_class)
    let path = PathBuf::from("starknet_programs/get_number_c.json");
    let contract_class_c = ContractClass::try_from(path).unwrap();

    // Create state reader with class hash data
    let mut casm_contract_class_cache = HashMap::new();
    let mut deprecated_contract_class_cache = HashMap::new();

    let address = Address(Felt252::one());
    let class_hash_c: ClassHash = Felt252::one().to_be_bytes();
    let nonce = Felt252::zero();

    deprecated_contract_class_cache.insert(class_hash_c, contract_class_c);

    // SET GET_NUMBER_B

    // Add get_number_b contract to the state

    let path = PathBuf::from("starknet_programs/get_number_d.json");
    let contract_class_d = ContractClass::try_from(path).unwrap();

    let class_hash_d: ClassHash = Felt252::from(2).to_be_bytes();

    deprecated_contract_class_cache.insert(class_hash_d, contract_class_d);
    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash_d);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce.clone());

    // SET GET_NUMBER_WRAPPER

    //  Create program and entry point types for contract class
    let program_data = include_bytes!("../starknet_programs/cairo1/get_number_wrapper.casm");
    let wrapper_contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = wrapper_contract_class.clone().entry_points_by_type;
    let get_numbers_entrypoint_selector = &entrypoints.external.get(2).unwrap().selector;

    let wrapper_address = Address(Felt252::from(2));
    let wrapper_class_hash: ClassHash = [3; 32];

    casm_contract_class_cache.insert(wrapper_class_hash, wrapper_contract_class);
    state_reader
        .address_to_class_hash_mut()
        .insert(wrapper_address.clone(), wrapper_class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(wrapper_address, nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(
        state_reader,
        Some(deprecated_contract_class_cache),
        Some(casm_contract_class_cache),
    );

    // INITIALIZE STARKNET CONFIG
    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );
    let mut resources_manager = ExecutionResourcesManager::default();

    // CALL GET_NUMBERS_OLD_NEW

    let calldata = [Felt252::from_bytes_be(&class_hash_c)].to_vec();
    let caller_address = Address(0000.into());
    let entry_point_type = EntryPointType::External;

    let exec_entry_point = ExecutionEntryPoint::new(
        address,
        calldata,
        Felt252::new(get_numbers_entrypoint_selector.clone()),
        caller_address,
        entry_point_type,
        Some(CallType::Delegate),
        Some(wrapper_class_hash),
        u64::MAX.into(),
    );

    let result = exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();
    assert_eq!(result.retdata, vec![64.into(), 33.into()]);
}

#[test]
fn test_out_of_gas_failure() {
    //  Create program and entry point types for contract class
    let program_data = include_bytes!("../starknet_programs/cairo1/emit_event.casm");
    let contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = contract_class.clone().entry_points_by_type;
    let entrypoint_selector = &entrypoints.external.get(0).unwrap().selector;

    // Create state reader with class hash data
    let mut contract_class_cache = HashMap::new();

    let address = Address(1111.into());
    let class_hash: ClassHash = [1; 32];
    let nonce = Felt252::zero();

    contract_class_cache.insert(class_hash, contract_class);
    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(state_reader, None, Some(contract_class_cache));

    // Create an execution entry point
    let calldata = [].to_vec();
    let caller_address = Address(0000.into());
    let entry_point_type = EntryPointType::External;

    // Purposefully set initial gas to 0 so that the syscall fails
    let exec_entry_point = ExecutionEntryPoint::new(
        address,
        calldata,
        Felt252::new(entrypoint_selector.clone()),
        caller_address,
        entry_point_type,
        Some(CallType::Delegate),
        Some(class_hash),
        0,
    );

    // Execute the entrypoint
    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );
    let mut resources_manager = ExecutionResourcesManager::default();
    let call_info = exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();
    assert_eq!(
        call_info.retdata,
        vec![Felt252::from_bytes_be("Out of gas".as_bytes())]
    );
    assert!(call_info.failure_flag)
}

#[test]
fn deploy_syscall_failure_uninitialized_class_hash() {
    //  Create program and entry point types for contract class
    let program_data = include_bytes!("../starknet_programs/cairo1/deploy_contract_no_args.casm");
    let contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = contract_class.clone().entry_points_by_type;
    let entrypoint_selector = &entrypoints.external.get(0).unwrap().selector;

    // Create state reader with class hash data
    let mut contract_class_cache = HashMap::new();

    let address = Address(1111.into());
    let class_hash: ClassHash = [1; 32];
    let nonce = Felt252::zero();

    contract_class_cache.insert(class_hash, contract_class);
    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(state_reader, None, Some(contract_class_cache));

    // Create an execution entry point
    let calldata = [Felt252::zero()].to_vec();
    let caller_address = Address(0000.into());
    let entry_point_type = EntryPointType::External;

    let exec_entry_point = ExecutionEntryPoint::new(
        address,
        calldata,
        Felt252::new(entrypoint_selector.clone()),
        caller_address,
        entry_point_type,
        Some(CallType::Delegate),
        Some(class_hash),
        100000,
    );

    // Execute the entrypoint
    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );
    let mut resources_manager = ExecutionResourcesManager::default();
    let call_info = exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();
    assert_eq!(
        std::str::from_utf8(&call_info.retdata[0].to_be_bytes())
            .unwrap()
            .trim_start_matches('\0'),
        "CLASS_HASH_NOT_FOUND"
    )
}

#[test]
fn deploy_syscall_failure_in_constructor() {
    //  Create program and entry point types for contract class
    let program_data = include_bytes!("../starknet_programs/cairo1/deploy_contract_no_args.casm");
    let contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = contract_class.clone().entry_points_by_type;
    let entrypoint_selector = &entrypoints.external.get(0).unwrap().selector;

    // Create state reader with class hash data
    let mut contract_class_cache = HashMap::new();

    let address = Address(1111.into());
    let class_hash: ClassHash = [1; 32];
    let nonce = Felt252::zero();

    contract_class_cache.insert(class_hash, contract_class);
    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce);

    // Add failing constructor contract
    let f_c_program_data = include_bytes!("../starknet_programs/cairo1/failing_constructor.casm");
    let f_c_contract_class: CasmContractClass = serde_json::from_slice(f_c_program_data).unwrap();
    let f_c_class_hash = Felt252::one();
    contract_class_cache.insert(f_c_class_hash.to_be_bytes(), f_c_contract_class);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(state_reader, None, Some(contract_class_cache));

    // Create an execution entry point
    let calldata = [f_c_class_hash].to_vec();
    let caller_address = Address(0000.into());
    let entry_point_type = EntryPointType::External;

    let exec_entry_point = ExecutionEntryPoint::new(
        address,
        calldata,
        Felt252::new(entrypoint_selector.clone()),
        caller_address,
        entry_point_type,
        Some(CallType::Delegate),
        Some(class_hash),
        100000,
    );

    // Execute the entrypoint
    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );
    let mut resources_manager = ExecutionResourcesManager::default();
    let call_info = exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();
    // Check that we get the error from the constructor
    // assert( 1 == 0 , 'Oops');
    assert_eq!(
        std::str::from_utf8(&call_info.retdata[0].to_be_bytes())
            .unwrap()
            .trim_start_matches('\0'),
        "Oops"
    )
}

#[test]
fn storage_read_no_value() {
    //  Create program and entry point types for contract class
    let program_data = include_bytes!("../starknet_programs/cairo1/simple_wallet.casm");
    let contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = contract_class.clone().entry_points_by_type;
    let get_balance_entrypoint_selector = &entrypoints.external.get(1).unwrap().selector;

    // Create state reader with class hash data
    let mut contract_class_cache = HashMap::new();

    let address = Address(1111.into());
    let class_hash: ClassHash = [1; 32];
    let nonce = Felt252::zero();

    contract_class_cache.insert(class_hash, contract_class);
    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(state_reader, None, Some(contract_class_cache));

    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );

    let mut resources_manager = ExecutionResourcesManager::default();

    let create_execute_extrypoint = |selector: &BigUint,
                                     calldata: Vec<Felt252>,
                                     entry_point_type: EntryPointType|
     -> ExecutionEntryPoint {
        ExecutionEntryPoint::new(
            address.clone(),
            calldata,
            Felt252::new(selector.clone()),
            Address(0000.into()),
            entry_point_type,
            Some(CallType::Delegate),
            Some(class_hash),
            100000,
        )
    };

    // RUN GET_BALANCE
    // Create an execution entry point
    let calldata = [].to_vec();
    let get_balance_exec_entry_point = create_execute_extrypoint(
        get_balance_entrypoint_selector,
        calldata,
        EntryPointType::External,
    );

    // Run get_balance entrypoint
    let call_info = get_balance_exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();
    // As the value doesn't exist in storage, it's value will be 0
    assert_eq!(call_info.retdata, [0.into()]);
}

#[test]
fn storage_read_unavailable_address_domain() {
    //  Create program and entry point types for contract class
    let program_data =
        include_bytes!("../starknet_programs/cairo1/faulty_low_level_storage_read.casm");
    let contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = contract_class.clone().entry_points_by_type;
    let read_storage_entrypoint_selector = &entrypoints.external.get(0).unwrap().selector;

    // Create state reader with class hash data
    let mut contract_class_cache = HashMap::new();

    let address = Address(1111.into());
    let class_hash: ClassHash = [1; 32];
    let nonce = Felt252::zero();

    contract_class_cache.insert(class_hash, contract_class);
    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(state_reader, None, Some(contract_class_cache));

    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );

    let mut resources_manager = ExecutionResourcesManager::default();

    let create_execute_extrypoint = |selector: &BigUint,
                                     calldata: Vec<Felt252>,
                                     entry_point_type: EntryPointType|
     -> ExecutionEntryPoint {
        ExecutionEntryPoint::new(
            address.clone(),
            calldata,
            Felt252::new(selector.clone()),
            Address(0000.into()),
            entry_point_type,
            Some(CallType::Delegate),
            Some(class_hash),
            100000,
        )
    };

    // RUN READ_STORAGE
    // Create an execution entry point
    let calldata = [].to_vec();
    let read_storage_exec_entry_point = create_execute_extrypoint(
        read_storage_entrypoint_selector,
        calldata,
        EntryPointType::External,
    );

    // Run read_storage entrypoint
    let call_info = read_storage_exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();

    assert_eq!(
        call_info.retdata[0],
        Felt252::from_bytes_be(b"Unsupported address domain")
    );
}

#[test]
fn storage_write_unavailable_address_domain() {
    //  Create program and entry point types for contract class
    let program_data =
        include_bytes!("../starknet_programs/cairo1/faulty_low_level_storage_write.casm");
    let contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = contract_class.clone().entry_points_by_type;
    let read_storage_entrypoint_selector = &entrypoints.external.get(0).unwrap().selector;

    // Create state reader with class hash data
    let mut contract_class_cache = HashMap::new();

    let address = Address(1111.into());
    let class_hash: ClassHash = [1; 32];
    let nonce = Felt252::zero();

    contract_class_cache.insert(class_hash, contract_class);
    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(state_reader, None, Some(contract_class_cache));

    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );

    let mut resources_manager = ExecutionResourcesManager::default();

    let create_execute_extrypoint = |selector: &BigUint,
                                     calldata: Vec<Felt252>,
                                     entry_point_type: EntryPointType|
     -> ExecutionEntryPoint {
        ExecutionEntryPoint::new(
            address.clone(),
            calldata,
            Felt252::new(selector.clone()),
            Address(0000.into()),
            entry_point_type,
            Some(CallType::Delegate),
            Some(class_hash),
            100000,
        )
    };

    // RUN READ_STORAGE
    // Create an execution entry point
    let calldata = [].to_vec();
    let read_storage_exec_entry_point = create_execute_extrypoint(
        read_storage_entrypoint_selector,
        calldata,
        EntryPointType::External,
    );

    // Run read_storage entrypoint
    let call_info = read_storage_exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();

    assert_eq!(
        call_info.retdata[0],
        Felt252::from_bytes_be(b"Unsupported address domain")
    );
}

#[test]
fn library_call_failure() {
    //  Create program and entry point types for contract class
    let program_data = include_bytes!("../starknet_programs/cairo1/square_root.casm");
    let contract_class: CasmContractClass = serde_json::from_slice(program_data).unwrap();
    let entrypoints = contract_class.clone().entry_points_by_type;
    let entrypoint_selector = &entrypoints.external.get(0).unwrap().selector;

    // Create state reader with class hash data
    let mut contract_class_cache = HashMap::new();

    let address = Address(1111.into());
    let class_hash: ClassHash = [1; 32];
    let nonce = Felt252::zero();

    contract_class_cache.insert(class_hash, contract_class);
    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce);

    // Add lib contract to the state

    let lib_program_data = include_bytes!("../starknet_programs/cairo1/faulty_math_lib.casm");
    let lib_contract_class: CasmContractClass = serde_json::from_slice(lib_program_data).unwrap();

    let lib_address = Address(1112.into());
    let lib_class_hash: ClassHash = [2; 32];
    let lib_nonce = Felt252::zero();

    contract_class_cache.insert(lib_class_hash, lib_contract_class);
    state_reader
        .address_to_class_hash_mut()
        .insert(lib_address.clone(), lib_class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(lib_address, lib_nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(state_reader, None, Some(contract_class_cache));

    // Create an execution entry point
    let calldata = [25.into(), Felt252::from_bytes_be(&lib_class_hash)].to_vec();
    let caller_address = Address(0000.into());
    let entry_point_type = EntryPointType::External;

    let exec_entry_point = ExecutionEntryPoint::new(
        address,
        calldata,
        Felt252::new(entrypoint_selector.clone()),
        caller_address,
        entry_point_type,
        Some(CallType::Delegate),
        Some(class_hash),
        100000,
    );

    // Execute the entrypoint
    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(0.into()),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );
    let mut resources_manager = ExecutionResourcesManager::default();
    let mut expected_execution_resources = ExecutionResources::default();
    expected_execution_resources
        .builtin_instance_counter
        .insert(RANGE_CHECK_BUILTIN_NAME.to_string(), 7);
    expected_execution_resources.n_memory_holes = 6;

    let call_info = exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();
    assert_eq!(
        std::str::from_utf8(&call_info.retdata[0].to_be_bytes())
            .unwrap()
            .trim_start_matches('\0'),
        "Unimplemented"
    );
    assert!(call_info.failure_flag);
}

#[test]
fn duck_duck() {
    //  Create program and entry point types for contract class
    let program_data = include_bytes!("../proxy_contract.json");
    let sierra_class: SierraContractClass = serde_json::from_slice(program_data).unwrap();
    let contract_class = CasmContractClass::from_contract_class(sierra_class, false).unwrap();
    let entrypoint_selector =
        Felt252::from_bytes_be(&calculate_sn_keccak("__execute__".as_bytes()));

    // Create state reader with class hash data
    let mut contract_class_cache = HashMap::new();

    let address = Address(felt_str!(
        "77873968982793223421019657832424040434423608647272643589382819351924744979"
    ));
    let class_hash: ClassHash =
        felt_str!("802624353088231488816884627621717971676208682927435467151080563415187453099")
            .to_be_bytes();
    let nonce = 8.into();

    contract_class_cache.insert(class_hash, contract_class);
    let mut state_reader = InMemoryStateReader::default();
    state_reader
        .address_to_class_hash_mut()
        .insert(address.clone(), class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(address.clone(), nonce);

    // // Add contract a 

    let program_data = include_bytes!("../contract.json");
    let sierra_class: SierraContractClass = serde_json::from_slice(program_data).unwrap();
    let a_contract_class = CasmContractClass::from_contract_class(sierra_class, false).unwrap();

    let a_address = Address(felt_str!(
        "2580190347992197155439822453898096474131348849780754094111958454284924548879"
    ));
    let a_class_hash: ClassHash =
        felt_str!("842887443104562215789282210875508475757928512615979091764734108797688225873")
            .to_be_bytes();
    let a_nonce = 8.into();

    contract_class_cache.insert(a_class_hash, a_contract_class);
    state_reader
        .address_to_class_hash_mut()
        .insert(a_address.clone(), a_class_hash);
    state_reader
        .address_to_nonce_mut()
        .insert(a_address, a_nonce);

        // // Add contract b

        let program_data = include_bytes!("../contract_b.json");
        let sierra_class: SierraContractClass = serde_json::from_slice(program_data).unwrap();
        let b_contract_class = CasmContractClass::from_contract_class(sierra_class, false).unwrap();
    
        let b_address = Address(felt_str!(
            "2130449614087309992146828810340965483383114570091747539799843737140266351087"
        ));
        let b_class_hash: ClassHash =
            felt_str!("802624353088231488816884627621717971676208682927435467151080563415187453099")
                .to_be_bytes();
        let b_nonce = 8.into();
    
        contract_class_cache.insert(b_class_hash, b_contract_class);
        state_reader
            .address_to_class_hash_mut()
            .insert(b_address.clone(), b_class_hash);
        state_reader
            .address_to_nonce_mut()
            .insert(b_address, b_nonce);

    // Create state from the state_reader and contract cache.
    let mut state = CachedState::new(state_reader, None, Some(contract_class_cache));

    // Create an execution entry point
    let calldata = [
        7.into(),
        felt_str!("2580190347992197155439822453898096474131348849780754094111958454284924548879"),
        felt_str!("1492333436847995578409168440683494605461205546075485927265435102999436131328"),
        9.into(),
        felt_str!("77873968982793223421019657832424040434423608647272643589382819351924744979"),
        felt_str!("150636832903696325546641289108291653851"),
        felt_str!("2807"),
        1.into(),
        0.into(),
        felt_str!("1686614108162"),
        2.into(),
        felt_str!("2838037593421011682040206231115643514351145082096714485433305571877255482573"),
        felt_str!("576002959559456027292539029984046431986066260421562596972732223346710464473"),
        felt_str!("2580190347992197155439822453898096474131348849780754094111958454284924548879"),
        felt_str!("1492333436847995578409168440683494605461205546075485927265435102999436131328"),
        9.into(),
        felt_str!("77873968982793223421019657832424040434423608647272643589382819351924744979"),
        felt_str!("60701700828080268569173957296498950203"),
        3970.into(),
        1.into(),
        0.into(),
        felt_str!("1686614108073"),
        2.into(),
        felt_str!("2738970863352631846119294542616503024596973183673754838481598341307340563873"),
        felt_str!("2289542090749782974873244021798739935713097027878001942657875702161154215818"),
        felt_str!("2580190347992197155439822453898096474131348849780754094111958454284924548879"),
        felt_str!("1492333436847995578409168440683494605461205546075485927265435102999436131328"),
        9.into(),
        felt_str!("77873968982793223421019657832424040434423608647272643589382819351924744979"),
        felt_str!("186730040080592220962709944354791575751"),
        3901.into(),
        1.into(),
        0.into(),
        felt_str!("1686614107983"),
        2.into(),
        felt_str!("642149337328904143857345936858149148810891643212815928563759075479436902882"),
        felt_str!("2239263860171003056487104845409108089910217969443805784366359135959640908389"),
        felt_str!("2580190347992197155439822453898096474131348849780754094111958454284924548879"),
        felt_str!("1492333436847995578409168440683494605461205546075485927265435102999436131328"),
        9.into(),
        felt_str!("77873968982793223421019657832424040434423608647272643589382819351924744979"),
        felt_str!("5594899672282902975493094788360279362"),
        felt_str!("3951"),
        1.into(),
        0.into(),
        felt_str!("1686614107895"),
        2.into(),
        felt_str!("2117980541956216127969187287994655342696369213088562040104464159565197846431"),
        felt_str!("1718149643841749268075812605501048476772167968852070960593134916233067950887"),
        felt_str!("2580190347992197155439822453898096474131348849780754094111958454284924548879"),
        felt_str!("1492333436847995578409168440683494605461205546075485927265435102999436131328"),
        9.into(),
        felt_str!("77873968982793223421019657832424040434423608647272643589382819351924744979"),
        felt_str!("72346440218236392623702732007279558860"),
        95.into(),
        1.into(),
        0.into(),
        felt_str!("1686609367017"),
        2.into(),
        felt_str!("2417356759951474636298405104785780039531934595725743038046307112652082895853"),
        felt_str!("2931732294736423302953169349445495756192133466958844746409896044681482210726"),
        felt_str!("2580190347992197155439822453898096474131348849780754094111958454284924548879"),
        felt_str!("1492333436847995578409168440683494605461205546075485927265435102999436131328"),
        9.into(),
        felt_str!("77873968982793223421019657832424040434423608647272643589382819351924744979"),
        felt_str!("164035543437603415651599364501344036828"),
        felt_str!("1901"),
        1.into(),
        0.into(),
        felt_str!("1686606806597"),
        2.into(),
        felt_str!("3124104964329003281873145385557057053662725373395617664452733262013059127109"),
        felt_str!("346724688481333372980882433121536165170847934867308826598344711274471562613"),
        felt_str!("2580190347992197155439822453898096474131348849780754094111958454284924548879"),
        felt_str!("1507885890829902860906790956815692246920905994909660809307241680906695044279"),
        29.into(),
        felt_str!("77873968982793223421019657832424040434423608647272643589382819351924744979"),
        felt_str!("3287993214422593717072581321321443562923386764694574974403034603715113475772"),
        6.into(),
        felt_str!("150636832903696325546641289108291653851"), //79
        2807.into(),
        felt_str!("60701700828080268569173957296498950203"),
        3970.into(),
        felt_str!("186730040080592220962709944354791575751"),
        3901.into(),
        felt_str!("5594899672282902975493094788360279362"),
        3951.into(),
        felt_str!("72346440218236392623702732007279558860"),
        95.into(),
        felt_str!("164035543437603415651599364501344036828"),
        1901.into(),
        6.into(), //91
        1.into(), //92
        0.into(), //93
        1.into(), //94
        0.into(),
        1.into(),
        0.into(), //97
        1.into(),
        0.into(),
        1.into(), //100
        0.into(),
        1.into(), //102
        0.into(),
        0.into(),
    ]
    .to_vec();
    let caller_address = Address(felt_str!("77873968982793223421019657832424040434423608647272643589382819351924744979"));
    let entry_point_type = EntryPointType::External;

    let exec_entry_point = ExecutionEntryPoint::new(
        address,
        calldata,
        Felt252::new(entrypoint_selector.clone()),
        caller_address,
        entry_point_type,
        Some(CallType::Delegate),
        Some(class_hash),
        u128::MAX,
    );

    // Execute the entrypoint
    let block_context = BlockContext::default();
    let mut tx_execution_context = TransactionExecutionContext::new(
        Address(felt_str!("77873968982793223421019657832424040434423608647272643589382819351924744979")),
        Felt252::zero(),
        Vec::new(),
        0,
        10.into(),
        block_context.invoke_tx_max_n_steps(),
        TRANSACTION_VERSION.clone(),
    );
    let mut resources_manager = ExecutionResourcesManager::default();
    let mut expected_execution_resources = ExecutionResources::default();
    expected_execution_resources
        .builtin_instance_counter
        .insert(RANGE_CHECK_BUILTIN_NAME.to_string(), 7);
    expected_execution_resources.n_memory_holes = 6;

    let call_info = exec_entry_point
        .execute(
            &mut state,
            &block_context,
            &mut resources_manager,
            &mut tx_execution_context,
            false,
        )
        .unwrap();
    assert_eq!(
        std::str::from_utf8(&call_info.retdata[0].to_be_bytes())
            .unwrap()
            .trim_start_matches('\0'),
        "Unimplemented"
    );
    assert!(call_info.failure_flag);
}

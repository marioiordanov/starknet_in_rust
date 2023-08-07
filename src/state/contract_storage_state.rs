use super::{
    cached_state::CachedState,
    state_api::{State, StateReader},
};
use crate::{
    core::errors::state_errors::StateError,
    utils::{Address, ClassHash},
};
use cairo_vm::felt::Felt252;
use std::collections::HashSet;

/// Represents the storage state of a contract, keeping track of read values and accessed keys.
#[derive(Debug)]
pub(crate) struct ContractStorageState<'a, S: StateReader> {
    /// Cached state reference for reading and writing contract storage.
    pub(crate) state: &'a mut CachedState<S>,
    /// Address of the contract whose storage is being managed.
    pub(crate) contract_address: Address,
    /// Maintain all read request values in chronological order
    pub(crate) read_values: Vec<Felt252>,
    /// Set of keys (ClassHash) that have been accessed (either read or written to).
    pub(crate) accessed_keys: HashSet<ClassHash>,
}

impl<'a, S: StateReader> ContractStorageState<'a, S> {
    /// Creates a new ContractStorageState instance for a given contract address.
    pub(crate) fn new(state: &'a mut CachedState<S>, contract_address: Address) -> Self {
        Self {
            state,
            contract_address,
            read_values: Vec::new(),
            accessed_keys: HashSet::new(),
        }
    }

    /// Reads a value from the contract's storage for a given key (address).
    /// Records the read value and adds the key to the set of accessed keys.
    pub(crate) fn read(&mut self, address: &ClassHash) -> Result<Felt252, StateError> {
        self.accessed_keys.insert(*address);
        let value = self
            .state
            .get_storage_at(&(self.contract_address.clone(), *address))?;

        self.read_values.push(value.clone());
        Ok(value)
    }

    /// Writes a value to the contract's storage for a given key (address).
    /// Adds the key to the set of accessed keys.
    pub(crate) fn write(&mut self, address: &ClassHash, value: Felt252) {
        self.accessed_keys.insert(*address);
        self.state
            .set_storage_at(&(self.contract_address.clone(), *address), value);
    }
}

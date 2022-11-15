use radix_engine::ledger::{
    bootstrap, OutputValue, QueryableSubstateStore, ReadableSubstateStore, WriteableSubstateStore,
};
use radix_engine::model::PersistedSubstate;
use radix_engine::types::*;

/// A substate store that stores all typed substates in host memory.
#[derive(Debug, PartialEq, Eq)]
pub struct SerializedInMemorySubstateStore {
    substates: HashMap<Vec<u8>, Vec<u8>>,
}

impl SerializedInMemorySubstateStore {
    pub fn new() -> Self {
        Self {
            substates: HashMap::new(),
        }
    }

    pub fn with_bootstrap() -> Self {
        let mut substate_store = Self::new();
        bootstrap(&mut substate_store);
        substate_store
    }
}

impl Default for SerializedInMemorySubstateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadableSubstateStore for SerializedInMemorySubstateStore {
    fn get_substate(&self, substate_id: &SubstateId) -> Option<OutputValue> {
        self.substates
            .get(&scrypto_encode(substate_id))
            .map(|b| scrypto_decode(&b).unwrap())
    }
}

impl WriteableSubstateStore for SerializedInMemorySubstateStore {
    fn put_substate(&mut self, substate_id: SubstateId, substate: OutputValue) {
        self.substates
            .insert(scrypto_encode(&substate_id), scrypto_encode(&substate));
    }
}

impl QueryableSubstateStore for SerializedInMemorySubstateStore {
    fn get_kv_store_entries(
        &self,
        kv_store_id: &KeyValueStoreId,
    ) -> HashMap<Vec<u8>, PersistedSubstate> {
        self.substates
            .iter()
            .filter_map(|(key, value)| {
                let substate_id: SubstateId = scrypto_decode(key).unwrap();
                if let SubstateId(
                    RENodeId::KeyValueStore(id),
                    SubstateOffset::KeyValueStore(KeyValueStoreOffset::Entry(key)),
                ) = substate_id
                {
                    let output_value: OutputValue = scrypto_decode(value).unwrap();
                    if id == *kv_store_id {
                        Some((key.clone(), output_value.substate))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }
}
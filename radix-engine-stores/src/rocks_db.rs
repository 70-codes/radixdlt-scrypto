use crate::interface::*;
use radix_engine_interface::crypto::Hash;
use radix_engine_interface::data::scrypto::{scrypto_decode, scrypto_encode};
use radix_engine_interface::types::*;
use rocksdb::{DBWithThreadMode, Direction, IteratorMode, SingleThreaded, DB};
use sbor::rust::prelude::*;
use std::path::PathBuf;

pub struct RocksdbSubstateStore {
    db: DBWithThreadMode<SingleThreaded>,
}

impl RocksdbSubstateStore {
    pub fn standard(root: PathBuf) -> Self {
        let configs: BTreeMap<ModuleId, ModuleConfig> = btreemap!(
            SysModuleId::TypeInfo.into() => ModuleConfig {
                iteration_enabled: false,
            },
            SysModuleId::ObjectTuple.into() => ModuleConfig {
                iteration_enabled: true,
            },
            SysModuleId::Metadata.into() => ModuleConfig {
                iteration_enabled: true,
            },
            SysModuleId::Royalty.into() => ModuleConfig {
                iteration_enabled: false,
            },
            SysModuleId::AccessRules.into() => ModuleConfig {
                iteration_enabled: false,
            },
        );
        let db = DB::open_default(root.as_path()).expect("IO Error");

        if db.get([0]).expect("IO Error").is_none() {
            db.put(
                [0],
                scrypto_encode(&configs).expect("Failed to encode configs"),
            )
            .expect("IO Error");
        }

        Self { db }
    }

    pub fn configs(&self) -> BTreeMap<ModuleId, ModuleConfig> {
        scrypto_decode(
            &self
                .db
                .get([0])
                .expect("IO error")
                .expect("Missing configs"),
        )
        .expect("Failed to decode configs")
    }

    pub fn list_nodes(&self) -> Vec<NodeId> {
        let mut items = Vec::new();
        let mut iter = self
            .db
            .iterator(IteratorMode::From(&[], Direction::Forward));
        while let Some(kv) = iter.next() {
            let (key, _value) = kv.unwrap();
            if key.len() < NodeId::LENGTH {
                continue;
            }
            let (node_id, _, _) = decode_substate_id(key.as_ref()).unwrap();
            if items.last() != Some(&node_id) {
                items.push(node_id);
            }
        }
        items
    }

    pub fn list_packages(&self) -> Vec<PackageAddress> {
        self.list_nodes()
            .into_iter()
            .filter_map(|x| PackageAddress::try_from(x.as_ref()).ok())
            .collect()
    }

    pub fn list_components(&self) -> Vec<ComponentAddress> {
        self.list_nodes()
            .into_iter()
            .filter_map(|x| ComponentAddress::try_from(x.as_ref()).ok())
            .collect()
    }

    pub fn list_resource_managers(&self) -> Vec<ResourceAddress> {
        self.list_nodes()
            .into_iter()
            .filter_map(|x| ResourceAddress::try_from(x.as_ref()).ok())
            .collect()
    }
}

impl SubstateDatabase for RocksdbSubstateStore {
    fn get_substate(
        &self,
        node_id: &NodeId,
        module_id: ModuleId,
        substate_key: &SubstateKey,
    ) -> Result<Option<Vec<u8>>, GetSubstateError> {
        if !self.configs().contains_key(&module_id) {
            return Err(GetSubstateError::UnknownModuleId);
        }

        let key = encode_substate_id(node_id, module_id, substate_key);
        let value = self
            .db
            .get(&key)
            .expect("IO Error")
            .map(|x| scrypto_decode::<Vec<u8>>(&x).expect("Failed to decode value"));
        Ok(value)
    }

    fn list_substates(
        &self,
        node_id: &NodeId,
        module_id: ModuleId,
    ) -> Result<(Vec<(SubstateKey, Vec<u8>)>, Hash), ListSubstatesError> {
        match self.configs().get(&module_id) {
            None => {
                return Err(ListSubstatesError::UnknownModuleId);
            }
            Some(config) => {
                if !config.iteration_enabled {
                    return Err(ListSubstatesError::IterationNotAllowed);
                }
            }
        }

        let start = encode_substate_id(node_id, module_id, &SubstateKey::min());
        let end = encode_substate_id(node_id, module_id, &SubstateKey::max());
        let mut substates = Vec::<(SubstateKey, Vec<u8>)>::new();

        let mut iter = self
            .db
            .iterator(IteratorMode::From(&start, Direction::Forward));
        while let Some(kv) = iter.next() {
            let (key, value) = kv.unwrap();
            if key.as_ref() > &end {
                break;
            }
            let (_, _, substate_key) =
                decode_substate_id(key.as_ref()).expect("Failed to decode substate ID");
            let value = scrypto_decode::<Vec<u8>>(value.as_ref()).expect("Failed to decode value");
            substates.push((substate_key, value));
        }

        Ok((substates, Hash([0; Hash::LENGTH])))
    }
}

impl CommittableSubstateDatabase for RocksdbSubstateStore {
    fn commit(&mut self, state_changes: &StateUpdates) -> Result<(), CommitError> {
        for ((node_id, module_id, substate_key), substate_change) in &state_changes.substate_changes
        {
            let substate_id = encode_substate_id(node_id, *module_id, substate_key);
            match substate_change {
                StateUpdate::Create(substate_value) => {
                    self.db
                        .put(substate_id, scrypto_encode(&substate_value).unwrap())
                        .expect("IO error");
                }
                StateUpdate::Update(substate_value) => {
                    self.db
                        .put(substate_id, scrypto_encode(&substate_value).unwrap())
                        .expect("IO error");
                }
            }
        }
        Ok(())
    }
}

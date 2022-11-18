use radix_engine_interface::api::api::EngineApi;
use radix_engine_interface::api::types::{
    KeyValueStoreId, KeyValueStoreOffset, RENodeId, ScryptoRENode, SubstateOffset,
};
use radix_engine_interface::data::*;

use sbor::rust::borrow::ToOwned;
use sbor::rust::boxed::Box;
use sbor::rust::fmt;
use sbor::rust::marker::PhantomData;
use sbor::rust::str::FromStr;
use sbor::rust::string::*;
use sbor::rust::vec::Vec;
use sbor::*;
use utils::copy_u8_array;

use crate::abi::*;
use crate::engine::scrypto_env::ScryptoEnv;
use crate::runtime::{DataRef, DataRefMut};

/// A scalable key-value map which loads entries on demand.
pub struct KeyValueStore<
    K: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>,
    V: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>,
> {
    pub id: KeyValueStoreId,
    pub key: PhantomData<K>,
    pub value: PhantomData<V>,
}

// TODO: de-duplication
#[derive(Debug, Clone, TypeId, Encode, Decode, PartialEq, Eq)]
pub struct KeyValueStoreEntrySubstate(pub Option<Vec<u8>>);

impl<
        K: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>,
        V: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>,
    > KeyValueStore<K, V>
{
    /// Creates a new key value store.
    pub fn new() -> Self {
        let mut syscalls = ScryptoEnv;
        let id = syscalls
            .sys_create_node(ScryptoRENode::KeyValueStore)
            .unwrap();

        Self {
            id: id.into(),
            key: PhantomData,
            value: PhantomData,
        }
    }

    /// Returns the value that is associated with the given key.
    pub fn get(&self, key: &K) -> Option<DataRef<V>> {
        let mut syscalls = ScryptoEnv;
        let offset = SubstateOffset::KeyValueStore(KeyValueStoreOffset::Entry(scrypto_encode(key)));
        let lock_handle = syscalls
            .sys_lock_substate(RENodeId::KeyValueStore(self.id), offset, false)
            .unwrap();
        let raw_bytes = syscalls.sys_read(lock_handle).unwrap();
        let value: KeyValueStoreEntrySubstate = scrypto_decode(&raw_bytes).unwrap();

        if value.0.is_none() {
            syscalls.sys_drop_lock(lock_handle).unwrap();
        }

        value
            .0
            .map(|raw| DataRef::new(lock_handle, scrypto_decode(&raw).unwrap()))
    }

    pub fn get_mut(&mut self, key: &K) -> Option<DataRefMut<V>> {
        let mut syscalls = ScryptoEnv;
        let offset = SubstateOffset::KeyValueStore(KeyValueStoreOffset::Entry(scrypto_encode(key)));
        let lock_handle = syscalls
            .sys_lock_substate(RENodeId::KeyValueStore(self.id), offset.clone(), true)
            .unwrap();
        let raw_bytes = syscalls.sys_read(lock_handle).unwrap();
        let value: KeyValueStoreEntrySubstate = scrypto_decode(&raw_bytes).unwrap();

        if value.0.is_none() {
            syscalls.sys_drop_lock(lock_handle).unwrap();
        }

        value
            .0
            .map(|raw| DataRefMut::new(lock_handle, offset, scrypto_decode(&raw).unwrap()))
    }

    /// Inserts a new key-value pair into this map.
    pub fn insert(&self, key: K, value: V) {
        let mut syscalls = ScryptoEnv;
        let offset =
            SubstateOffset::KeyValueStore(KeyValueStoreOffset::Entry(scrypto_encode(&key)));
        let lock_handle = syscalls
            .sys_lock_substate(RENodeId::KeyValueStore(self.id), offset.clone(), true)
            .unwrap();
        let substate = KeyValueStoreEntrySubstate(Some(scrypto_encode(&value)));
        syscalls
            .sys_write(lock_handle, scrypto_encode(&substate))
            .unwrap();
        syscalls.sys_drop_lock(lock_handle).unwrap();
    }
}

//========
// error
//========

/// Represents an error when decoding key value store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseKeyValueStoreError {
    InvalidHex(String),
    InvalidLength(usize),
}

#[cfg(not(feature = "alloc"))]
impl std::error::Error for ParseKeyValueStoreError {}

#[cfg(not(feature = "alloc"))]
impl fmt::Display for ParseKeyValueStoreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

//========
// binary
//========

impl<
        K: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>,
        V: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>,
    > TryFrom<&[u8]> for KeyValueStore<K, V>
{
    type Error = ParseKeyValueStoreError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        match slice.len() {
            36 => Ok(Self {
                id: copy_u8_array(slice),
                key: PhantomData,
                value: PhantomData,
            }),
            _ => Err(ParseKeyValueStoreError::InvalidLength(slice.len())),
        }
    }
}

impl<
        K: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>,
        V: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>,
    > KeyValueStore<K, V>
{
    pub fn to_vec(&self) -> Vec<u8> {
        self.id.to_vec()
    }
}

// TODO: extend scrypto_type! macro to support generics

impl<
        K: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>,
        V: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>,
    > TypeId<ScryptoCustomTypeId> for KeyValueStore<K, V>
{
    #[inline]
    fn type_id() -> ScryptoTypeId {
        SborTypeId::Custom(ScryptoCustomTypeId::KeyValueStore)
    }
}

impl<
        K: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>,
        V: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>,
    > Encode<ScryptoCustomTypeId> for KeyValueStore<K, V>
{
    #[inline]
    fn encode_type_id(&self, encoder: &mut ScryptoEncoder) {
        encoder.write_type_id(Self::type_id());
    }

    #[inline]
    fn encode_body(&self, encoder: &mut ScryptoEncoder) {
        encoder.write_slice(&self.to_vec());
    }
}

impl<
        K: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>,
        V: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>,
    > Decode<ScryptoCustomTypeId> for KeyValueStore<K, V>
{
    fn decode_with_type_id(
        decoder: &mut ScryptoDecoder,
        type_id: ScryptoTypeId,
    ) -> Result<Self, DecodeError> {
        decoder.check_preloaded_type_id(type_id, Self::type_id())?;
        let slice = decoder.read_slice(36)?;
        Self::try_from(slice).map_err(|_| DecodeError::InvalidCustomValue)
    }
}

impl<
        K: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId> + Describe,
        V: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId> + Describe,
    > Describe for KeyValueStore<K, V>
{
    fn describe() -> Type {
        Type::KeyValueStore {
            key_type: Box::new(K::describe()),
            value_type: Box::new(V::describe()),
        }
    }
}

//======
// text
//======

impl<
        K: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>,
        V: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>,
    > FromStr for KeyValueStore<K, V>
{
    type Err = ParseKeyValueStoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes =
            hex::decode(s).map_err(|_| ParseKeyValueStoreError::InvalidHex(s.to_owned()))?;
        Self::try_from(bytes.as_slice())
    }
}

impl<
        K: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>,
        V: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>,
    > fmt::Display for KeyValueStore<K, V>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", hex::encode(self.to_vec()))
    }
}

impl<
        K: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>,
        V: Encode<ScryptoCustomTypeId> + Decode<ScryptoCustomTypeId>,
    > fmt::Debug for KeyValueStore<K, V>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self)
    }
}

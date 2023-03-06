use radix_engine_interface::api::component::KeyValueStoreEntrySubstate;
use radix_engine_interface::api::substate_api::LockFlags;
use radix_engine_interface::api::types::{
    KeyValueStoreId, KeyValueStoreOffset, RENodeId, SubstateOffset,
};
use radix_engine_interface::api::*;
use radix_engine_interface::data::scrypto::model::*;
use radix_engine_interface::data::scrypto::well_known_scrypto_custom_types::OWN_KEY_VALUE_STORE_ID;
use radix_engine_interface::data::scrypto::*;
use sbor::rust::marker::PhantomData;
use sbor::*;
use scrypto_schema::KeyValueStoreSchema;

use crate::engine::scrypto_env::ScryptoEnv;
use crate::runtime::{DataRef, DataRefMut, OriginalData};

// TODO: optimize `rust_value -> bytes -> scrypto_value` conversion.

/// A scalable key-value map which loads entries on demand.
pub struct KeyValueStore<K: ScryptoEncode + ScryptoDecode + ScryptoDescribe, V: ScryptoEncode + ScryptoDecode + ScryptoDescribe> {
    pub id: KeyValueStoreId,
    pub key: PhantomData<K>,
    pub value: PhantomData<V>,
}

impl<K: ScryptoEncode + ScryptoDecode + ScryptoDescribe, V: ScryptoEncode + ScryptoDecode + ScryptoDescribe> KeyValueStore<K, V> {
    /// Creates a new key value store.
    pub fn new() -> Self {
        let mut env = ScryptoEnv;

        let schema = KeyValueStoreSchema::new::<K, V>();

        let id = env.new_key_value_store(schema).unwrap();

        Self {
            id,
            key: PhantomData,
            value: PhantomData,
        }
    }

    /// Returns the value that is associated with the given key.
    pub fn get(&self, key: &K) -> Option<DataRef<V>> {
        let mut env = ScryptoEnv;
        let key_payload = scrypto_encode(key).unwrap();
        let offset = SubstateOffset::KeyValueStore(KeyValueStoreOffset::Entry(key_payload));
        let handle = env
            .sys_lock_substate(
                RENodeId::KeyValueStore(self.id),
                offset,
                LockFlags::read_only(),
            )
            .unwrap();
        let raw_bytes = env.sys_read_substate(handle).unwrap();

        // Decode and create Ref
        let substate: KeyValueStoreEntrySubstate = scrypto_decode(&raw_bytes).unwrap();
        match substate {
            KeyValueStoreEntrySubstate::Some(value) => Some(DataRef::new(
                handle,
                scrypto_decode(&scrypto_encode(&value).unwrap()).unwrap(),
            )),
            KeyValueStoreEntrySubstate::None => {
                env.sys_drop_lock(handle).unwrap();
                None
            }
        }
    }

    pub fn get_mut(&mut self, key: &K) -> Option<DataRefMut<V>> {
        let mut env = ScryptoEnv;
        let key_payload = scrypto_encode(key).unwrap();
        let offset = SubstateOffset::KeyValueStore(KeyValueStoreOffset::Entry(key_payload));
        let handle = env
            .sys_lock_substate(
                RENodeId::KeyValueStore(self.id),
                offset.clone(),
                LockFlags::MUTABLE,
            )
            .unwrap();
        let raw_bytes = env.sys_read_substate(handle).unwrap();

        // Decode and create RefMut
        let substate: KeyValueStoreEntrySubstate = scrypto_decode(&raw_bytes).unwrap();
        match substate {
            KeyValueStoreEntrySubstate::Some(value) => {
                let rust_value = scrypto_decode(&scrypto_encode(&value).unwrap()).unwrap();
                Some(DataRefMut::new(
                    handle,
                    OriginalData::KeyValueStoreEntry(value),
                    rust_value,
                ))
            }
            KeyValueStoreEntrySubstate::None => {
                env.sys_drop_lock(handle).unwrap();
                None
            }
        }
    }

    /// Inserts a new key-value pair into this map.
    pub fn insert(&self, key: K, value: V) {
        let mut env = ScryptoEnv;
        let key_payload = scrypto_encode(&key).unwrap();
        let value_payload = scrypto_encode(&value).unwrap();
        let offset = SubstateOffset::KeyValueStore(KeyValueStoreOffset::Entry(key_payload.clone()));
        let handle = env
            .sys_lock_substate(
                RENodeId::KeyValueStore(self.id),
                offset.clone(),
                LockFlags::MUTABLE,
            )
            .unwrap();
        env.sys_write_substate(
            handle,
            scrypto_encode(&KeyValueStoreEntrySubstate::Some(
                scrypto_decode(&value_payload).unwrap(),
            ))
            .unwrap(),
        )
        .unwrap();
        env.sys_drop_lock(handle).unwrap();
    }
}

//========
// binary
//========
impl<K: ScryptoEncode + ScryptoDecode + ScryptoDescribe, V: ScryptoEncode + ScryptoDecode + ScryptoDescribe>
    Categorize<ScryptoCustomValueKind> for KeyValueStore<K, V>
{
    #[inline]
    fn value_kind() -> ValueKind<ScryptoCustomValueKind> {
        ValueKind::Custom(ScryptoCustomValueKind::Own)
    }
}

impl<
        K: ScryptoEncode + ScryptoDecode + ScryptoDescribe,
        V: ScryptoEncode + ScryptoDecode + ScryptoDescribe,
        E: Encoder<ScryptoCustomValueKind>,
    > Encode<ScryptoCustomValueKind, E> for KeyValueStore<K, V>
{
    #[inline]
    fn encode_value_kind(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encoder.write_value_kind(Self::value_kind())
    }

    #[inline]
    fn encode_body(&self, encoder: &mut E) -> Result<(), EncodeError> {
        Own::KeyValueStore(self.id).encode_body(encoder)
    }
}

impl<
        K: ScryptoEncode + ScryptoDecode + ScryptoDescribe,
        V: ScryptoEncode + ScryptoDecode + ScryptoDescribe,
        D: Decoder<ScryptoCustomValueKind>,
    > Decode<ScryptoCustomValueKind, D> for KeyValueStore<K, V>
{
    fn decode_body_with_value_kind(
        decoder: &mut D,
        value_kind: ValueKind<ScryptoCustomValueKind>,
    ) -> Result<Self, DecodeError> {
        let own = Own::decode_body_with_value_kind(decoder, value_kind)?;
        match own {
            Own::KeyValueStore(_) => Ok(Self {
                id: own.kv_store_id(),
                key: PhantomData,
                value: PhantomData,
            }),
            _ => Err(DecodeError::InvalidCustomValue),
        }
    }
}

impl<K: ScryptoEncode + ScryptoDecode + ScryptoDescribe, V: ScryptoEncode + ScryptoDecode + ScryptoDescribe>
    Describe<ScryptoCustomTypeKind> for KeyValueStore<K, V>
{
    const TYPE_ID: GlobalTypeId = GlobalTypeId::WellKnown([OWN_KEY_VALUE_STORE_ID]);
}

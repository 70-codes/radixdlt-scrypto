use crate::blueprints::resource::*;
use crate::constants::SYSTEM_TOKEN;
use crate::crypto::PublicKey;
use sbor::rust::vec::Vec;

pub struct AuthAddresses;

impl AuthAddresses {
    pub fn system_role() -> NonFungibleGlobalId {
        NonFungibleGlobalId::new(SYSTEM_TOKEN, NonFungibleLocalId::Integer(0))
    }

    pub fn validator_role() -> NonFungibleGlobalId {
        NonFungibleGlobalId::new(SYSTEM_TOKEN, NonFungibleLocalId::Integer(1))
    }

    pub fn signer_set(signer_public_keys: &[PublicKey]) -> Vec<NonFungibleGlobalId> {
        signer_public_keys
            .iter()
            .map(NonFungibleGlobalId::from_public_key)
            .collect()
    }
}

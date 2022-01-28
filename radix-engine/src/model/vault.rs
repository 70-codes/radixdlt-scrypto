use sbor::*;
use scrypto::rust::vec::Vec;
use scrypto::types::*;

use crate::model::{Actor, Bucket, BucketError, Supply};

/// Represents an error when accessing a vault.
#[derive(Debug, Clone)]
pub enum VaultError {
    AccountingError(BucketError),
    UnauthorizedAccess,
}

/// A persistent resource container on ledger state.
#[derive(Debug, Clone, TypeId, Encode, Decode)]
pub struct Vault {
    bucket: Bucket,
    authority: Address,
}

impl Vault {
    pub fn new(bucket: Bucket, authority: Address) -> Self {
        Self { bucket, authority }
    }

    pub fn put(&mut self, other: Bucket, actor: Actor) -> Result<(), VaultError> {
        if actor.check(self.authority) {
            self.bucket.put(other).map_err(VaultError::AccountingError)
        } else {
            Err(VaultError::UnauthorizedAccess)
        }
    }

    pub fn take(&mut self, amount: Decimal, actor: Actor) -> Result<Bucket, VaultError> {
        if actor.check(self.authority) {
            self.bucket
                .take(amount)
                .map_err(VaultError::AccountingError)
        } else {
            Err(VaultError::UnauthorizedAccess)
        }
    }

    pub fn take_nft(&mut self, id: NftKey, actor: Actor) -> Result<Bucket, VaultError> {
        if actor.check(self.authority) {
            self.bucket
                .take_nft(id)
                .map_err(VaultError::AccountingError)
        } else {
            Err(VaultError::UnauthorizedAccess)
        }
    }

    pub fn get_nft_ids(&self, actor: Actor) -> Result<Vec<NftKey>, VaultError> {
        if actor.check(self.authority) {
            self.bucket
                .get_nft_ids()
                .map_err(VaultError::AccountingError)
        } else {
            Err(VaultError::UnauthorizedAccess)
        }
    }

    pub fn total_supply(&self, actor: Actor) -> Result<Supply, VaultError> {
        if actor.check(self.authority) {
            Ok(self.bucket.supply())
        } else {
            Err(VaultError::UnauthorizedAccess)
        }
    }

    pub fn amount(&self, actor: Actor) -> Result<Decimal, VaultError> {
        if actor.check(self.authority) {
            Ok(self.bucket.amount())
        } else {
            Err(VaultError::UnauthorizedAccess)
        }
    }

    pub fn resource_address(&self, actor: Actor) -> Result<Address, VaultError> {
        if actor.check(self.authority) {
            Ok(self.bucket.resource_address())
        } else {
            Err(VaultError::UnauthorizedAccess)
        }
    }
}

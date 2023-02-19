use radix_engine_interface::api::node_modules::auth::*;
use radix_engine_interface::api::node_modules::auth::{
    AccessRulesSetGroupAccessRuleInput, AccessRulesSetMethodAccessRuleInput,
};
use radix_engine_interface::api::node_modules::metadata::{
    MetadataGetInput, MetadataSetInput, METADATA_GET_IDENT, METADATA_SET_IDENT,
};
use radix_engine_interface::api::types::{NodeModuleId, ScryptoReceiver};
use radix_engine_interface::api::ClientComponentApi;
use radix_engine_interface::blueprints::resource::*;
use radix_engine_interface::data::{scrypto_decode, scrypto_encode};
use radix_engine_interface::math::Decimal;
use sbor::rust::collections::BTreeMap;
use sbor::rust::string::String;
use sbor::rust::string::ToString;
use sbor::rust::vec::Vec;
use scrypto::engine::scrypto_env::ScryptoEnv;

use crate::*;

/// Represents a resource manager.
#[derive(Debug)]
pub struct ResourceManager(pub(crate) ResourceAddress);

impl ResourceManager {
    pub fn set_metadata(&mut self, key: String, value: String) {
        ScryptoEnv
            .call_module_method(
                ScryptoReceiver::Resource(self.0),
                NodeModuleId::Metadata,
                METADATA_SET_IDENT,
                scrypto_encode(&MetadataSetInput { key, value }).unwrap(),
            )
            .unwrap();
    }

    pub fn get_metadata(&mut self, key: String) -> Option<String> {
        let rtn = ScryptoEnv
            .call_module_method(
                ScryptoReceiver::Resource(self.0),
                NodeModuleId::Metadata,
                METADATA_GET_IDENT,
                scrypto_encode(&MetadataGetInput { key }).unwrap(),
            )
            .unwrap();

        scrypto_decode(&rtn).unwrap()
    }

    pub fn set_mintable(&mut self, access_rule: AccessRule) {
        ScryptoEnv
            .call_module_method(
                ScryptoReceiver::Resource(self.0),
                NodeModuleId::AccessRules,
                ACCESS_RULES_SET_GROUP_ACCESS_RULE_IDENT,
                scrypto_encode(&AccessRulesSetGroupAccessRuleInput {
                    index: 0,
                    name: "mint".to_string(),
                    rule: access_rule,
                })
                .unwrap(),
            )
            .unwrap();
    }

    pub fn set_burnable(&mut self, access_rule: AccessRule) -> () {
        ScryptoEnv
            .call_module_method(
                ScryptoReceiver::Resource(self.0),
                NodeModuleId::AccessRules,
                ACCESS_RULES_SET_METHOD_ACCESS_RULE_IDENT,
                scrypto_encode(&AccessRulesSetMethodAccessRuleInput {
                    index: 0,
                    key: AccessRuleKey::new(
                        NodeModuleId::SELF,
                        RESOURCE_MANAGER_BURN_IDENT.to_string(),
                    ),
                    rule: AccessRuleEntry::AccessRule(access_rule),
                })
                .unwrap(),
            )
            .unwrap();
    }

    pub fn set_withdrawable(&mut self, access_rule: AccessRule) {
        let mut env = ScryptoEnv;
        let _rtn = env
            .call_method(
                ScryptoReceiver::Resource(self.0),
                RESOURCE_MANAGER_UPDATE_VAULT_AUTH_IDENT,
                scrypto_encode(&ResourceManagerUpdateVaultAuthInput {
                    method: VaultMethodAuthKey::Withdraw,
                    access_rule,
                })
                .unwrap(),
            )
            .unwrap();
    }

    pub fn set_depositable(&mut self, access_rule: AccessRule) {
        let mut env = ScryptoEnv;
        let _rtn = env
            .call_method(
                ScryptoReceiver::Resource(self.0),
                RESOURCE_MANAGER_UPDATE_VAULT_AUTH_IDENT,
                scrypto_encode(&ResourceManagerUpdateVaultAuthInput {
                    method: VaultMethodAuthKey::Deposit,
                    access_rule,
                })
                .unwrap(),
            )
            .unwrap();
    }

    pub fn set_recallable(&mut self, access_rule: AccessRule) {
        let mut env = ScryptoEnv;
        let _rtn = env
            .call_method(
                ScryptoReceiver::Resource(self.0),
                RESOURCE_MANAGER_UPDATE_VAULT_AUTH_IDENT,
                scrypto_encode(&ResourceManagerUpdateVaultAuthInput {
                    method: VaultMethodAuthKey::Recall,
                    access_rule,
                })
                .unwrap(),
            )
            .unwrap();
    }

    pub fn set_updateable_metadata(&self, access_rule: AccessRule) {
        ScryptoEnv
            .call_module_method(
                ScryptoReceiver::Resource(self.0),
                NodeModuleId::AccessRules,
                ACCESS_RULES_SET_METHOD_ACCESS_RULE_IDENT,
                scrypto_encode(&AccessRulesSetMethodAccessRuleInput {
                    index: 0,
                    key: AccessRuleKey::new(
                        NodeModuleId::Metadata,
                        METADATA_SET_IDENT.to_string(),
                    ),
                    rule: AccessRuleEntry::AccessRule(access_rule),
                })
                .unwrap(),
            )
            .unwrap();
    }

    pub fn set_updateable_non_fungible_data(&self, access_rule: AccessRule) {
        ScryptoEnv
            .call_module_method(
                ScryptoReceiver::Resource(self.0),
                NodeModuleId::AccessRules,
                ACCESS_RULES_SET_METHOD_ACCESS_RULE_IDENT,
                scrypto_encode(&AccessRulesSetMethodAccessRuleInput {
                    index: 0,
                    key: AccessRuleKey::new(
                        NodeModuleId::SELF,
                        RESOURCE_MANAGER_UPDATE_NON_FUNGIBLE_DATA_IDENT.to_string(),
                    ),
                    rule: AccessRuleEntry::AccessRule(access_rule),
                })
                .unwrap(),
            )
            .unwrap();
    }

    pub fn lock_mintable(&mut self) {
        ScryptoEnv
            .call_module_method(
                ScryptoReceiver::Resource(self.0),
                NodeModuleId::AccessRules,
                ACCESS_RULES_SET_GROUP_MUTABILITY_IDENT,
                scrypto_encode(&AccessRulesSetGroupMutabilityInput {
                    index: 0,
                    name: "mint".to_string(),
                    mutability: AccessRule::DenyAll,
                })
                .unwrap(),
            )
            .unwrap();
    }

    pub fn lock_burnable(&mut self) {
        ScryptoEnv
            .call_module_method(
                ScryptoReceiver::Resource(self.0),
                NodeModuleId::AccessRules,
                ACCESS_RULES_SET_METHOD_MUTABILITY_IDENT,
                scrypto_encode(&AccessRulesSetMethodMutabilityInput {
                    index: 0,
                    key: AccessRuleKey::new(
                        NodeModuleId::SELF,
                        RESOURCE_MANAGER_BURN_IDENT.to_string(),
                    ),
                    mutability: AccessRule::DenyAll,
                })
                .unwrap(),
            )
            .unwrap();
    }

    pub fn lock_updateable_metadata(&mut self) {
        ScryptoEnv
            .call_module_method(
                ScryptoReceiver::Resource(self.0),
                NodeModuleId::AccessRules,
                ACCESS_RULES_SET_METHOD_MUTABILITY_IDENT,
                scrypto_encode(&AccessRulesSetMethodMutabilityInput {
                    index: 0,
                    key: AccessRuleKey::new(
                        NodeModuleId::Metadata,
                        METADATA_SET_IDENT.to_string(),
                    ),
                    mutability: AccessRule::DenyAll,
                })
                .unwrap(),
            )
            .unwrap();
    }

    pub fn lock_updateable_non_fungible_data(&mut self) {
        ScryptoEnv
            .call_module_method(
                ScryptoReceiver::Resource(self.0),
                NodeModuleId::AccessRules,
                ACCESS_RULES_SET_METHOD_MUTABILITY_IDENT,
                scrypto_encode(&AccessRulesSetMethodMutabilityInput {
                    index: 0,
                    key: AccessRuleKey::new(
                        NodeModuleId::SELF,
                        RESOURCE_MANAGER_UPDATE_NON_FUNGIBLE_DATA_IDENT.to_string(),
                    ),
                    mutability: AccessRule::DenyAll,
                })
                .unwrap(),
            )
            .unwrap();
    }

    pub fn lock_withdrawable(&mut self) {
        let mut env = ScryptoEnv;
        let _rtn = env
            .call_method(
                ScryptoReceiver::Resource(self.0),
                RESOURCE_MANAGER_SET_VAULT_AUTH_MUTABILITY_IDENT,
                scrypto_encode(&ResourceManagerSetVaultAuthMutabilityInput {
                    method: VaultMethodAuthKey::Withdraw,
                    mutability: AccessRule::DenyAll,
                })
                .unwrap(),
            )
            .unwrap();
    }

    pub fn lock_depositable(&mut self) {
        let mut env = ScryptoEnv;
        let _rtn = env
            .call_method(
                ScryptoReceiver::Resource(self.0),
                RESOURCE_MANAGER_SET_VAULT_AUTH_MUTABILITY_IDENT,
                scrypto_encode(&ResourceManagerSetVaultAuthMutabilityInput {
                    method: VaultMethodAuthKey::Deposit,
                    mutability: AccessRule::DenyAll,
                })
                .unwrap(),
            )
            .unwrap();
    }

    pub fn lock_recallable(&mut self) {
        let mut env = ScryptoEnv;
        let _rtn = env
            .call_method(
                ScryptoReceiver::Resource(self.0),
                RESOURCE_MANAGER_SET_VAULT_AUTH_MUTABILITY_IDENT,
                scrypto_encode(&ResourceManagerSetVaultAuthMutabilityInput {
                    method: VaultMethodAuthKey::Recall,
                    mutability: AccessRule::DenyAll,
                })
                .unwrap(),
            )
            .unwrap();
    }

    fn update_non_fungible_data_internal(&mut self, id: NonFungibleLocalId, data: Vec<u8>) {
        let mut env = ScryptoEnv;
        let _rtn = env
            .call_method(
                ScryptoReceiver::Resource(self.0),
                RESOURCE_MANAGER_UPDATE_NON_FUNGIBLE_DATA_IDENT,
                scrypto_encode(&ResourceManagerUpdateNonFungibleDataInput { id, data }).unwrap(),
            )
            .unwrap();
    }

    fn get_non_fungible_data_internal(&self, id: NonFungibleLocalId) -> [Vec<u8>; 2] {
        let mut env = ScryptoEnv;
        let rtn = env
            .call_method(
                ScryptoReceiver::Resource(self.0),
                RESOURCE_MANAGER_GET_NON_FUNGIBLE_IDENT,
                scrypto_encode(&ResourceManagerGetNonFungibleInput { id }).unwrap(),
            )
            .unwrap();
        scrypto_decode(&rtn).unwrap()
    }

    pub fn resource_type(&self) -> ResourceType {
        let mut env = ScryptoEnv;
        let rtn = env
            .call_method(
                ScryptoReceiver::Resource(self.0),
                RESOURCE_MANAGER_GET_RESOURCE_TYPE_IDENT,
                scrypto_encode(&ResourceManagerGetResourceTypeInput {}).unwrap(),
            )
            .unwrap();
        scrypto_decode(&rtn).unwrap()
    }

    pub fn total_supply(&self) -> Decimal {
        let mut env = ScryptoEnv;
        let rtn = env
            .call_method(
                ScryptoReceiver::Resource(self.0),
                RESOURCE_MANAGER_GET_TOTAL_SUPPLY_IDENT,
                scrypto_encode(&ResourceManagerGetTotalSupplyInput {}).unwrap(),
            )
            .unwrap();
        scrypto_decode(&rtn).unwrap()
    }

    pub fn non_fungible_exists(&self, id: &NonFungibleLocalId) -> bool {
        let mut env = ScryptoEnv;

        let rtn = env
            .call_method(
                ScryptoReceiver::Resource(self.0),
                RESOURCE_MANAGER_NON_FUNGIBLE_EXISTS_IDENT,
                scrypto_encode(&ResourceManagerNonFungibleExistsInput { id: id.clone() }).unwrap(),
            )
            .unwrap();

        scrypto_decode(&rtn).unwrap()
    }

    pub fn burn(&mut self, bucket: Bucket) {
        let mut env = ScryptoEnv;

        let _rtn = env
            .call_method(
                ScryptoReceiver::Resource(self.0),
                RESOURCE_MANAGER_BURN_IDENT,
                scrypto_encode(&ResourceManagerBurnInput {
                    bucket: Bucket(bucket.0),
                })
                .unwrap(),
            )
            .unwrap();
    }

    /// Mints fungible resources
    pub fn mint<T: Into<Decimal>>(&mut self, amount: T) -> Bucket {
        let mut env = ScryptoEnv;

        let rtn = env
            .call_method(
                ScryptoReceiver::Resource(self.0),
                RESOURCE_MANAGER_MINT_FUNGIBLE,
                scrypto_encode(&ResourceManagerMintFungibleInput {
                    amount: amount.into(),
                })
                .unwrap(),
            )
            .unwrap();

        scrypto_decode(&rtn).unwrap()
    }

    /// Mints non-fungible resources
    pub fn mint_non_fungible<T: NonFungibleData>(
        &mut self,
        id: &NonFungibleLocalId,
        data: T,
    ) -> Bucket {
        let mut entries = BTreeMap::new();
        entries.insert(
            id.clone(),
            (data.immutable_data().unwrap(), data.mutable_data().unwrap()),
        );
        let mut env = ScryptoEnv;
        let rtn = env
            .call_method(
                ScryptoReceiver::Resource(self.0),
                RESOURCE_MANAGER_MINT_NON_FUNGIBLE,
                scrypto_encode(&ResourceManagerMintNonFungibleInput { entries }).unwrap(),
            )
            .unwrap();

        scrypto_decode(&rtn).unwrap()
    }

    /// Mints uuid non-fungible resources
    pub fn mint_uuid_non_fungible<T: NonFungibleData>(&mut self, data: T) -> Bucket {
        let mut entries = Vec::new();
        entries.push((data.immutable_data().unwrap(), data.mutable_data().unwrap()));
        let mut env = ScryptoEnv;

        let rtn = env
            .call_method(
                ScryptoReceiver::Resource(self.0),
                RESOURCE_MANAGER_MINT_UUID_NON_FUNGIBLE,
                scrypto_encode(&ResourceManagerMintUuidNonFungibleInput { entries }).unwrap(),
            )
            .unwrap();

        scrypto_decode(&rtn).unwrap()
    }

    /// Returns the data of a non-fungible unit, both the immutable and mutable parts.
    ///
    /// # Panics
    /// Panics if this is not a non-fungible resource or the specified non-fungible is not found.
    pub fn get_non_fungible_data<T: NonFungibleData>(&self, id: &NonFungibleLocalId) -> T {
        let non_fungible = self.get_non_fungible_data_internal(id.clone());
        T::decode(&non_fungible[0], &non_fungible[1]).unwrap()
    }

    /// Updates the mutable part of a non-fungible unit.
    ///
    /// # Panics
    /// Panics if this is not a non-fungible resource or the specified non-fungible is not found.
    pub fn update_non_fungible_data<T: NonFungibleData>(
        &mut self,
        id: &NonFungibleLocalId,
        new_data: T,
    ) {
        self.update_non_fungible_data_internal(id.clone(), new_data.mutable_data().unwrap())
    }
}

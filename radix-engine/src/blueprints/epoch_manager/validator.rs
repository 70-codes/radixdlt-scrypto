use crate::blueprints::epoch_manager::EpochManagerSubstate;
use crate::blueprints::util::{MethodType, SecurifiedAccessRules};
use crate::errors::ApplicationError;
use crate::errors::RuntimeError;
use crate::kernel::kernel_api::KernelNodeApi;
use crate::types::*;
use native_sdk::modules::metadata::Metadata;
use native_sdk::modules::royalty::ComponentRoyalty;
use native_sdk::resource::{ResourceManager, SysBucket, Vault};
use native_sdk::runtime::Runtime;
use radix_engine_interface::api::node_modules::auth::{
    AccessRulesSetMethodAccessRuleInput, ACCESS_RULES_SET_METHOD_ACCESS_RULE_IDENT,
};
use radix_engine_interface::api::object_api::ObjectModuleId;
use radix_engine_interface::api::sorted_store_api::SortedKey;
use radix_engine_interface::api::substate_api::LockFlags;
use radix_engine_interface::api::ClientApi;
use radix_engine_interface::blueprints::epoch_manager::*;
use radix_engine_interface::blueprints::resource::*;
use radix_engine_interface::rule;

use super::{
    ClaimXrdEvent, RegisterValidatorEvent, StakeEvent, UnregisterValidatorEvent, UnstakeEvent,
    UpdateAcceptingStakeDelegationStateEvent,
};

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub struct ValidatorSubstate {
    pub sorted_key: Option<SortedKey>,
    pub key: EcdsaSecp256k1PublicKey,
    pub is_registered: bool,

    pub unstake_nft: ResourceAddress,
    pub liquidity_token: ResourceAddress,
    pub stake_xrd_vault_id: Own,
    pub pending_xrd_withdraw_vault_id: Own,
}

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub struct UnstakeData {
    epoch_unlocked: u64,
    amount: Decimal,
}

impl NonFungibleData for UnstakeData {
    const MUTABLE_FIELDS: &'static [&'static str] = &[];
}

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub enum ValidatorError {
    InvalidClaimResource,
    EpochUnlockHasNotOccurredYet,
}

pub struct ValidatorBlueprint;

impl ValidatorBlueprint {
    pub fn register<Y>(receiver: &NodeId, api: &mut Y) -> Result<(), RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        Self::register_update(receiver, true, api)
    }

    pub fn unregister<Y>(receiver: &NodeId, api: &mut Y) -> Result<(), RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        Self::register_update(receiver, false, api)
    }

    pub fn stake<Y>(receiver: &NodeId, stake: Bucket, api: &mut Y) -> Result<Bucket, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        // Prepare the event and emit it once the operations succeed
        let event = {
            let amount = stake.sys_amount(api)?;
            StakeEvent { xrd_staked: amount }
        };

        let handle = api.lock_field(
            receiver,
            &ValidatorOffset::Validator.into(),
            LockFlags::MUTABLE,
        )?;

        let mut validator: ValidatorSubstate = api.sys_read_substate_typed(handle)?;

        // Stake
        let (lp_token_bucket, new_stake_amount) = {
            let mut lp_token_resman = ResourceManager(validator.liquidity_token);
            let mut xrd_vault = Vault(validator.stake_xrd_vault_id);

            let total_lp_supply = lp_token_resman.total_supply(api)?;
            let active_stake_amount = xrd_vault.sys_amount(api)?;
            let xrd_bucket = stake;
            let stake_amount = xrd_bucket.sys_amount(api)?;
            let lp_mint_amount = if active_stake_amount.is_zero() {
                stake_amount
            } else {
                stake_amount * total_lp_supply / active_stake_amount
            };

            let lp_token_bucket = lp_token_resman.mint_fungible(lp_mint_amount, api)?;
            xrd_vault.sys_put(xrd_bucket, api)?;
            let new_stake_amount = xrd_vault.sys_amount(api)?;
            (lp_token_bucket, new_stake_amount)
        };

        // Update EpochManager
        let new_index_key = Self::index_update(
            receiver,
            &validator,
            validator.is_registered,
            new_stake_amount,
            api,
        )?;

        validator.sorted_key = new_index_key;
        api.sys_write_substate_typed(handle, &validator)?;
        Runtime::emit_event(api, event)?;

        Ok(lp_token_bucket)
    }

    pub fn unstake<Y>(
        receiver: &NodeId,
        lp_tokens: Bucket,
        api: &mut Y,
    ) -> Result<Bucket, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        // Prepare event and emit it once operations finish
        let event = {
            let amount = lp_tokens.sys_amount(api)?;
            UnstakeEvent {
                stake_units: amount,
            }
        };

        let handle = api.lock_field(
            receiver,
            &ValidatorOffset::Validator.into(),
            LockFlags::MUTABLE,
        )?;

        let manager = api.get_object_info(receiver)?.type_parent.unwrap();

        let mut validator: ValidatorSubstate = api.sys_read_substate_typed(handle)?;

        // Unstake
        let (unstake_bucket, new_stake_amount) = {
            let mut stake_vault = Vault(validator.stake_xrd_vault_id);
            let mut unstake_vault = Vault(validator.pending_xrd_withdraw_vault_id);
            let nft_resman = ResourceManager(validator.unstake_nft);
            let mut lp_token_resman = ResourceManager(validator.liquidity_token);

            let active_stake_amount = stake_vault.sys_amount(api)?;
            let total_lp_supply = lp_token_resman.total_supply(api)?;
            let lp_token_amount = lp_tokens.sys_amount(api)?;
            let xrd_amount = if total_lp_supply.is_zero() {
                Decimal::zero()
            } else {
                lp_token_amount * active_stake_amount / total_lp_supply
            };

            lp_token_resman.burn(lp_tokens, api)?;

            let manager_handle = api.lock_field(
                manager.as_node_id(),
                &EpochManagerOffset::EpochManager.into(),
                LockFlags::read_only(),
            )?;
            let epoch_manager: EpochManagerSubstate =
                api.sys_read_substate_typed(manager_handle)?;
            let current_epoch = epoch_manager.epoch;
            let epoch_unlocked = current_epoch + epoch_manager.num_unstake_epochs;
            api.sys_drop_lock(manager_handle)?;

            let data = UnstakeData {
                epoch_unlocked,
                amount: xrd_amount,
            };

            let bucket = stake_vault.sys_take(xrd_amount, api)?;
            unstake_vault.sys_put(bucket, api)?;
            let (unstake_bucket, _) = nft_resman.mint_non_fungible_single_uuid(data, api)?;

            let new_stake_amount = stake_vault.sys_amount(api)?;

            (unstake_bucket, new_stake_amount)
        };

        // Update EpochManager
        let new_index_key = Self::index_update(
            receiver,
            &validator,
            validator.is_registered,
            new_stake_amount,
            api,
        )?;

        validator.sorted_key = new_index_key;
        api.sys_write_substate_typed(handle, &validator)?;
        Runtime::emit_event(api, event)?;

        Ok(unstake_bucket)
    }

    fn register_update<Y>(
        receiver: &NodeId,
        new_registered: bool,
        api: &mut Y,
    ) -> Result<(), RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let substate_key = ValidatorOffset::Validator.into();
        let handle = api.lock_field(receiver, &substate_key, LockFlags::MUTABLE)?;

        let mut validator: ValidatorSubstate = api.sys_read_substate_typed(handle)?;
        // No update
        if validator.is_registered == new_registered {
            return Ok(());
        }

        let stake_amount = {
            let stake_vault = Vault(validator.stake_xrd_vault_id);
            stake_vault.sys_amount(api)?
        };

        let index_key =
            Self::index_update(receiver, &validator, new_registered, stake_amount, api)?;

        validator.is_registered = new_registered;
        validator.sorted_key = index_key;
        api.sys_write_substate_typed(handle, &validator)?;

        if new_registered {
            Runtime::emit_event(api, RegisterValidatorEvent)?;
        } else {
            Runtime::emit_event(api, UnregisterValidatorEvent)?;
        }

        return Ok(());
    }

    fn index_update<Y>(
        receiver: &NodeId,
        validator: &ValidatorSubstate,
        new_registered: bool,
        new_stake_amount: Decimal,
        api: &mut Y,
    ) -> Result<Option<SortedKey>, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let validator_address: ComponentAddress =
            ComponentAddress::new_unchecked(api.get_global_address()?.into());
        let new_sorted_key =
            Self::to_sorted_key(new_registered, new_stake_amount, validator_address);

        let update = if let Some(cur_index_key) = &validator.sorted_key {
            if let Some(new_index_key) = &new_sorted_key {
                Some(UpdateSecondaryIndex::UpdateStake {
                    index_key: cur_index_key.clone(),
                    new_index_key: new_index_key.clone(),
                    new_stake_amount,
                })
            } else {
                Some(UpdateSecondaryIndex::Remove {
                    index_key: cur_index_key.clone(),
                })
            }
        } else {
            if let Some(new_index_key) = &new_sorted_key {
                Some(UpdateSecondaryIndex::Create {
                    index_key: new_index_key.clone(),
                    stake: new_stake_amount,
                    primary: validator_address,
                    key: validator.key,
                })
            } else {
                None
            }
        };

        if let Some(update) = update {
            let manager = api.get_object_info(receiver)?.type_parent.unwrap();
            api.call_method(
                manager.as_node_id(),
                EPOCH_MANAGER_UPDATE_VALIDATOR_IDENT,
                scrypto_encode(&EpochManagerUpdateValidatorInput { update }).unwrap(),
            )?;
        }

        Ok(new_sorted_key)
    }

    pub fn claim_xrd<Y>(
        receiver: &NodeId,
        bucket: Bucket,
        api: &mut Y,
    ) -> Result<Bucket, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let handle = api.lock_field(
            receiver,
            &ValidatorOffset::Validator.into(),
            LockFlags::read_only(),
        )?;
        let validator: ValidatorSubstate = api.sys_read_substate_typed(handle)?;
        let mut nft_resman = ResourceManager(validator.unstake_nft);
        let resource_address = validator.unstake_nft;
        let manager = api.get_object_info(receiver)?.type_parent.unwrap();
        let mut unstake_vault = Vault(validator.pending_xrd_withdraw_vault_id);

        // TODO: Move this check into a more appropriate place
        if !resource_address.eq(&bucket.sys_resource_address(api)?) {
            return Err(RuntimeError::ApplicationError(
                ApplicationError::ValidatorError(ValidatorError::InvalidClaimResource),
            ));
        }

        let current_epoch = {
            let mgr_handle = api.lock_field(
                manager.as_node_id(),
                &EpochManagerOffset::EpochManager.into(),
                LockFlags::read_only(),
            )?;
            let mgr_substate: EpochManagerSubstate = api.sys_read_substate_typed(mgr_handle)?;
            let epoch = mgr_substate.epoch;
            api.sys_drop_lock(mgr_handle)?;
            epoch
        };

        let mut unstake_amount = Decimal::zero();

        for id in bucket.sys_non_fungible_local_ids(api)? {
            let data: UnstakeData = nft_resman.get_non_fungible_data(id, api)?;
            if current_epoch < data.epoch_unlocked {
                return Err(RuntimeError::ApplicationError(
                    ApplicationError::ValidatorError(ValidatorError::EpochUnlockHasNotOccurredYet),
                ));
            }
            unstake_amount += data.amount;
        }
        nft_resman.burn(bucket, api)?;

        let claimed_bucket = unstake_vault.sys_take(unstake_amount, api)?;

        let amount = claimed_bucket.sys_amount(api)?;
        Runtime::emit_event(
            api,
            ClaimXrdEvent {
                claimed_xrd: amount,
            },
        )?;

        Ok(claimed_bucket)
    }

    pub fn update_key<Y>(
        receiver: &NodeId,
        key: EcdsaSecp256k1PublicKey,
        api: &mut Y,
    ) -> Result<(), RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let handle = api.lock_field(
            receiver,
            &ValidatorOffset::Validator.into(),
            LockFlags::MUTABLE,
        )?;
        let mut validator: ValidatorSubstate = api.sys_read_substate_typed(handle)?;

        // Update Epoch Manager
        {
            if let Some(index_key) = &validator.sorted_key {
                let manager = api.get_object_info(receiver)?.type_parent.unwrap();
                let update = UpdateSecondaryIndex::UpdatePublicKey {
                    index_key: index_key.clone(),
                    key,
                };
                api.call_method(
                    manager.as_node_id(),
                    EPOCH_MANAGER_UPDATE_VALIDATOR_IDENT,
                    scrypto_encode(&EpochManagerUpdateValidatorInput { update }).unwrap(),
                )?;
            }
        }

        validator.key = key;
        api.sys_write_substate_typed(handle, &validator)?;

        Ok(())
    }

    pub fn update_accept_delegated_stake<Y>(
        receiver: &NodeId,
        accept_delegated_stake: bool,
        api: &mut Y,
    ) -> Result<(), RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let rule = if accept_delegated_stake {
            AccessRuleEntry::AccessRule(AccessRule::AllowAll)
        } else {
            AccessRuleEntry::Group("owner".to_string())
        };

        api.call_module_method(
            receiver,
            ObjectModuleId::AccessRules,
            ACCESS_RULES_SET_METHOD_ACCESS_RULE_IDENT,
            scrypto_encode(&AccessRulesSetMethodAccessRuleInput {
                object_key: ObjectKey::SELF,
                method_key: MethodKey::new(ObjectModuleId::SELF, VALIDATOR_STAKE_IDENT),
                rule,
            })
            .unwrap(),
        )?;

        Runtime::emit_event(
            api,
            UpdateAcceptingStakeDelegationStateEvent {
                accepts_delegation: accept_delegated_stake,
            },
        )?;

        Ok(())
    }

    fn to_sorted_key(
        registered: bool,
        stake: Decimal,
        address: ComponentAddress,
    ) -> Option<SortedKey> {
        if !registered || stake.is_zero() {
            None
        } else {
            let stake_100k =
                stake / Decimal::from(10).powi(Decimal::SCALE.into()) / Decimal::from(100000);
            let stake_100k_le = stake_100k.to_vec();
            let bytes = [stake_100k_le[1], stake_100k_le[0]];
            let reverse_stake_u16 = u16::from_be_bytes(bytes);
            let sorted_key = u16::MAX - reverse_stake_u16;
            Some(SortedKey::new(
                sorted_key,
                scrypto_encode(&address).unwrap(),
            ))
        }
    }
}

struct SecurifiedValidator;

impl SecurifiedAccessRules for SecurifiedValidator {
    const SECURIFY_IDENT: Option<&'static str> = None;
    const OWNER_GROUP_NAME: &'static str = "owner";
    const OWNER_TOKEN: ResourceAddress = VALIDATOR_OWNER_TOKEN;

    fn non_owner_methods() -> Vec<(&'static str, MethodType)> {
        let non_fungible_global_id = NonFungibleGlobalId::package_actor(EPOCH_MANAGER_PACKAGE);
        vec![
            (VALIDATOR_UNSTAKE_IDENT, MethodType::Public),
            (VALIDATOR_CLAIM_XRD_IDENT, MethodType::Public),
            (
                VALIDATOR_STAKE_IDENT,
                MethodType::Custom(
                    AccessRuleEntry::group(Self::OWNER_GROUP_NAME),
                    AccessRuleEntry::AccessRule(rule!(require(non_fungible_global_id))),
                ),
            ),
        ]
    }
}

pub(crate) struct ValidatorCreator;

impl ValidatorCreator {
    fn create_liquidity_token_with_initial_amount<Y>(
        amount: Decimal,
        api: &mut Y,
    ) -> Result<(ResourceAddress, Bucket), RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let mut liquidity_token_auth = BTreeMap::new();
        let non_fungible_id =
            NonFungibleLocalId::bytes(scrypto_encode(&EPOCH_MANAGER_PACKAGE).unwrap()).unwrap();
        let non_fungible_global_id = NonFungibleGlobalId::new(PACKAGE_TOKEN, non_fungible_id);
        liquidity_token_auth.insert(
            Mint,
            (
                rule!(require(non_fungible_global_id.clone())),
                rule!(deny_all),
            ),
        );
        liquidity_token_auth.insert(
            Burn,
            (rule!(require(non_fungible_global_id)), rule!(deny_all)),
        );
        liquidity_token_auth.insert(Withdraw, (rule!(allow_all), rule!(deny_all)));
        liquidity_token_auth.insert(Deposit, (rule!(allow_all), rule!(deny_all)));

        let (liquidity_token_resource_manager, bucket) =
            ResourceManager::new_fungible_with_initial_supply(
                18,
                amount,
                BTreeMap::new(),
                liquidity_token_auth,
                api,
            )?;

        Ok((liquidity_token_resource_manager.0, bucket))
    }

    fn create_liquidity_token<Y>(api: &mut Y) -> Result<ResourceAddress, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let mut liquidity_token_auth = BTreeMap::new();
        let non_fungible_local_id =
            NonFungibleLocalId::bytes(scrypto_encode(&EPOCH_MANAGER_PACKAGE).unwrap()).unwrap();
        let non_fungible_global_id = NonFungibleGlobalId::new(PACKAGE_TOKEN, non_fungible_local_id);

        liquidity_token_auth.insert(
            Mint,
            (
                rule!(require(non_fungible_global_id.clone())),
                rule!(deny_all),
            ),
        );
        liquidity_token_auth.insert(
            Burn,
            (rule!(require(non_fungible_global_id)), rule!(deny_all)),
        );
        liquidity_token_auth.insert(Withdraw, (rule!(allow_all), rule!(deny_all)));
        liquidity_token_auth.insert(Deposit, (rule!(allow_all), rule!(deny_all)));

        let liquidity_token_resource_manager =
            ResourceManager::new_fungible(18, BTreeMap::new(), liquidity_token_auth, api)?;

        Ok(liquidity_token_resource_manager.0)
    }

    fn create_unstake_nft<Y>(api: &mut Y) -> Result<ResourceAddress, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let mut unstake_token_auth = BTreeMap::new();
        let non_fungible_local_id =
            NonFungibleLocalId::bytes(scrypto_encode(&EPOCH_MANAGER_PACKAGE).unwrap()).unwrap();
        let non_fungible_global_id = NonFungibleGlobalId::new(PACKAGE_TOKEN, non_fungible_local_id);

        unstake_token_auth.insert(
            Mint,
            (
                rule!(require(non_fungible_global_id.clone())),
                rule!(deny_all),
            ),
        );
        unstake_token_auth.insert(
            Burn,
            (rule!(require(non_fungible_global_id)), rule!(deny_all)),
        );
        unstake_token_auth.insert(Withdraw, (rule!(allow_all), rule!(deny_all)));
        unstake_token_auth.insert(Deposit, (rule!(allow_all), rule!(deny_all)));

        let unstake_resource_manager =
            ResourceManager::new_non_fungible::<UnstakeData, Y, RuntimeError>(
                NonFungibleIdType::UUID,
                BTreeMap::new(),
                unstake_token_auth,
                api,
            )?;

        Ok(unstake_resource_manager.0)
    }

    pub fn create_with_stake<Y>(
        key: EcdsaSecp256k1PublicKey,
        initial_stake: Bucket,
        is_registered: bool,
        api: &mut Y,
    ) -> Result<(ComponentAddress, Bucket, Bucket, Option<SortedKey>), RuntimeError>
    where
        Y: KernelNodeApi + ClientApi<RuntimeError>,
    {
        let global_node_id = api.kernel_allocate_node_id(EntityType::GlobalValidator)?;
        let address: ComponentAddress = ComponentAddress::new_unchecked(global_node_id.into());
        let initial_liquidity_amount = initial_stake.sys_amount(api)?;
        let mut stake_vault = Vault::sys_new(RADIX_TOKEN, api)?;
        stake_vault.sys_put(initial_stake, api)?;
        let unstake_vault = Vault::sys_new(RADIX_TOKEN, api)?;
        let unstake_nft = Self::create_unstake_nft(api)?;
        let (liquidity_token, liquidity_bucket) =
            Self::create_liquidity_token_with_initial_amount(initial_liquidity_amount, api)?;

        let sorted_key =
            ValidatorBlueprint::to_sorted_key(is_registered, initial_liquidity_amount, address);

        let substate = ValidatorSubstate {
            sorted_key: sorted_key.clone(),
            key,
            liquidity_token,
            unstake_nft,
            stake_xrd_vault_id: stake_vault.0,
            pending_xrd_withdraw_vault_id: unstake_vault.0,
            is_registered,
        };

        let validator_id = api.new_object(
            VALIDATOR_BLUEPRINT,
            vec![scrypto_encode(&substate).unwrap()],
        )?;

        let (access_rules, owner_token_bucket) = SecurifiedValidator::create_securified(api)?;
        let metadata = Metadata::sys_create(api)?;
        let royalty = ComponentRoyalty::sys_create(RoyaltyConfig::default(), api)?;

        api.globalize_with_address(
            btreemap!(
                ObjectModuleId::SELF => validator_id,
                ObjectModuleId::AccessRules => access_rules.0.0,
                ObjectModuleId::Metadata => metadata.0,
                ObjectModuleId::Royalty => royalty.0,
            ),
            address.into(),
        )?;
        Ok((
            address.into(),
            liquidity_bucket,
            owner_token_bucket,
            sorted_key,
        ))
    }

    pub fn create<Y>(
        key: EcdsaSecp256k1PublicKey,
        is_registered: bool,
        api: &mut Y,
    ) -> Result<(ComponentAddress, Bucket), RuntimeError>
    where
        Y: KernelNodeApi + ClientApi<RuntimeError>,
    {
        let global_node_id = api.kernel_allocate_node_id(EntityType::GlobalValidator)?;
        let address = ComponentAddress::new_unchecked(global_node_id.into());
        let stake_vault = Vault::sys_new(RADIX_TOKEN, api)?;
        let unstake_vault = Vault::sys_new(RADIX_TOKEN, api)?;
        let unstake_nft = Self::create_unstake_nft(api)?;
        let liquidity_token = Self::create_liquidity_token(api)?;

        let substate = ValidatorSubstate {
            sorted_key: None,
            key,
            liquidity_token,
            unstake_nft,
            stake_xrd_vault_id: stake_vault.0,
            pending_xrd_withdraw_vault_id: unstake_vault.0,
            is_registered,
        };

        let validator_id = api.new_object(
            VALIDATOR_BLUEPRINT,
            vec![scrypto_encode(&substate).unwrap()],
        )?;

        let (access_rules, owner_token_bucket) = SecurifiedValidator::create_securified(api)?;
        let metadata = Metadata::sys_create(api)?;
        let royalty = ComponentRoyalty::sys_create(RoyaltyConfig::default(), api)?;

        api.globalize_with_address(
            btreemap!(
                ObjectModuleId::SELF => validator_id,
                ObjectModuleId::AccessRules => access_rules.0.0,
                ObjectModuleId::Metadata => metadata.0,
                ObjectModuleId::Royalty => royalty.0,
            ),
            address.into(),
        )?;
        Ok((address.into(), owner_token_bucket))
    }
}

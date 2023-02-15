use super::ValidatorCreator;
use crate::errors::RuntimeError;
use crate::errors::{ApplicationError, InterpreterError};
use crate::kernel::actor::ResolvedActor;
use crate::kernel::call_frame::CallFrameUpdate;
use crate::kernel::interpreters::deref_and_update;
use crate::kernel::kernel_api::LockFlags;
use crate::kernel::kernel_api::{ExecutableInvocation, Executor, KernelNodeApi, KernelSubstateApi};
use crate::system::global::GlobalAddressSubstate;
use crate::system::kernel_modules::auth::*;
use crate::system::node::RENodeInit;
use crate::system::node::RENodeModuleInit;
use crate::system::node_modules::auth::AccessRulesChainSubstate;
use crate::types::*;
use crate::wasm::WasmEngine;
use native_sdk::resource::{ResourceManager, SysBucket};
use radix_engine_interface::api::node_modules::auth::AuthAddresses;
use radix_engine_interface::api::types::*;
use radix_engine_interface::api::ClientNativeInvokeApi;
use radix_engine_interface::api::{ClientApi, ClientDerefApi, ClientSubstateApi};
use radix_engine_interface::blueprints::account::AccountDepositInvocation;
use radix_engine_interface::blueprints::epoch_manager::*;
use radix_engine_interface::blueprints::resource::*;
use radix_engine_interface::data::ScryptoValue;
use radix_engine_interface::rule;

#[derive(Debug, Clone, PartialEq, Eq, ScryptoCategorize, ScryptoEncode, ScryptoDecode)]
pub struct EpochManagerSubstate {
    pub address: ComponentAddress, // TODO: Does it make sense for this to be stored here?
    pub epoch: u64,
    pub round: u64,

    // TODO: Move configuration to an immutable substate
    pub rounds_per_epoch: u64,
    pub num_unstake_epochs: u64,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Ord, PartialOrd, ScryptoCategorize, ScryptoEncode, ScryptoDecode,
)]
pub struct Validator {
    pub key: EcdsaSecp256k1PublicKey,
    pub stake: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, ScryptoCategorize, ScryptoEncode, ScryptoDecode)]
pub struct ValidatorSetSubstate {
    pub validator_set: BTreeMap<ComponentAddress, Validator>,
    pub epoch: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Categorize, Encode, Decode)]
pub enum EpochManagerError {
    InvalidRoundUpdate { from: u64, to: u64 },
}

pub struct EpochManagerNativePackage;

impl EpochManagerNativePackage {
    pub fn create_auth() -> Vec<MethodAuthorization> {
        vec![MethodAuthorization::Protected(HardAuthRule::ProofRule(
            HardProofRule::Require(HardResourceOrNonFungible::NonFungible(
                AuthAddresses::system_role(),
            )),
        ))]
    }

    pub fn invoke_export<Y>(
        export_name: &str,
        input: ScryptoValue,
        api: &mut Y,
    ) -> Result<IndexedScryptoValue, RuntimeError>
    where
        Y: KernelNodeApi
            + KernelSubstateApi
            + ClientSubstateApi<RuntimeError>
            + ClientApi<RuntimeError>
            + ClientNativeInvokeApi<RuntimeError>,
    {
        match export_name {
            EPOCH_MANAGER_CREATE_IDENT => Self::create(input, api),
            _ => Err(RuntimeError::InterpreterError(
                InterpreterError::InvalidInvocation,
            )),
        }
    }

    fn create<Y>(input: ScryptoValue, api: &mut Y) -> Result<IndexedScryptoValue, RuntimeError>
    where
        Y: KernelNodeApi
            + KernelSubstateApi
            + ClientSubstateApi<RuntimeError>
            + ClientApi<RuntimeError>
            + ClientNativeInvokeApi<RuntimeError>,
    {
        // TODO: Remove decode/encode mess
        let input: EpochManagerCreateInput = scrypto_decode(&scrypto_encode(&input).unwrap())
            .map_err(|_| RuntimeError::InterpreterError(InterpreterError::InvalidInvocation))?;

        let underlying_node_id = api.kernel_allocate_node_id(RENodeType::EpochManager)?;
        let global_node_id = RENodeId::Global(GlobalAddress::Component(
            ComponentAddress::EpochManager(input.component_address),
        ));

        let epoch_manager = EpochManagerSubstate {
            address: global_node_id.into(),
            epoch: input.initial_epoch,
            round: 0,
            rounds_per_epoch: input.rounds_per_epoch,
            num_unstake_epochs: input.num_unstake_epochs,
        };

        let mut olympia_validator_token_resman: ResourceManager = {
            let metadata: BTreeMap<String, String> = BTreeMap::new();
            let mut access_rules = BTreeMap::new();

            // TODO: remove mint and premint all tokens
            {
                let non_fungible_local_id = NonFungibleLocalId::bytes(
                    scrypto_encode(&PackageIdentifier::Scrypto(EPOCH_MANAGER_PACKAGE)).unwrap(),
                )
                .unwrap();
                let global_id = NonFungibleGlobalId::new(PACKAGE_TOKEN, non_fungible_local_id);
                access_rules.insert(Mint, (rule!(require(global_id)), rule!(deny_all)));
            }

            access_rules.insert(Withdraw, (rule!(allow_all), rule!(deny_all)));

            let result = api.call_function(
                RESOURCE_MANAGER_PACKAGE,
                RESOURCE_MANAGER_BLUEPRINT,
                RESOURCE_MANAGER_CREATE_NON_FUNGIBLE_WITH_ADDRESS_IDENT,
                scrypto_encode(&ResourceManagerCreateNonFungibleWithAddressInput {
                    id_type: NonFungibleIdType::Bytes,
                    metadata,
                    access_rules,
                    resource_address: input.olympia_validator_token_address,
                })
                .unwrap(),
            )?;
            let resource_address: ResourceAddress = scrypto_decode(result.as_slice()).unwrap();
            ResourceManager(resource_address)
        };

        let mut validator_set = BTreeMap::new();

        for (key, validator_init) in input.validator_set {
            let local_id = NonFungibleLocalId::bytes(key.to_vec()).unwrap();
            let global_id =
                NonFungibleGlobalId::new(olympia_validator_token_resman.0, local_id.clone());
            let owner_token_bucket =
                olympia_validator_token_resman.mint_non_fungible(local_id, api)?;
            api.call_native(AccountDepositInvocation {
                receiver: validator_init.validator_account_address,
                bucket: owner_token_bucket.0,
            })?;

            let stake = validator_init.initial_stake.sys_amount(api)?;
            let (address, lp_bucket) = ValidatorCreator::create_with_initial_stake(
                global_node_id.into(),
                key,
                rule!(require(global_id)),
                validator_init.initial_stake,
                true,
                api,
            )?;
            let validator = Validator { key, stake };
            validator_set.insert(address, validator);
            api.call_native(AccountDepositInvocation {
                receiver: validator_init.stake_account_address,
                bucket: lp_bucket.0,
            })?;
        }

        let current_validator_set = ValidatorSetSubstate {
            epoch: input.initial_epoch,
            validator_set: validator_set.clone(),
        };

        let preparing_validator_set = ValidatorSetSubstate {
            epoch: input.initial_epoch + 1,
            validator_set,
        };

        let mut access_rules = AccessRules::new();
        access_rules.set_method_access_rule(
            AccessRuleKey::Native(NativeFn::EpochManager(EpochManagerFn::NextRound)),
            rule!(require(AuthAddresses::validator_role())),
        );
        access_rules.set_method_access_rule(
            AccessRuleKey::Native(NativeFn::EpochManager(EpochManagerFn::GetCurrentEpoch)),
            rule!(allow_all),
        );
        access_rules.set_method_access_rule(
            AccessRuleKey::Native(NativeFn::EpochManager(EpochManagerFn::CreateValidator)),
            rule!(allow_all),
        );
        let non_fungible_local_id = NonFungibleLocalId::bytes(
            scrypto_encode(&PackageIdentifier::Native(NativePackage::EpochManager)).unwrap(),
        )
        .unwrap();
        let non_fungible_global_id = NonFungibleGlobalId::new(PACKAGE_TOKEN, non_fungible_local_id);
        access_rules.set_method_access_rule(
            AccessRuleKey::Native(NativeFn::EpochManager(EpochManagerFn::UpdateValidator)),
            rule!(require(non_fungible_global_id)),
        );
        access_rules.set_method_access_rule(
            AccessRuleKey::Native(NativeFn::EpochManager(EpochManagerFn::SetEpoch)),
            rule!(require(AuthAddresses::system_role())), // Set epoch only used for debugging
        );

        let mut node_modules = BTreeMap::new();
        node_modules.insert(
            NodeModuleId::AccessRules,
            RENodeModuleInit::AccessRulesChain(AccessRulesChainSubstate {
                access_rules_chain: vec![access_rules],
            }),
        );

        api.kernel_create_node(
            underlying_node_id,
            RENodeInit::EpochManager(
                epoch_manager,
                current_validator_set,
                preparing_validator_set,
            ),
            node_modules,
        )?;

        api.kernel_create_node(
            global_node_id,
            RENodeInit::Global(GlobalAddressSubstate::EpochManager(
                underlying_node_id.into(),
            )),
            BTreeMap::new(),
        )?;

        let component_address: ComponentAddress = global_node_id.into();
        Ok(IndexedScryptoValue::from_typed(&component_address))
    }
}

pub struct EpochManagerGetCurrentEpochExecutable(RENodeId);

impl ExecutableInvocation for EpochManagerGetCurrentEpochInvocation {
    type Exec = EpochManagerGetCurrentEpochExecutable;

    fn resolve<D: ClientDerefApi<RuntimeError>>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;

        let actor = ResolvedActor::method(
            NativeFn::EpochManager(EpochManagerFn::GetCurrentEpoch),
            resolved_receiver,
        );
        let executor = EpochManagerGetCurrentEpochExecutable(resolved_receiver.receiver);

        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for EpochManagerGetCurrentEpochExecutable {
    type Output = u64;

    fn execute<Y, W: WasmEngine>(self, api: &mut Y) -> Result<(u64, CallFrameUpdate), RuntimeError>
    where
        Y: KernelSubstateApi,
    {
        let offset = SubstateOffset::EpochManager(EpochManagerOffset::EpochManager);
        let handle =
            api.kernel_lock_substate(self.0, NodeModuleId::SELF, offset, LockFlags::read_only())?;
        let substate_ref = api.kernel_get_substate_ref(handle)?;
        let epoch_manager = substate_ref.epoch_manager();
        Ok((epoch_manager.epoch, CallFrameUpdate::empty()))
    }
}

pub struct EpochManagerNextRoundExecutable {
    node_id: RENodeId,
    round: u64,
}

impl ExecutableInvocation for EpochManagerNextRoundInvocation {
    type Exec = EpochManagerNextRoundExecutable;

    fn resolve<D: ClientDerefApi<RuntimeError>>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;

        let actor = ResolvedActor::method(
            NativeFn::EpochManager(EpochManagerFn::NextRound),
            resolved_receiver,
        );
        let executor = EpochManagerNextRoundExecutable {
            node_id: resolved_receiver.receiver,
            round: self.round,
        };

        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for EpochManagerNextRoundExecutable {
    type Output = ();

    fn execute<Y, W: WasmEngine>(self, api: &mut Y) -> Result<((), CallFrameUpdate), RuntimeError>
    where
        Y: KernelSubstateApi,
    {
        let offset = SubstateOffset::EpochManager(EpochManagerOffset::EpochManager);
        let mgr_handle =
            api.kernel_lock_substate(self.node_id, NodeModuleId::SELF, offset, LockFlags::MUTABLE)?;
        let mut substate_mut = api.kernel_get_substate_ref_mut(mgr_handle)?;
        let epoch_manager = substate_mut.epoch_manager();

        if self.round <= epoch_manager.round {
            return Err(RuntimeError::ApplicationError(
                ApplicationError::EpochManagerError(EpochManagerError::InvalidRoundUpdate {
                    from: epoch_manager.round,
                    to: self.round,
                }),
            ));
        }

        if self.round >= epoch_manager.rounds_per_epoch {
            let offset = SubstateOffset::EpochManager(EpochManagerOffset::PreparingValidatorSet);
            let handle = api.kernel_lock_substate(
                self.node_id,
                NodeModuleId::SELF,
                offset,
                LockFlags::MUTABLE,
            )?;
            let mut substate_mut = api.kernel_get_substate_ref_mut(handle)?;
            let preparing_validator_set = substate_mut.validator_set();
            let prepared_epoch = preparing_validator_set.epoch;
            let next_validator_set = preparing_validator_set.validator_set.clone();
            preparing_validator_set.epoch = prepared_epoch + 1;

            let mut substate_mut = api.kernel_get_substate_ref_mut(mgr_handle)?;
            let epoch_manager = substate_mut.epoch_manager();
            epoch_manager.epoch = prepared_epoch;
            epoch_manager.round = 0;

            let offset = SubstateOffset::EpochManager(EpochManagerOffset::CurrentValidatorSet);
            let handle = api.kernel_lock_substate(
                self.node_id,
                NodeModuleId::SELF,
                offset,
                LockFlags::MUTABLE,
            )?;
            let mut substate_mut = api.kernel_get_substate_ref_mut(handle)?;
            let validator_set = substate_mut.validator_set();
            validator_set.epoch = prepared_epoch;
            validator_set.validator_set = next_validator_set;
        } else {
            epoch_manager.round = self.round;
        }

        Ok(((), CallFrameUpdate::empty()))
    }
}

pub struct EpochManagerSetEpochExecutable(RENodeId, u64);

impl ExecutableInvocation for EpochManagerSetEpochInvocation {
    type Exec = EpochManagerSetEpochExecutable;

    fn resolve<D: ClientDerefApi<RuntimeError>>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;

        let actor = ResolvedActor::method(
            NativeFn::EpochManager(EpochManagerFn::SetEpoch),
            resolved_receiver,
        );
        let executor = EpochManagerSetEpochExecutable(resolved_receiver.receiver, self.epoch);

        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for EpochManagerSetEpochExecutable {
    type Output = ();

    fn execute<Y, W: WasmEngine>(self, api: &mut Y) -> Result<((), CallFrameUpdate), RuntimeError>
    where
        Y: KernelSubstateApi,
    {
        let offset = SubstateOffset::EpochManager(EpochManagerOffset::EpochManager);
        let handle =
            api.kernel_lock_substate(self.0, NodeModuleId::SELF, offset, LockFlags::MUTABLE)?;
        let mut substate_mut = api.kernel_get_substate_ref_mut(handle)?;
        substate_mut.epoch_manager().epoch = self.1;
        Ok(((), CallFrameUpdate::empty()))
    }
}

pub struct EpochManagerCreateValidatorExecutable(RENodeId, EcdsaSecp256k1PublicKey, AccessRule);

impl ExecutableInvocation for EpochManagerCreateValidatorInvocation {
    type Exec = EpochManagerCreateValidatorExecutable;

    fn resolve<D: ClientDerefApi<RuntimeError>>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;
        call_frame_update.add_ref(RENodeId::Global(GlobalAddress::Resource(RADIX_TOKEN)));
        call_frame_update.add_ref(RENodeId::Global(GlobalAddress::Resource(PACKAGE_TOKEN)));

        let actor = ResolvedActor::method(
            NativeFn::EpochManager(EpochManagerFn::CreateValidator),
            resolved_receiver,
        );
        let executor = EpochManagerCreateValidatorExecutable(
            resolved_receiver.receiver,
            self.key,
            self.owner_access_rule,
        );

        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for EpochManagerCreateValidatorExecutable {
    type Output = ComponentAddress;

    fn execute<Y, W: WasmEngine>(
        self,
        api: &mut Y,
    ) -> Result<(ComponentAddress, CallFrameUpdate), RuntimeError>
    where
        Y: KernelNodeApi
            + KernelSubstateApi
            + ClientApi<RuntimeError>
            + ClientNativeInvokeApi<RuntimeError>,
    {
        let handle = api.kernel_lock_substate(
            self.0,
            NodeModuleId::SELF,
            SubstateOffset::EpochManager(EpochManagerOffset::EpochManager),
            LockFlags::read_only(),
        )?;
        let substate_ref = api.kernel_get_substate_ref(handle)?;
        let epoch_manager = substate_ref.epoch_manager();
        let manager = epoch_manager.address;
        let validator_address = ValidatorCreator::create(manager, self.1, self.2, false, api)?;
        Ok((
            validator_address,
            CallFrameUpdate::copy_ref(RENodeId::Global(GlobalAddress::Component(
                validator_address,
            ))),
        ))
    }
}

pub struct EpochManagerUpdateValidatorExecutable(RENodeId, ComponentAddress, UpdateValidator);

impl ExecutableInvocation for EpochManagerUpdateValidatorInvocation {
    type Exec = EpochManagerUpdateValidatorExecutable;

    fn resolve<D: ClientDerefApi<RuntimeError>>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;

        let actor = ResolvedActor::method(
            NativeFn::EpochManager(EpochManagerFn::UpdateValidator),
            resolved_receiver,
        );
        let executor = EpochManagerUpdateValidatorExecutable(
            resolved_receiver.receiver,
            self.validator_address,
            self.update,
        );

        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for EpochManagerUpdateValidatorExecutable {
    type Output = ();

    fn execute<Y, W: WasmEngine>(self, api: &mut Y) -> Result<((), CallFrameUpdate), RuntimeError>
    where
        Y: KernelSubstateApi + ClientNativeInvokeApi<RuntimeError>,
    {
        let offset = SubstateOffset::EpochManager(EpochManagerOffset::PreparingValidatorSet);
        let handle =
            api.kernel_lock_substate(self.0, NodeModuleId::SELF, offset, LockFlags::MUTABLE)?;
        let mut substate_ref = api.kernel_get_substate_ref_mut(handle)?;
        let validator_set = substate_ref.validator_set();
        match self.2 {
            UpdateValidator::Register(key, stake) => {
                validator_set
                    .validator_set
                    .insert(self.1, Validator { key, stake });
            }
            UpdateValidator::Unregister => {
                validator_set.validator_set.remove(&self.1);
            }
        }

        Ok(((), CallFrameUpdate::empty()))
    }
}
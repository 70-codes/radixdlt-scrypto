use crate::errors::*;
use crate::system::kernel_modules::execution_trace::ProofSnapshot;
use crate::system::node::RENodeInit;
use crate::system::node::RENodeModuleInit;
use crate::system::node_substates::{SubstateRef, SubstateRefMut};
use crate::types::*;
use crate::wasm::WasmEngine;
use bitflags::bitflags;
use radix_engine_interface::api::component::*;
use radix_engine_interface::api::node_modules::auth::*;
use radix_engine_interface::api::node_modules::metadata::*;
use radix_engine_interface::api::package::*;
use radix_engine_interface::api::types::*;
use radix_engine_interface::api::ClientApi;
use radix_engine_interface::api::ClientDerefApi;
use radix_engine_interface::blueprints::access_controller::*;
use radix_engine_interface::blueprints::account::*;
use radix_engine_interface::blueprints::clock::*;
use radix_engine_interface::blueprints::epoch_manager::*;
use radix_engine_interface::blueprints::logger::*;
use radix_engine_interface::blueprints::resource::*;
use radix_engine_interface::blueprints::transaction_runtime::*;

use super::actor::ResolvedActor;
use super::call_frame::CallFrameUpdate;
use super::call_frame::RENodeVisibilityOrigin;
use super::heap::HeapRENode;
use super::interpreters::ScryptoInterpreter;
use super::module_mixer::KernelModuleMixer;

bitflags! {
    #[derive(Encode, Decode, Categorize)]
    pub struct LockFlags: u32 {
        /// Allows the locked substate to be mutated
        const MUTABLE = 0b00000001;
        /// Checks that the substate locked is unmodified from the beginning of
        /// the transaction. This is used mainly for locking fees in vaults which
        /// requires this in order to be able to support rollbacks
        const UNMODIFIED_BASE = 0b00000010;
        /// Forces a write of a substate even on a transaction failure
        /// Currently used for vault fees.
        const FORCE_WRITE = 0b00000100;
    }
}

impl LockFlags {
    pub fn read_only() -> Self {
        LockFlags::empty()
    }
}

pub struct LockInfo {
    pub offset: SubstateOffset,
}

// Following the convention of Linux Kernel API, https://www.kernel.org/doc/htmldocs/kernel-api/,
// all methods are prefixed by the subsystem of kernel.

pub trait KernelActorApi<E> {
    fn kernel_get_fn_identifier(&mut self) -> Result<FnIdentifier, E>;
}

pub trait KernelNodeApi {
    /// Removes an RENode and all of it's children from the Heap
    fn kernel_drop_node(&mut self, node_id: RENodeId) -> Result<HeapRENode, RuntimeError>;

    /// Allocates a new node id useable for create_node
    fn kernel_allocate_node_id(&mut self, node_type: RENodeType) -> Result<RENodeId, RuntimeError>;

    /// Creates a new RENode
    /// TODO: Remove, replace with lock_substate + get_ref_mut use
    fn kernel_create_node(
        &mut self,
        node_id: RENodeId,
        init: RENodeInit,
        node_module_init: BTreeMap<NodeModuleId, RENodeModuleInit>,
    ) -> Result<(), RuntimeError>;
}

pub trait KernelSubstateApi {
    /// Locks a visible substate
    fn kernel_lock_substate(
        &mut self,
        node_id: RENodeId,
        module_id: NodeModuleId,
        offset: SubstateOffset,
        flags: LockFlags,
    ) -> Result<LockHandle, RuntimeError>;

    fn kernel_get_lock_info(&mut self, lock_handle: LockHandle) -> Result<LockInfo, RuntimeError>;

    /// Drops a lock
    fn kernel_drop_lock(&mut self, lock_handle: LockHandle) -> Result<(), RuntimeError>;

    /// Get a non-mutable reference to a locked substate
    fn kernel_get_substate_ref(
        &mut self,
        lock_handle: LockHandle,
    ) -> Result<SubstateRef, RuntimeError>;

    /// Get a mutable reference to a locked substate
    fn kernel_get_substate_ref_mut(
        &mut self,
        lock_handle: LockHandle,
    ) -> Result<SubstateRefMut, RuntimeError>;
}

pub trait KernelWasmApi<W: WasmEngine> {
    fn kernel_get_scrypto_interpreter(&mut self) -> &ScryptoInterpreter<W>;
}

pub trait Invokable<I: Invocation, E> {
    fn kernel_invoke(&mut self, invocation: I) -> Result<I::Output, E>;
}

pub trait Executor {
    type Output: Debug;

    fn execute<Y, W>(self, api: &mut Y) -> Result<(Self::Output, CallFrameUpdate), RuntimeError>
    where
        Y: KernelNodeApi + KernelSubstateApi + KernelWasmApi<W> + ClientApi<RuntimeError>,
        W: WasmEngine;
}

pub trait ExecutableInvocation: Invocation {
    type Exec: Executor<Output = Self::Output>;

    fn resolve<Y: KernelSubstateApi + ClientDerefApi<RuntimeError>>(
        self,
        api: &mut Y,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>;
}

pub trait KernelInvokeApi<E>:
    Invokable<EpochManagerNextRoundInvocation, E>
    + Invokable<EpochManagerGetCurrentEpochInvocation, E>
    + Invokable<EpochManagerSetEpochInvocation, E>
    + Invokable<EpochManagerUpdateValidatorInvocation, E>
    + Invokable<ValidatorRegisterInvocation, E>
    + Invokable<ValidatorUnregisterInvocation, E>
    + Invokable<ValidatorStakeInvocation, E>
    + Invokable<ValidatorUnstakeInvocation, E>
    + Invokable<ValidatorClaimXrdInvocation, E>
    + Invokable<ValidatorUpdateKeyInvocation, E>
    + Invokable<ValidatorUpdateAcceptDelegatedStakeInvocation, E>
    + Invokable<EpochManagerCreateValidatorInvocation, E>
    + Invokable<ClockSetCurrentTimeInvocation, E>
    + Invokable<ClockGetCurrentTimeInvocation, E>
    + Invokable<ClockCompareCurrentTimeInvocation, E>
    + Invokable<MetadataSetInvocation, E>
    + Invokable<MetadataGetInvocation, E>
    + Invokable<AccessRulesAddAccessCheckInvocation, E>
    + Invokable<AccessRulesSetMethodAccessRuleInvocation, E>
    + Invokable<AccessRulesSetMethodMutabilityInvocation, E>
    + Invokable<AccessRulesSetGroupAccessRuleInvocation, E>
    + Invokable<AccessRulesSetGroupMutabilityInvocation, E>
    + Invokable<AccessRulesGetLengthInvocation, E>
    + Invokable<AuthZonePopInvocation, E>
    + Invokable<AuthZonePushInvocation, E>
    + Invokable<AuthZoneCreateProofInvocation, E>
    + Invokable<AuthZoneCreateProofByAmountInvocation, E>
    + Invokable<AuthZoneCreateProofByIdsInvocation, E>
    + Invokable<AuthZoneClearInvocation, E>
    + Invokable<AuthZoneDrainInvocation, E>
    + Invokable<AuthZoneAssertAccessRuleInvocation, E>
    + Invokable<AccessRulesAddAccessCheckInvocation, E>
    + Invokable<ComponentGlobalizeInvocation, E>
    + Invokable<ComponentGlobalizeWithOwnerInvocation, E>
    + Invokable<ComponentSetRoyaltyConfigInvocation, E>
    + Invokable<ComponentClaimRoyaltyInvocation, E>
    + Invokable<PackageSetRoyaltyConfigInvocation, E>
    + Invokable<PackageClaimRoyaltyInvocation, E>
    + Invokable<PackagePublishInvocation, E>
    + Invokable<PackagePublishNativeInvocation, E>
    + Invokable<BucketTakeInvocation, E>
    + Invokable<BucketPutInvocation, E>
    + Invokable<BucketTakeNonFungiblesInvocation, E>
    + Invokable<BucketGetNonFungibleLocalIdsInvocation, E>
    + Invokable<BucketGetAmountInvocation, E>
    + Invokable<BucketGetResourceAddressInvocation, E>
    + Invokable<BucketCreateProofInvocation, E>
    + Invokable<BucketCreateProofInvocation, E>
    + Invokable<ProofCloneInvocation, E>
    + Invokable<ProofGetAmountInvocation, E>
    + Invokable<ProofGetNonFungibleLocalIdsInvocation, E>
    + Invokable<ProofGetResourceAddressInvocation, E>
    + Invokable<ResourceManagerBurnBucketInvocation, E>
    + Invokable<ResourceManagerBurnInvocation, E>
    + Invokable<ResourceManagerUpdateVaultAuthInvocation, E>
    + Invokable<ResourceManagerSetVaultAuthMutabilityInvocation, E>
    + Invokable<ResourceManagerCreateVaultInvocation, E>
    + Invokable<ResourceManagerCreateBucketInvocation, E>
    + Invokable<ResourceManagerMintNonFungibleInvocation, E>
    + Invokable<ResourceManagerMintUuidNonFungibleInvocation, E>
    + Invokable<ResourceManagerMintFungibleInvocation, E>
    + Invokable<ResourceManagerGetResourceTypeInvocation, E>
    + Invokable<ResourceManagerGetTotalSupplyInvocation, E>
    + Invokable<ResourceManagerUpdateNonFungibleDataInvocation, E>
    + Invokable<ResourceManagerNonFungibleExistsInvocation, E>
    + Invokable<ResourceManagerGetNonFungibleInvocation, E>
    + Invokable<VaultTakeInvocation, E>
    + Invokable<VaultPutInvocation, E>
    + Invokable<VaultLockFeeInvocation, E>
    + Invokable<VaultTakeNonFungiblesInvocation, E>
    + Invokable<VaultGetAmountInvocation, E>
    + Invokable<VaultGetResourceAddressInvocation, E>
    + Invokable<VaultGetNonFungibleLocalIdsInvocation, E>
    + Invokable<VaultCreateProofInvocation, E>
    + Invokable<VaultCreateProofByAmountInvocation, E>
    + Invokable<VaultCreateProofByIdsInvocation, E>
    + Invokable<VaultRecallInvocation, E>
    + Invokable<VaultRecallNonFungiblesInvocation, E>
    + Invokable<WorktopPutInvocation, E>
    + Invokable<WorktopTakeAmountInvocation, E>
    + Invokable<WorktopTakeAllInvocation, E>
    + Invokable<WorktopTakeNonFungiblesInvocation, E>
    + Invokable<WorktopAssertContainsInvocation, E>
    + Invokable<WorktopAssertContainsAmountInvocation, E>
    + Invokable<WorktopAssertContainsNonFungiblesInvocation, E>
    + Invokable<WorktopDrainInvocation, E>
    + Invokable<TransactionRuntimeGetHashInvocation, E>
    + Invokable<TransactionRuntimeGenerateUuidInvocation, E>
    + Invokable<LoggerLogInvocation, E>
    + Invokable<AccessControllerCreateProofInvocation, E>
    + Invokable<AccessControllerInitiateRecoveryAsPrimaryInvocation, E>
    + Invokable<AccessControllerInitiateRecoveryAsRecoveryInvocation, E>
    + Invokable<AccessControllerQuickConfirmPrimaryRoleRecoveryProposalInvocation, E>
    + Invokable<AccessControllerQuickConfirmRecoveryRoleRecoveryProposalInvocation, E>
    + Invokable<AccessControllerTimedConfirmRecoveryInvocation, E>
    + Invokable<AccessControllerCancelPrimaryRoleRecoveryProposalInvocation, E>
    + Invokable<AccessControllerCancelRecoveryRoleRecoveryProposalInvocation, E>
    + Invokable<AccessControllerLockPrimaryRoleInvocation, E>
    + Invokable<AccessControllerUnlockPrimaryRoleInvocation, E>
    + Invokable<AccessControllerStopTimedRecoveryInvocation, E>
    + Invokable<AccountLockFeeInvocation, E>
    + Invokable<AccountLockContingentFeeInvocation, E>
    + Invokable<AccountDepositInvocation, E>
    + Invokable<AccountDepositBatchInvocation, E>
    + Invokable<AccountWithdrawInvocation, E>
    + Invokable<AccountWithdrawAllInvocation, E>
    + Invokable<AccountWithdrawNonFungiblesInvocation, E>
    + Invokable<AccountLockFeeAndWithdrawInvocation, E>
    + Invokable<AccountLockFeeAndWithdrawAllInvocation, E>
    + Invokable<AccountLockFeeAndWithdrawNonFungiblesInvocation, E>
    + Invokable<AccountCreateProofInvocation, E>
    + Invokable<AccountCreateProofByAmountInvocation, E>
    + Invokable<AccountCreateProofByIdsInvocation, E>
{
}

/// Interface of the Kernel, for Kernel modules.
pub trait KernelApi<W: WasmEngine, E>:
    KernelActorApi<E> + KernelNodeApi + KernelSubstateApi + KernelWasmApi<W> + KernelInvokeApi<E>
{
}

/// Internal API for kernel modules.
/// No kernel state changes are expected as of a result of invoking such APIs, except updating returned references.
pub trait KernelInternalApi {
    fn kernel_get_module_state(&mut self) -> &mut KernelModuleMixer;
    fn kernel_get_node_visibility_origin(
        &self,
        node_id: RENodeId,
    ) -> Option<RENodeVisibilityOrigin>;
    fn kernel_get_current_depth(&self) -> usize;
    fn kernel_get_current_actor(&self) -> ResolvedActor;

    /* Super unstable interface, specifically for `ExecutionTrace` kernel module */
    fn kernel_read_bucket(&mut self, bucket_id: BucketId) -> Option<Resource>;
    fn kernel_read_proof(&mut self, proof_id: BucketId) -> Option<ProofSnapshot>;
}

pub trait KernelModuleApi<E>: KernelNodeApi + KernelSubstateApi + KernelInternalApi {}
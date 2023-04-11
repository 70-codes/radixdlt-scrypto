use super::call_frame::RefType;
use super::heap::HeapNode;
use crate::errors::*;
use crate::kernel::actor::Actor;
use crate::kernel::call_frame::CallFrameUpdate;
use crate::kernel::kernel_callback_api::KernelCallbackObject;
use crate::system::system_modules::execution_trace::BucketSnapshot;
use crate::system::system_modules::execution_trace::ProofSnapshot;
use crate::types::*;
use radix_engine_interface::api::substate_api::LockFlags;

// Following the convention of Linux Kernel API, https://www.kernel.org/doc/htmldocs/kernel-api/,
// all methods are prefixed by the subsystem of kernel.

/// API for managing nodes
pub trait KernelNodeApi {
    /// Removes an RENode and all of it's children from the Heap
    fn kernel_drop_node(&mut self, node_id: &NodeId) -> Result<HeapNode, RuntimeError>;

    /// TODO: Remove
    fn kernel_allocate_virtual_node_id(&mut self, node_id: NodeId) -> Result<(), RuntimeError>;

    /// Allocates a new node id useable for create_node
    fn kernel_allocate_node_id(&mut self, node_type: EntityType) -> Result<NodeId, RuntimeError>;

    /// Creates a new RENode
    fn kernel_create_node(
        &mut self,
        node_id: NodeId,
        module_init: BTreeMap<SysModuleId, BTreeMap<SubstateKey, IndexedScryptoValue>>,
    ) -> Result<(), RuntimeError>;
}

/// Info regarding the substate locked as well as what type of lock
pub struct LockInfo {
    pub node_id: NodeId,
    pub module_id: SysModuleId,
    pub substate_key: SubstateKey,
    pub flags: LockFlags,
}

/// API for managing substates within nodes
pub trait KernelSubstateApi {
    /// Locks a substate to make available for reading and/or writing
    fn kernel_lock_substate(
        &mut self,
        node_id: &NodeId,
        module_id: SysModuleId,
        substate_key: &SubstateKey,
        flags: LockFlags,
    ) -> Result<LockHandle, RuntimeError>;

    /// Retrieves info related to a lock
    fn kernel_get_lock_info(&mut self, lock_handle: LockHandle) -> Result<LockInfo, RuntimeError>;

    /// Drops a lock on some substate, if the lock is writable, updates are flushed to
    /// the store at this point.
    fn kernel_drop_lock(&mut self, lock_handle: LockHandle) -> Result<(), RuntimeError>;

    /// Reads the value of the substate locked by the given lock handle
    fn kernel_read_substate(
        &mut self,
        lock_handle: LockHandle,
    ) -> Result<&IndexedScryptoValue, RuntimeError>;

    /// Writes a value to the substate locked by the given lock handle
    fn kernel_write_substate(
        &mut self,
        lock_handle: LockHandle,
        value: IndexedScryptoValue,
    ) -> Result<(), RuntimeError>;
}

#[derive(Debug)]
pub struct KernelInvocation<I: Debug> {
    pub sys_invocation: I,

    // TODO: Remove
    pub payload_size: usize,

    // TODO: Make these two RENodes / Substates
    pub resolved_actor: Actor,
    pub args: IndexedScryptoValue,
}

impl<I: Debug> KernelInvocation<I> {
    pub fn get_update(&self) -> CallFrameUpdate {
        let nodes_to_move = self.args.owned_node_ids().clone();
        let mut node_refs_to_copy = self.args.references().clone();
        match self.resolved_actor {
            Actor::Method { node_id, .. } => {
                node_refs_to_copy.insert(node_id);
            }
            Actor::Function { .. } | Actor::VirtualLazyLoad { .. } => {}
        }

        CallFrameUpdate {
            nodes_to_move,
            node_refs_to_copy,
        }
    }
}

/// API for invoking a function creating a new call frame and passing
/// control to the callee
pub trait KernelInvokeDownstreamApi<I: Debug> {
    fn kernel_invoke_downstream(
        &mut self,
        invocation: Box<KernelInvocation<I>>,
    ) -> Result<IndexedScryptoValue, RuntimeError>;
}

/// Internal API for kernel modules.
/// No kernel state changes are expected as of a result of invoking such APIs, except updating returned references.
pub trait KernelInternalApi<M: KernelCallbackObject> {
    /// Retrieves data associated with the kernel upstream layer (system)
    fn kernel_get_callback(&mut self) -> &mut M;

    /// Gets the number of call frames that are currently in the call frame stack
    fn kernel_get_current_depth(&self) -> usize;

    // TODO: Cleanup
    fn kernel_get_node_info(&self, node_id: &NodeId) -> Option<(RefType, bool)>;

    // TODO: Remove these, these are temporary until the architecture
    // TODO: gets cleaned up a little more
    fn kernel_get_current_actor(&mut self) -> Option<Actor>;
    fn kernel_load_package_package_dependencies(&mut self);
    fn kernel_load_common(&mut self);

    /* Super unstable interface, specifically for `ExecutionTrace` kernel module */
    fn kernel_read_bucket(&mut self, bucket_id: &NodeId) -> Option<BucketSnapshot>;
    fn kernel_read_proof(&mut self, proof_id: &NodeId) -> Option<ProofSnapshot>;
}

pub trait KernelApi<M: KernelCallbackObject>:
    KernelNodeApi + KernelSubstateApi + KernelInvokeDownstreamApi<M::Invocation> + KernelInternalApi<M>
{
}

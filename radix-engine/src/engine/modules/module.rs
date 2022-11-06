use crate::engine::call_frame::RENodeLocation;
use crate::engine::*;
use crate::fee::FeeReserve;
use crate::model::Resource;
use crate::types::*;

pub enum SysCallInput<'a> {
    Invoke {
        name: String,
        input_size: u32,
        value_count: u32,
        depth: usize,
    },
    ReadOwnedNodes,
    BorrowNode {
        node_id: &'a RENodeId,
    },
    DropNode {
        node_id: &'a RENodeId,
    },
    CreateNode {
        node: &'a RENode,
    },
    LockSubstate {
        node_id: &'a RENodeId,
        offset: &'a SubstateOffset,
        flags: &'a LockFlags,
    },
    GetRef {
        lock_handle: &'a LockHandle,
    },
    GetRefMut {
        lock_handle: &'a LockHandle,
    },
    DropLock {
        lock_handle: &'a LockHandle,
    },
    TakeSubstate {
        substate_id: &'a SubstateId,
    },
    ReadTransactionHash,
    ReadBlob {
        blob_hash: &'a Hash,
    },
    GenerateUuid,
    EmitLog {
        level: &'a Level,
        message: &'a String,
    },
}

pub enum SysCallOutput<'a> {
    Invoke { rtn: String },
    BorrowNode { node_pointer: &'a RENodeLocation },
    DropNode { node: &'a HeapRENode },
    CreateNode { node_id: &'a RENodeId },
    LockSubstate { lock_handle: LockHandle },
    GetRef { lock_handle: LockHandle },
    GetRefMut,
    DropLock,
    ReadTransactionHash { hash: &'a Hash },
    ReadBlob { blob: &'a [u8] },
    GenerateUuid { uuid: u128 },
    EmitLog,
}

pub trait Module<R: FeeReserve> {
    fn pre_sys_call(
        &mut self,
        call_frame: &CallFrame,
        heap: &mut Heap,
        track: &mut Track<R>,
        input: SysCallInput,
    ) -> Result<(), ModuleError>;

    fn post_sys_call(
        &mut self,
        call_frame: &CallFrame,
        heap: &mut Heap,
        track: &mut Track<R>,
        output: SysCallOutput,
    ) -> Result<(), ModuleError>;

    fn on_run(
        &mut self,
        actor: &REActor,
        input: &IndexedScryptoValue,
        call_frame: &CallFrame,
        heap: &mut Heap,
        track: &mut Track<R>,
    ) -> Result<(), ModuleError>;

    fn on_wasm_instantiation(
        &mut self,
        call_frame: &CallFrame,
        heap: &mut Heap,
        track: &mut Track<R>,
        code: &[u8],
    ) -> Result<(), ModuleError>;

    fn on_wasm_costing(
        &mut self,
        call_frame: &CallFrame,
        heap: &mut Heap,
        track: &mut Track<R>,
        units: u32,
    ) -> Result<(), ModuleError>;

    fn on_lock_fee(
        &mut self,
        call_frame: &CallFrame,
        heap: &mut Heap,
        track: &mut Track<R>,
        vault_id: VaultId,
        fee: Resource,
        contingent: bool,
    ) -> Result<Resource, ModuleError>;
}

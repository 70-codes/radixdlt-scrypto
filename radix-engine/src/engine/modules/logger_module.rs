use crate::engine::*;
use crate::fee::FeeReserve;
use crate::model::Resource;
use crate::types::*;

pub struct LoggerModule {
    depth: usize,
}

impl LoggerModule {
    pub fn new() -> Self {
        Self { depth: 0 }
    }
}

#[macro_export]
macro_rules! log {
    ( $self: expr, $msg: expr $( , $arg:expr )* ) => {
        #[cfg(not(feature = "alloc"))]
        println!("{}[{}] {}", "    ".repeat($self.depth), $self.depth, sbor::rust::format!($msg, $( $arg ),*));
    };
}

#[allow(unused_variables)] // for no_std
impl<R: FeeReserve> Module<R> for LoggerModule {
    fn pre_sys_call(
        &mut self,
        _call_frame: &CallFrame,
        _heap: &mut Heap,
        _track: &mut Track<R>,
        input: SysCallInput,
    ) -> Result<(), ModuleError> {
        match input {
            SysCallInput::Invoke { info, .. } => {
                log!(self, "Invoking: {:?}", info);

                self.depth = self.depth + 1;
            }
            SysCallInput::ReadOwnedNodes => {
                log!(self, "Reading owned nodes");
            }
            SysCallInput::BorrowNode { node_id } => {
                log!(self, "Borrowing node: node_id = {:?}", node_id);
            }
            SysCallInput::DropNode { node_id } => {
                log!(self, "Dropping node: node_id = {:?}", node_id);
            }
            SysCallInput::CreateNode { node } => {
                log!(self, "Creating node: node = {:?}", node);
            }
            SysCallInput::LockSubstate {
                node_id,
                offset,
                flags,
            } => {
                log!(
                    self,
                    "Lock substate: node_id = {:?} offset = {:?} flags = {:?}",
                    node_id,
                    offset,
                    flags
                );
            }
            SysCallInput::GetRef { lock_handle } => {
                log!(self, "Reading substate: lock_handle = {:?}", lock_handle);
            }
            SysCallInput::GetRefMut { lock_handle } => {
                log!(self, "Get Mut: lock_handle = {:?}", lock_handle);
            }
            SysCallInput::DropLock { lock_handle } => {
                log!(self, "Drop Lock: lock_handle = {:?}", lock_handle);
            }
            SysCallInput::TakeSubstate { substate_id } => {
                log!(self, "Taking substate: substate_id = {:?}", substate_id);
            }
            SysCallInput::ReadTransactionHash => {
                log!(self, "Reading transaction hash");
            }
            SysCallInput::ReadBlob { blob_hash } => {
                log!(self, "Reading blob: hash = {}", blob_hash);
            }
            SysCallInput::GenerateUuid => {
                log!(self, "Generating UUID");
            }
            SysCallInput::EmitLog { .. } => {
                log!(self, "Emitting application log");
            }
        }

        Ok(())
    }

    fn post_sys_call(
        &mut self,
        _call_frame: &CallFrame,
        _heap: &mut Heap,
        _track: &mut Track<R>,
        output: SysCallOutput,
    ) -> Result<(), ModuleError> {
        match output {
            SysCallOutput::Invoke { rtn, .. } => {
                self.depth = self.depth - 1;
                log!(self, "Exiting invoke: output = {:?}", rtn);
            }
            SysCallOutput::BorrowNode { .. } => {}
            SysCallOutput::DropNode { .. } => {}
            SysCallOutput::CreateNode { .. } => {}
            SysCallOutput::LockSubstate { .. } => {}
            SysCallOutput::GetRef { .. } => {}
            SysCallOutput::GetRefMut { .. } => {}
            SysCallOutput::DropLock { .. } => {}
            SysCallOutput::ReadTransactionHash { .. } => {}
            SysCallOutput::ReadBlob { .. } => {}
            SysCallOutput::GenerateUuid { .. } => {}
            SysCallOutput::EmitLog { .. } => {}
            SysCallOutput::ReadOwnedNodes { .. } => {}
        }

        Ok(())
    }

    fn on_run(
        &mut self,
        _actor: &REActor,
        _input: &ScryptoValue,
        _call_frame: &CallFrame,
        _heap: &mut Heap,
        _track: &mut Track<R>,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn on_wasm_instantiation(
        &mut self,
        _call_frame: &CallFrame,
        _heap: &mut Heap,
        _track: &mut Track<R>,
        _code: &[u8],
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn on_wasm_costing(
        &mut self,
        _call_frame: &CallFrame,
        _heap: &mut Heap,
        _track: &mut Track<R>,
        _units: u32,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn on_lock_fee(
        &mut self,
        _call_frame: &CallFrame,
        _heap: &mut Heap,
        _track: &mut Track<R>,
        _vault_id: VaultId,
        fee: Resource,
        _contingent: bool,
    ) -> Result<Resource, ModuleError> {
        Ok(fee)
    }

    fn on_application_event(
        &mut self,
        _call_frame: &CallFrame,
        _heap: &mut Heap,
        _track: &mut Track<R>,
        _event: &ApplicationEvent,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn on_finished_processing(
        &mut self,
        _heap: &mut Heap,
        _track: &mut Track<R>,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

use crate::engine::errors::KernelError;
use crate::engine::*;
use crate::fee::*;
use crate::model::{
    ComponentInfoSubstate, ComponentStateSubstate, GlobalAddressSubstate, InvokeError,
    KeyValueStore, RuntimeSubstate,
};
use crate::types::*;
use crate::wasm::*;

/// A glue between system api (call frame and track abstraction) and WASM.
///
/// Execution is free from a costing perspective, as we assume
/// the system api will bill properly.
pub struct RadixEngineWasmRuntime<'y, 's, Y, R>
where
    Y: SystemApi<'s, R>
        + Invokable<ScryptoInvocation, ScryptoValue>
        + Invokable<NativeInvocation, ScryptoValue>,
    R: FeeReserve,
{
    actor: ScryptoActor,
    system_api: &'y mut Y,
    lock_types: HashMap<LockHandle, SubstateOffset>,
    phantom1: PhantomData<R>,
    phantom2: PhantomData<&'s ()>,
}

impl<'y, 's, Y, R> RadixEngineWasmRuntime<'y, 's, Y, R>
where
    Y: SystemApi<'s, R>
        + Invokable<ScryptoInvocation, ScryptoValue>
        + Invokable<NativeInvocation, ScryptoValue>,
    R: FeeReserve,
{
    // TODO: expose API for reading blobs

    // TODO: do we want to allow dynamic creation of blobs?

    // TODO: do we check existence of blobs when being passed as arguments/return?

    pub fn new(actor: ScryptoActor, system_api: &'y mut Y) -> Self {
        RadixEngineWasmRuntime {
            actor,
            system_api,
            lock_types: HashMap::new(),
            phantom1: PhantomData,
            phantom2: PhantomData,
        }
    }

    fn handle_invoke_scrypto_function(
        &mut self,
        fn_ident: ScryptoFunctionIdent,
        args: Vec<u8>,
    ) -> Result<ScryptoValue, RuntimeError> {
        let args = ScryptoValue::from_slice(&args)
            .map_err(|e| RuntimeError::KernelError(KernelError::DecodeError(e)))?;
        self.system_api
            .invoke(ScryptoInvocation::Function(fn_ident, args))
    }

    fn handle_invoke_scrypto_method(
        &mut self,
        fn_ident: ScryptoMethodIdent,
        args: Vec<u8>,
    ) -> Result<ScryptoValue, RuntimeError> {
        let args = ScryptoValue::from_slice(&args)
            .map_err(|e| RuntimeError::KernelError(KernelError::DecodeError(e)))?;
        self.system_api
            .invoke(ScryptoInvocation::Method(fn_ident, args))
    }

    fn handle_invoke_native_function(
        &mut self,
        native_function: NativeFunction,
        args: Vec<u8>,
    ) -> Result<ScryptoValue, RuntimeError> {
        let args = ScryptoValue::from_slice(&args)
            .map_err(|e| RuntimeError::KernelError(KernelError::DecodeError(e)))?;

        self.system_api
            .invoke(NativeInvocation::Function(native_function, args))
    }

    fn handle_invoke_native_method(
        &mut self,
        native_method: NativeMethod,
        receiver: RENodeId,
        args: Vec<u8>,
    ) -> Result<ScryptoValue, RuntimeError> {
        let args = ScryptoValue::from_slice(&args)
            .map_err(|e| RuntimeError::KernelError(KernelError::DecodeError(e)))?;

        self.system_api
            .invoke(NativeInvocation::Method(native_method, receiver, args))
    }

    fn handle_node_create(
        &mut self,
        scrypto_node: ScryptoRENode,
    ) -> Result<ScryptoValue, RuntimeError> {
        let node = match scrypto_node {
            ScryptoRENode::GlobalComponent(component_id) => RENode::Global(
                GlobalAddressSubstate::Component(scrypto::component::Component(component_id)),
            ),
            ScryptoRENode::Component(package_address, blueprint_name, state) => {
                // Create component
                RENode::Component(
                    ComponentInfoSubstate::new(package_address, blueprint_name, Vec::new()),
                    ComponentStateSubstate::new(state),
                )
            }
            ScryptoRENode::KeyValueStore => RENode::KeyValueStore(KeyValueStore::new()),
        };

        let id = self.system_api.create_node(node)?;
        Ok(ScryptoValue::from_typed(&id))
    }

    fn handle_get_visible_node_ids(&mut self) -> Result<ScryptoValue, RuntimeError> {
        let node_ids = self.system_api.get_visible_node_ids()?;
        Ok(ScryptoValue::from_typed(&node_ids))
    }

    fn handle_drop_node(&mut self, node_id: RENodeId) -> Result<ScryptoValue, RuntimeError> {
        self.system_api.drop_node(node_id)?;
        Ok(ScryptoValue::from_typed(&()))
    }

    fn handle_lock_substate(
        &mut self,
        node_id: RENodeId,
        offset: SubstateOffset,
        mutable: bool,
    ) -> Result<ScryptoValue, RuntimeError> {
        let flags = if mutable {
            LockFlags::MUTABLE
        } else {
            // TODO: Do we want to expose full flag functionality to Scrypto?
            LockFlags::read_only()
        };

        let handle = self
            .system_api
            .lock_substate(node_id, offset.clone(), flags)?;

        self.lock_types.insert(handle, offset);

        Ok(ScryptoValue::from_typed(&handle))
    }

    fn handle_read(&mut self, lock_handle: LockHandle) -> Result<ScryptoValue, RuntimeError> {
        self.system_api
            .get_ref(lock_handle)
            .map(|substate_ref| substate_ref.to_scrypto_value())
    }

    fn handle_write(
        &mut self,
        lock_handle: LockHandle,
        buffer: Vec<u8>,
    ) -> Result<ScryptoValue, RuntimeError> {
        let offset = self
            .lock_types
            .get(&lock_handle)
            .ok_or(RuntimeError::KernelError(KernelError::LockDoesNotExist(
                lock_handle,
            )))?;
        let substate = RuntimeSubstate::decode_from_buffer(offset, &buffer)?;
        let mut substate_mut = self.system_api.get_ref_mut(lock_handle)?;

        match substate {
            RuntimeSubstate::ComponentState(next) => *substate_mut.component_state() = next,
            RuntimeSubstate::KeyValueStoreEntry(next) => {
                *substate_mut.kv_store_entry() = next;
            }
            RuntimeSubstate::NonFungible(next) => {
                *substate_mut.non_fungible() = next;
            }
            _ => return Err(RuntimeError::KernelError(KernelError::InvalidOverwrite)),
        }

        Ok(ScryptoValue::unit())
    }

    fn handle_drop_lock(&mut self, lock_handle: LockHandle) -> Result<ScryptoValue, RuntimeError> {
        self.lock_types.remove(&lock_handle);
        self.system_api
            .drop_lock(lock_handle)
            .map(|unit| ScryptoValue::from_typed(&unit))
    }

    fn handle_get_actor(&mut self) -> Result<ScryptoActor, RuntimeError> {
        return Ok(self.actor.clone());
    }

    fn handle_get_transaction_hash(&mut self) -> Result<Hash, RuntimeError> {
        self.system_api.read_transaction_hash()
    }

    fn handle_generate_uuid(&mut self) -> Result<u128, RuntimeError> {
        self.system_api.generate_uuid()
    }

    fn handle_emit_log(&mut self, level: Level, message: String) -> Result<(), RuntimeError> {
        self.system_api.emit_log(level, message)
    }
}

fn encode<T: Encode>(output: T) -> ScryptoValue {
    ScryptoValue::from_typed(&output)
}

impl<'y, 's, Y, R> WasmRuntime for RadixEngineWasmRuntime<'y, 's, Y, R>
where
    Y: SystemApi<'s, R>
        + Invokable<ScryptoInvocation, ScryptoValue>
        + Invokable<NativeInvocation, ScryptoValue>,
    R: FeeReserve,
{
    fn main(&mut self, input: ScryptoValue) -> Result<ScryptoValue, InvokeError<WasmError>> {
        let input: RadixEngineInput = scrypto_decode(&input.raw)
            .map_err(|_| InvokeError::Error(WasmError::InvalidRadixEngineInput))?;
        let rtn = match input {
            RadixEngineInput::InvokeScryptoFunction(function_ident, args) => {
                self.handle_invoke_scrypto_function(function_ident, args)?
            }
            RadixEngineInput::InvokeScryptoMethod(method_ident, args) => {
                self.handle_invoke_scrypto_method(method_ident, args)?
            }
            RadixEngineInput::InvokeNativeFunction(native_function, args) => {
                self.handle_invoke_native_function(native_function, args)?
            }
            RadixEngineInput::InvokeNativeMethod(native_method, receiver, args) => {
                self.handle_invoke_native_method(native_method, receiver, args)?
            }
            RadixEngineInput::CreateNode(node) => self.handle_node_create(node)?,
            RadixEngineInput::GetVisibleNodeIds() => self.handle_get_visible_node_ids()?,
            RadixEngineInput::DropNode(node_id) => self.handle_drop_node(node_id)?,
            RadixEngineInput::LockSubstate(node_id, offset, mutable) => {
                self.handle_lock_substate(node_id, offset, mutable)?
            }
            RadixEngineInput::Read(lock_handle) => self.handle_read(lock_handle)?,
            RadixEngineInput::Write(lock_handle, value) => self.handle_write(lock_handle, value)?,
            RadixEngineInput::DropLock(lock_handle) => self.handle_drop_lock(lock_handle)?,

            RadixEngineInput::GetActor() => self.handle_get_actor().map(encode)?,
            RadixEngineInput::GetTransactionHash() => {
                self.handle_get_transaction_hash().map(encode)?
            }
            RadixEngineInput::GenerateUuid() => self.handle_generate_uuid().map(encode)?,
            RadixEngineInput::EmitLog(level, message) => {
                self.handle_emit_log(level, message).map(encode)?
            }
        };

        Ok(rtn)
    }

    fn consume_cost_units(&mut self, n: u32) -> Result<(), InvokeError<WasmError>> {
        self.system_api
            .consume_cost_units(n)
            .map_err(InvokeError::downstream)
    }
}

/// A `Nop` runtime accepts any external function calls by doing nothing and returning void.
pub struct NopWasmRuntime {
    fee_reserve: SystemLoanFeeReserve,
}

impl NopWasmRuntime {
    pub fn new(fee_reserve: SystemLoanFeeReserve) -> Self {
        Self { fee_reserve }
    }
}

impl WasmRuntime for NopWasmRuntime {
    fn main(&mut self, _input: ScryptoValue) -> Result<ScryptoValue, InvokeError<WasmError>> {
        Ok(ScryptoValue::unit())
    }

    fn consume_cost_units(&mut self, n: u32) -> Result<(), InvokeError<WasmError>> {
        self.fee_reserve
            .consume(n, "run_wasm", false)
            .map_err(|e| InvokeError::Error(WasmError::CostingError(e)))
    }
}

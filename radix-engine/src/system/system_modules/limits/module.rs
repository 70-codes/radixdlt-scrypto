use crate::kernel::actor::Actor;
use crate::kernel::kernel_api::KernelInvocation;
use crate::system::module::SystemModule;
use crate::system::system_callback::SystemConfig;
use crate::system::system_callback_api::SystemCallbackObject;
use crate::track::interface::StoreAccessInfo;
use crate::transaction::ExecutionMetrics;
use crate::types::*;
use crate::{
    errors::RuntimeError,
    errors::SystemModuleError,
    kernel::{call_frame::Message, kernel_api::KernelApi},
    types::Vec,
};

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub enum TransactionLimitsError {
    /// Retruned when WASM memory consumed during transaction execution exceeds defined limit,
    /// as parameter current memory value is returned.
    MaxWasmMemoryExceeded(usize),
    /// Retruned when one instance WASM memory consumed during transaction execution exceeds defined limit,
    /// as parameter memory consumed by that instave is returned.
    MaxWasmInstanceMemoryExceeded(usize),
    /// Returned when substate read count during transaction execution
    /// exceeds defined limit just after read occurs.
    MaxSubstateReadCountExceeded,
    /// Returned when substate write count during transaction execution
    /// exceeds defined limit just after write occurs.
    MaxSubstateWriteCountExceeded,
    /// Returned when substate read size exceeds defined limit just after read occurs.
    MaxSubstateReadSizeExceeded(usize),
    /// Returned when substate write size exceeds defined limit just after write occurs.
    MaxSubstateWriteSizeExceeded(usize),
    /// Returned when function or method invocation payload size exceeds defined limit,
    /// as parameter actual payload size is returned.
    MaxInvokePayloadSizeExceeded(usize),
}

/// Representation of data which needs to be limited for each call frame.
#[derive(Default)]
struct CallFrameLimitInfo {
    /// Consumed WASM memory.
    wasm_memory_usage: usize,
}

pub struct TransactionLimitsConfig {
    /// Maximum WASM memory which can be consumed during transaction execution.
    pub max_wasm_memory: usize,
    /// Maximum WASM memory which can be consumed in one call frame.
    pub max_wasm_memory_per_call_frame: usize,
    /// Maximum Substates reads for a transaction.
    pub max_substate_read_count: usize,
    /// Maximum Substates writes for a transaction.
    pub max_substate_write_count: usize,
    /// Maximum Substate read and write size.
    pub max_substate_size: usize,
    /// Maximum Invoke payload size.
    pub max_invoke_payload_size: usize,
}

/// Tracks and verifies transaction limits during transactino execution,
/// if exceeded breaks execution with appropriate error.
/// Default limits values are defined in radix-engine-constants lib.
/// Stores boundary values of the limits and returns them in transaction receipt.
pub struct LimitsModule {
    /// Definitions of the limits levels.
    limits_config: TransactionLimitsConfig,
    /// Internal stack of data for each call frame.
    call_frames_stack: Vec<CallFrameLimitInfo>,
    /// Substate store read count.
    substate_db_read_count: usize,
    /// Substate store write count.
    substate_db_write_count: usize,
    /// Substate store read size total.
    substate_db_read_size_total: usize,
    /// Substate store write size total.
    substate_db_write_size_total: usize,
    /// Maximum WASM.
    wasm_max_memory: usize,
    /// Maximum Invoke payload size.
    invoke_payload_max_size: usize,
}

impl LimitsModule {
    pub fn new(limits_config: TransactionLimitsConfig) -> Self {
        LimitsModule {
            limits_config,
            call_frames_stack: Vec::with_capacity(8),
            substate_db_read_count: 0,
            substate_db_write_count: 0,
            substate_db_read_size_total: 0,
            substate_db_write_size_total: 0,
            wasm_max_memory: 0,
            invoke_payload_max_size: 0,
        }
    }

    /// Exports metrics to transaction receipt.
    pub fn finalize(self, execution_cost_units_consumed: u32) -> ExecutionMetrics {
        ExecutionMetrics {
            substate_read_count: self.substate_db_read_count,
            substate_write_count: self.substate_db_write_count,
            substate_read_size: self.substate_db_read_size_total,
            substate_write_size: self.substate_db_write_size_total,
            max_wasm_memory_used: self.wasm_max_memory,
            max_invoke_payload_size: self.invoke_payload_max_size,
            execution_cost_units_consumed,
        }
    }

    /// Checks if maximum WASM memory limit for one instance was exceeded and then
    /// checks if memory limit for all instances was exceeded.
    fn validate_wasm_memory(&mut self) -> Result<(), RuntimeError> {
        // check last (current) call frame
        let current_call_frame = self
            .call_frames_stack
            .last()
            .expect("Call frames stack (Wasm memory) should not be empty.");
        if current_call_frame.wasm_memory_usage > self.limits_config.max_wasm_memory_per_call_frame
        {
            return Err(RuntimeError::SystemModuleError(
                SystemModuleError::TransactionLimitsError(
                    TransactionLimitsError::MaxWasmInstanceMemoryExceeded(
                        current_call_frame.wasm_memory_usage,
                    ),
                ),
            ));
        };

        // calculate current maximum consumed memory
        // sum all call stack values
        let max_value = self
            .call_frames_stack
            .iter()
            .map(|item| item.wasm_memory_usage)
            .sum();

        if max_value > self.wasm_max_memory {
            self.wasm_max_memory = max_value;
        }

        // validate if limit was exceeded
        if max_value > self.limits_config.max_wasm_memory {
            Err(RuntimeError::SystemModuleError(
                SystemModuleError::TransactionLimitsError(
                    TransactionLimitsError::MaxWasmMemoryExceeded(max_value),
                ),
            ))
        } else {
            Ok(())
        }
    }

    /// Checks if substate reads/writes count and size is in the limit.
    fn validate_substates(
        &self,
        read_size: Option<usize>,
        write_size: Option<usize>,
    ) -> Result<(), RuntimeError> {
        if let Some(size) = read_size {
            if size > self.limits_config.max_substate_size {
                return Err(RuntimeError::SystemModuleError(
                    SystemModuleError::TransactionLimitsError(
                        TransactionLimitsError::MaxSubstateReadSizeExceeded(size),
                    ),
                ));
            }
        }
        if let Some(size) = write_size {
            if size > self.limits_config.max_substate_size {
                return Err(RuntimeError::SystemModuleError(
                    SystemModuleError::TransactionLimitsError(
                        TransactionLimitsError::MaxSubstateWriteSizeExceeded(size),
                    ),
                ));
            }
        }

        if self.substate_db_read_count > self.limits_config.max_substate_read_count {
            Err(RuntimeError::SystemModuleError(
                SystemModuleError::TransactionLimitsError(
                    TransactionLimitsError::MaxSubstateReadCountExceeded,
                ),
            ))
        } else if self.substate_db_write_count > self.limits_config.max_substate_write_count {
            Err(RuntimeError::SystemModuleError(
                SystemModuleError::TransactionLimitsError(
                    TransactionLimitsError::MaxSubstateWriteCountExceeded,
                ),
            ))
        } else {
            Ok(())
        }
    }

    // This event handler is called from two places:
    //  1. Before wasm nested function call
    //  2. After wasm invocation
    pub fn update_wasm_memory_usage(
        &mut self,
        depth: usize,
        consumed_memory: usize,
    ) -> Result<(), RuntimeError> {
        // update current frame consumed memory
        if let Some(val) = self.call_frames_stack.get_mut(depth) {
            val.wasm_memory_usage = consumed_memory;
        } else {
            // When kernel pops the call frame there are some nested calls which
            // are not aligned with before_push_frame() which requires pushing
            // new value on a stack instead of updating it.
            self.call_frames_stack.push(CallFrameLimitInfo {
                wasm_memory_usage: consumed_memory,
            })
        }

        self.validate_wasm_memory()
    }
}

impl<V: SystemCallbackObject> SystemModule<SystemConfig<V>> for LimitsModule {
    fn before_invoke<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        invocation: &KernelInvocation,
    ) -> Result<(), RuntimeError> {
        let tlimit = &mut api.kernel_get_system().modules.limits;
        let input_size = invocation.len();
        if input_size > tlimit.invoke_payload_max_size {
            tlimit.invoke_payload_max_size = input_size;
        }

        if input_size > tlimit.limits_config.max_invoke_payload_size {
            Err(RuntimeError::SystemModuleError(
                SystemModuleError::TransactionLimitsError(
                    TransactionLimitsError::MaxInvokePayloadSizeExceeded(input_size),
                ),
            ))
        } else {
            Ok(())
        }
    }

    fn before_push_frame<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        _callee: &Actor,
        _down_message: &mut Message,
        _args: &IndexedScryptoValue,
    ) -> Result<(), RuntimeError> {
        // push new empty wasm memory value refencing current call frame to internal stack
        api.kernel_get_system()
            .modules
            .limits
            .call_frames_stack
            .push(CallFrameLimitInfo::default());
        Ok(())
    }

    fn after_pop_frame<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        _dropped_actor: &Actor,
    ) -> Result<(), RuntimeError> {
        // pop from internal stack
        api.kernel_get_system()
            .modules
            .limits
            .call_frames_stack
            .pop();
        Ok(())
    }

    fn on_read_substate<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        _lock_handle: LockHandle,
        value_size: usize,
        _store_access: &StoreAccessInfo,
    ) -> Result<(), RuntimeError> {
        let tlimit = &mut api.kernel_get_system().modules.limits;

        // Increase read coutner.
        tlimit.substate_db_read_count += 1;

        // Increase total size.
        tlimit.substate_db_read_size_total += value_size;

        // Validate
        tlimit.validate_substates(Some(value_size), None)
    }

    fn on_write_substate<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        _lock_handle: LockHandle,
        value_size: usize,
        _store_access: &StoreAccessInfo,
    ) -> Result<(), RuntimeError> {
        let tlimit = &mut api.kernel_get_system().modules.limits;

        // Increase write coutner.
        tlimit.substate_db_write_count += 1;

        // Increase total size.
        tlimit.substate_db_write_size_total += value_size;

        // Validate
        tlimit.validate_substates(None, Some(value_size))
    }
}
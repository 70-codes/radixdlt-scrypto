use crate::constants::{DEFAULT_COST_UNIT_PRICE, DEFAULT_MAX_CALL_DEPTH, DEFAULT_SYSTEM_LOAN};
use crate::engine::Track;
use crate::engine::*;
use crate::fee::{FeeReserve, FeeTable, SystemLoanFeeReserve};
use crate::ledger::{ReadableSubstateStore, WriteableSubstateStore};
use crate::model::*;
use crate::transaction::*;
use crate::types::*;
use crate::wasm::*;
use sbor::rust::borrow::Cow;
use transaction::model::*;

pub struct FeeReserveConfig {
    pub cost_unit_price: Decimal,
    pub system_loan: u32,
}

impl FeeReserveConfig {
    pub fn standard() -> Self {
        Self {
            cost_unit_price: DEFAULT_COST_UNIT_PRICE
                .parse()
                .expect("Invalid cost unit price"),
            system_loan: DEFAULT_SYSTEM_LOAN,
        }
    }
}

pub struct ExecutionConfig {
    pub max_call_depth: usize,
    pub trace: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        ExecutionConfig::standard()
    }
}

impl ExecutionConfig {
    pub fn standard() -> Self {
        Self {
            max_call_depth: DEFAULT_MAX_CALL_DEPTH,
            trace: false,
        }
    }

    pub fn debug() -> Self {
        Self {
            max_call_depth: DEFAULT_MAX_CALL_DEPTH,
            trace: true,
        }
    }
}

/// An executor that runs transactions.
pub struct TransactionExecutor<'s, 'w, S, W, I>
where
    S: ReadableSubstateStore,
    W: WasmEngine<I>,
    I: WasmInstance,
{
    substate_store: &'s mut S,
    scrypto_interpreter: &'w mut ScryptoInterpreter<I, W>,
}

impl<'s, 'w, S, W, I> TransactionExecutor<'s, 'w, S, W, I>
where
    S: ReadableSubstateStore,
    W: WasmEngine<I>,
    I: WasmInstance,
{
    pub fn new(
        substate_store: &'s mut S,
        scrypto_interpreter: &'w mut ScryptoInterpreter<I, W>,
    ) -> Self {
        Self {
            substate_store,
            scrypto_interpreter,
        }
    }

    pub fn execute(
        &mut self,
        transaction: &Executable,
        fee_reserve_config: &FeeReserveConfig,
        execution_config: &ExecutionConfig,
    ) -> TransactionReceipt {
        let fee_reserve = SystemLoanFeeReserve::new(
            transaction.cost_unit_limit(),
            transaction.tip_percentage(),
            fee_reserve_config.cost_unit_price,
            fee_reserve_config.system_loan,
        );

        self.execute_with_fee_reserve(transaction, execution_config, fee_reserve)
    }

    pub fn execute_with_fee_reserve<R: FeeReserve>(
        &mut self,
        transaction: &Executable,
        execution_config: &ExecutionConfig,
        fee_reserve: R,
    ) -> TransactionReceipt {
        let transaction_hash = transaction.transaction_hash();
        let auth_zone_params = transaction.auth_zone_params();
        let instructions = transaction.instructions();
        let blobs = transaction.blobs();

        #[cfg(not(feature = "alloc"))]
        if execution_config.trace {
            println!("{:-^80}", "Transaction Metadata");
            println!("Transaction hash: {}", transaction_hash);
            println!("Transaction auth zone params: {:?}", auth_zone_params);
            println!("Number of unique blobs: {}", blobs.len());

            println!("{:-^80}", "Engine Execution Log");
        }

        // Prepare state track and execution trace
        let track = Track::new(self.substate_store, fee_reserve, FeeTable::new());

        // Apply pre execution costing
        let pre_execution_result = track.apply_pre_execution_costs(transaction);
        let mut track = match pre_execution_result {
            Ok(track) => track,
            Err(err) => {
                return TransactionReceipt {
                    contents: TransactionContents {
                        instructions: instructions.to_vec(),
                    },
                    execution: TransactionExecution {
                        fee_summary: err.fee_summary,
                        application_logs: vec![],
                    },
                    result: TransactionResult::Reject(RejectResult {
                        error: RejectionError::ErrorBeforeFeeLoanRepaid(RuntimeError::ModuleError(
                            ModuleError::CostingError(CostingError::FeeReserveError(err.error)),
                        )),
                    }),
                };
            }
        };

        // Invoke the function/method
        let invoke_result = {
            let mut modules = Vec::<Box<dyn Module<R>>>::new();
            if execution_config.trace {
                modules.push(Box::new(LoggerModule::new()));
            }
            modules.push(Box::new(CostingModule::default()));
            modules.push(Box::new(ExecutionTraceModule::new()));

            let mut kernel = Kernel::new(
                transaction_hash.clone(),
                auth_zone_params.clone(),
                blobs,
                execution_config.max_call_depth,
                &mut track,
                self.scrypto_interpreter,
                modules,
            );
            kernel
                .invoke_native(NativeInvocation::Function(
                    NativeFunction::TransactionProcessor(TransactionProcessorFunction::Run),
                    ScryptoValue::from_typed(&TransactionProcessorRunInput {
                        intent_validation: Cow::Borrowed(transaction.intent_validation()),
                        instructions: Cow::Borrowed(instructions),
                    }),
                ))
                .map(|o| {
                    scrypto_decode::<Vec<Vec<u8>>>(&o.raw)
                        .expect("TransactionProcessor returned data of unexpected type")
                })
        };

        // Produce the final transaction receipt
        let track_receipt = track.finalize(invoke_result);

        let receipt = TransactionReceipt {
            contents: TransactionContents {
                instructions: instructions.to_vec(),
            },
            execution: TransactionExecution {
                fee_summary: track_receipt.fee_summary,
                application_logs: track_receipt.application_logs,
            },
            result: track_receipt.result,
        };
        #[cfg(not(feature = "alloc"))]
        if execution_config.trace {
            println!("{:-^80}", "Cost Analysis");
            let break_down = receipt
                .execution
                .fee_summary
                .cost_breakdown
                .iter()
                .collect::<BTreeMap<&String, &u32>>();
            for (k, v) in break_down {
                println!("{:<30}: {:>8}", k, v);
            }

            println!("{:-^80}", "Application Logs");
            for (level, message) in &receipt.execution.application_logs {
                println!("[{}] {}", level, message);
            }
            if receipt.execution.application_logs.is_empty() {
                println!("None");
            }
        }
        receipt
    }
}

pub fn execute_and_commit_transaction<
    S: ReadableSubstateStore + WriteableSubstateStore,
    I: WasmInstance,
    W: WasmEngine<I>,
>(
    substate_store: &mut S,
    scrypto_interpreter: &mut ScryptoInterpreter<I, W>,
    fee_reserve_config: &FeeReserveConfig,
    execution_config: &ExecutionConfig,
    transaction: &Executable,
) -> TransactionReceipt {
    let receipt = execute_transaction(
        substate_store,
        scrypto_interpreter,
        fee_reserve_config,
        execution_config,
        transaction,
    );
    if let TransactionResult::Commit(commit) = &receipt.result {
        commit.state_updates.commit(substate_store);
    }
    receipt
}

}

use radix_engine::system::bootstrap::Bootstrapper;
use radix_engine::transaction::execute_and_commit_transaction;
use radix_engine::transaction::{ExecutionConfig, FeeReserveConfig};
use radix_engine::types::*;
use radix_engine::vm::wasm::WasmInstrumenter;
use radix_engine::vm::wasm::{DefaultWasmEngine, WasmMeteringConfig};
use radix_engine::vm::ScryptoVm;
use radix_engine_constants::DEFAULT_COST_UNIT_LIMIT;
use radix_engine_store_interface::interface::{SubstateDatabase, CommittableSubstateDatabase};
use radix_engine_utils::rocks_db_metrics::*;
use scrypto_unit::*;
use std::path::PathBuf;
use transaction::builder::ManifestBuilder;
use transaction::model::TestTransaction;


#[cfg(feature = "rocksdb")]
#[test]
fn db_read_test() {
    let path = PathBuf::from(r"/tmp/radix-scrypto-db");
    // clean database
    std::fs::remove_dir_all(path.clone()).ok();

    let mut substate_db = SubstateStoreWithMetrics::new_rocksdb(path);

    db_read_test_execution(&mut substate_db);

    // export results
    substate_db.export_graph_and_print_summary("/tmp/scrypto-rocksdb-reads-result.png").unwrap();


    let mut substate_db_mem = SubstateStoreWithMetrics::new_inmem();

    db_read_test_execution(&mut substate_db_mem);

    // export results
    substate_db_mem.export_graph_and_print_summary("/tmp/scrypto-inmem-reads-result.png").unwrap();
}



#[cfg(feature = "rocksdb")]
fn db_read_test_execution<S: SubstateDatabase + CommittableSubstateDatabase>( substate_db: &mut S )
{
    // Set up environment.
    let mut scrypto_interpreter = ScryptoVm {
        wasm_engine: DefaultWasmEngine::default(),
        wasm_instrumenter: WasmInstrumenter::default(),
        wasm_metering_config: WasmMeteringConfig::V0,
    };

    Bootstrapper::new(substate_db, &scrypto_interpreter, false)
        .bootstrap_test_default()
        .unwrap();

    // compile and publish scrypto blueprint
    let (code, schema) = Compile::compile("./tests/blueprints/kv_store");

    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET, 100.into())
        .publish_package_advanced(
            code,
            schema,
            BTreeMap::new(),
            BTreeMap::new(),
            AccessRulesConfig::new(),
        )
        .build();
    let package_address = execute_and_commit_transaction(
        substate_db,
        &mut scrypto_interpreter,
        &FeeReserveConfig::default(),
        &ExecutionConfig::default(),
        &TestTransaction::new(manifest.clone(), 1, DEFAULT_COST_UNIT_LIMIT)
            .get_executable(BTreeSet::new()),
    )
    .expect_commit(true)
    .new_package_addresses()[0];

    let max_count = 10u32;
    let cost_unit_limit = u32::MAX;

    // run scrypto blueprint - create component
    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET, 1000.into())
        .call_function(package_address, "DatabaseBench", "new", manifest_args!())
        .build();
    let component = execute_and_commit_transaction(
        substate_db,
        &mut scrypto_interpreter,
        &FeeReserveConfig::default(),
        &ExecutionConfig::default(),
        &TestTransaction::new(manifest.clone(), 2, cost_unit_limit)
            .get_executable(BTreeSet::new()),
    )
    .expect_commit(true)
    .new_component_addresses()[0];

    // fill KV-store with data
    let lengths = vec![ 10000u32, 20000u32, 30000u32, 50000u32, 60000u32, 70000u32, 80000u32, 90000u32, 
        100000u32, 150000u32, 200000u32, 250000u32, 300000u32, 350000u32, 400000u32, 450000u32, 500000u32, 
        550000u32, 600000u32, 650000u32, 700000u32 ];
    for i in 0..lengths.len() {
        let manifest = ManifestBuilder::new()
            .lock_fee(FAUCET, 1000.into())
            .call_method(
                component,
                "insert",
                manifest_args!(lengths[i]),
            )
            .build();
        execute_and_commit_transaction(
            substate_db,
            &mut scrypto_interpreter,
            &FeeReserveConfig::default(),
            &ExecutionConfig::default(),
            &TestTransaction::new(manifest.clone(), (i + 3) as u64, cost_unit_limit)
                .get_executable(BTreeSet::new()),
        )
        .expect_commit(true);
    }

    // read KV-store values
    for _ in 0..1000 {
        for i in 0..lengths.len() {
            let manifest = ManifestBuilder::new()
                .lock_fee(FAUCET, 1000.into())
                .call_method(component, "read", manifest_args!(lengths[i]))
                .build();
            execute_and_commit_transaction(
                substate_db,
                &mut scrypto_interpreter,
                &FeeReserveConfig::default(),
                &ExecutionConfig::default(),
                &TestTransaction::new(manifest.clone(), (max_count * 2 + i as 
                    u32 + 3) as u64, cost_unit_limit)
                    .get_executable(BTreeSet::new()),
            );
        }
    }

}

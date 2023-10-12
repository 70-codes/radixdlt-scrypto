use crate::replay::ledger_transaction::*;
use crate::replay::Error;
use clap::Parser;
use flate2::read::GzDecoder;
use radix_engine::system::bootstrap::*;
use radix_engine::transaction::{execute_transaction, CostingParameters, ExecutionConfig};
use radix_engine::types::*;
use radix_engine::vm::wasm::*;
use radix_engine::vm::{DefaultNativeVm, ScryptoVm, Vm};
use radix_engine_interface::prelude::node_modules::auth::AuthAddresses;
use radix_engine_interface::prelude::NetworkDefinition;
use radix_engine_store_interface::db_key_mapper::SpreadPrefixKeyMapper;
use radix_engine_store_interface::interface::CommittableSubstateDatabase;
use radix_engine_stores::rocks_db_with_merkle_tree::RocksDBWithMerkleTreeSubstateStore;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use tar::Archive;
use transaction::validation::{
    NotarizedTransactionValidator, TransactionValidator, ValidationConfig,
};

/// Run transactions
#[derive(Parser, Debug)]
pub struct Run {
    /// The transaction file, in `.tar.gz` format, with entries sorted
    pub transaction_file: PathBuf,
    /// Path to a folder for storing state
    pub database_dir: PathBuf,

    /// The network to use, [mainnet | stokenet]
    #[clap(short, long)]
    pub network: Option<String>,
    /// The max version to execute
    #[clap(short, long)]
    pub max_version: Option<u64>,
}

impl Run {
    pub fn run(&self) -> Result<(), Error> {
        let network = match &self.network {
            Some(n) => NetworkDefinition::from_str(n).map_err(Error::ParseNetworkError)?,
            None => NetworkDefinition::mainnet(),
        };

        let tar_gz = File::open(&self.transaction_file).map_err(Error::IOError)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);

        let mut database = RocksDBWithMerkleTreeSubstateStore::standard(self.database_dir.clone());
        let scrypto_vm = ScryptoVm::<DefaultWasmEngine>::default();
        let start = std::time::Instant::now();
        for entry in archive.entries().map_err(Error::IOError)? {
            // check limit
            let version = database.get_current_version();
            if version >= self.max_version.unwrap_or(u64::MAX) {
                break;
            }

            // read the entry
            let mut entry = entry.map_err(|e| Error::IOError(e))?;
            let tx_version = entry
                .header()
                .path()
                .ok()
                .and_then(|path| path.to_str().map(ToOwned::to_owned))
                .and_then(|s| u64::from_str(&s).ok())
                .ok_or(Error::InvalidTransactionFile)?;
            let mut tx_payload = Vec::new();
            entry
                .read_to_end(&mut tx_payload)
                .map_err(|e| Error::IOError(e))?;
            if tx_version <= version {
                continue;
            }

            // execute transaction
            let transaction = LedgerTransaction::from_payload_bytes(&tx_payload)
                .expect("Failed to decode transaction");
            let prepared = transaction
                .prepare()
                .expect("Failed to prepare transaction");
            let state_updates = match &prepared.inner {
                PreparedLedgerTransactionInner::Genesis(prepared_genesis_tx) => {
                    match prepared_genesis_tx.as_ref() {
                        PreparedGenesisTransaction::Flash(_) => {
                            let receipt = create_substate_flash_for_genesis();
                            receipt.state_updates
                        }
                        PreparedGenesisTransaction::Transaction(tx) => {
                            let receipt = execute_transaction(
                                &database,
                                Vm {
                                    scrypto_vm: &scrypto_vm,
                                    native_vm: DefaultNativeVm::new(),
                                },
                                &CostingParameters::default(),
                                &ExecutionConfig::for_genesis_transaction(network.clone()),
                                &tx.get_executable(btreeset!(AuthAddresses::system_role())),
                            );
                            receipt.expect_commit_ignore_outcome().state_updates.clone()
                        }
                    }
                }
                PreparedLedgerTransactionInner::UserV1(tx) => {
                    let receipt = execute_transaction(
                        &database,
                        Vm {
                            scrypto_vm: &scrypto_vm,
                            native_vm: DefaultNativeVm::new(),
                        },
                        &CostingParameters::default(),
                        &ExecutionConfig::for_notarized_transaction(network.clone()),
                        &NotarizedTransactionValidator::new(ValidationConfig::default(network.id))
                            .validate(tx.as_ref().clone())
                            .expect("Transaction validation failure")
                            .get_executable(),
                    );
                    receipt.expect_commit_ignore_outcome().state_updates.clone()
                }
                PreparedLedgerTransactionInner::RoundUpdateV1(tx) => {
                    let receipt = execute_transaction(
                        &database,
                        Vm {
                            scrypto_vm: &scrypto_vm,
                            native_vm: DefaultNativeVm::new(),
                        },
                        &CostingParameters::default(),
                        &ExecutionConfig::for_system_transaction(network.clone()),
                        &tx.get_executable(),
                    );
                    receipt.expect_commit_ignore_outcome().state_updates.clone()
                }
            };
            let database_updates = state_updates.create_database_updates::<SpreadPrefixKeyMapper>();
            database.commit(&database_updates);

            // print progress
            let new_version = database.get_current_version();
            if new_version < 1000 || new_version % 1000 == 0 {
                let new_root = database.get_current_root_hash();
                println!("New version: {}, {}", new_version, new_root);
            }
        }
        let duration = start.elapsed();
        println!("Time elapsed: {:?}", duration);
        println!("State version: {}", database.get_current_version());
        println!("State root hash: {}", database.get_current_root_hash());

        Ok(())
    }
}
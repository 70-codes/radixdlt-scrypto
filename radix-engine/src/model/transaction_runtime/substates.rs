use crate::types::*;

#[derive(Debug, Clone, PartialEq, Eq)]
#[scrypto(Categorize, Encode, Decode)]
pub struct TransactionRuntimeSubstate {
    pub hash: Hash,
    pub next_id: u32,
    pub instruction_index: u32,
}
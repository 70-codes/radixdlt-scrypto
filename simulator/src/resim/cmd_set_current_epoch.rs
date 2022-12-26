use clap::Parser;
use radix_engine::types::*;
use radix_engine_interface::modules::auth::AuthAddresses;
use radix_engine_interface::wasm::{
    EpochManagerMethodInvocation, NativeFnInvocation, NativeMethodInvocation,
};

use crate::resim::*;

/// Set the current epoch
#[derive(Parser, Debug)]
pub struct SetCurrentEpoch {
    /// The new epoch number
    epoch: u64,

    /// Turn on tracing
    #[clap(short, long)]
    trace: bool,
}

impl SetCurrentEpoch {
    pub fn run<O: std::io::Write>(&self, out: &mut O) -> Result<(), Error> {
        let instructions = vec![Instruction::System(NativeFnInvocation::Method(
            NativeMethodInvocation::EpochManager(EpochManagerMethodInvocation::SetEpoch(
                EpochManagerSetEpochInvocation {
                    receiver: EPOCH_MANAGER,
                    epoch: self.epoch,
                },
            )),
        ))];

        let blobs = vec![];
        let initial_proofs = vec![AuthAddresses::system_role()];
        handle_system_transaction(instructions, blobs, initial_proofs, self.trace, true, out)
            .map(|_| ())
    }
}

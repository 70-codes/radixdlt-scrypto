use crate::api::api::Invocation;
use crate::math::Decimal;
use crate::model::*;
use crate::scrypto;
use crate::wasm::*;
use sbor::rust::collections::BTreeSet;
use sbor::rust::vec::Vec;
use sbor::*;

#[derive(Debug, Eq, PartialEq)]
#[scrypto(Categorize, Encode, Decode)]
pub struct WorktopPutInvocation {
    pub bucket: Bucket,
}

impl Clone for WorktopPutInvocation {
    fn clone(&self) -> Self {
        Self {
            bucket: Bucket(self.bucket.0),
        }
    }
}

impl Invocation for WorktopPutInvocation {
    type Output = ();
}

impl SerializableInvocation for WorktopPutInvocation {
    type ScryptoOutput = ();
}

impl Into<CallTableInvocation> for WorktopPutInvocation {
    fn into(self) -> CallTableInvocation {
        NativeInvocation::Worktop(WorktopInvocation::Put(self)).into()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[scrypto(Categorize, Encode, Decode)]
pub struct WorktopTakeAmountInvocation {
    pub amount: Decimal,
    pub resource_address: ResourceAddress,
}

impl Invocation for WorktopTakeAmountInvocation {
    type Output = Bucket;
}

impl SerializableInvocation for WorktopTakeAmountInvocation {
    type ScryptoOutput = Bucket;
}

impl Into<CallTableInvocation> for WorktopTakeAmountInvocation {
    fn into(self) -> CallTableInvocation {
        NativeInvocation::Worktop(WorktopInvocation::TakeAmount(self)).into()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[scrypto(Categorize, Encode, Decode)]
pub struct WorktopTakeNonFungiblesInvocation {
    pub ids: BTreeSet<NonFungibleId>,
    pub resource_address: ResourceAddress,
}

impl Invocation for WorktopTakeNonFungiblesInvocation {
    type Output = Bucket;
}

impl SerializableInvocation for WorktopTakeNonFungiblesInvocation {
    type ScryptoOutput = Bucket;
}

impl Into<CallTableInvocation> for WorktopTakeNonFungiblesInvocation {
    fn into(self) -> CallTableInvocation {
        NativeInvocation::Worktop(WorktopInvocation::TakeNonFungibles(self)).into()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[scrypto(Categorize, Encode, Decode)]
pub struct WorktopTakeAllInvocation {
    pub resource_address: ResourceAddress,
}

impl Invocation for WorktopTakeAllInvocation {
    type Output = Bucket;
}

impl SerializableInvocation for WorktopTakeAllInvocation {
    type ScryptoOutput = Bucket;
}

impl Into<CallTableInvocation> for WorktopTakeAllInvocation {
    fn into(self) -> CallTableInvocation {
        NativeInvocation::Worktop(WorktopInvocation::TakeAll(self)).into()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[scrypto(Categorize, Encode, Decode)]
pub struct WorktopAssertContainsInvocation {
    pub resource_address: ResourceAddress,
}

impl Invocation for WorktopAssertContainsInvocation {
    type Output = ();
}

impl SerializableInvocation for WorktopAssertContainsInvocation {
    type ScryptoOutput = ();
}

impl Into<CallTableInvocation> for WorktopAssertContainsInvocation {
    fn into(self) -> CallTableInvocation {
        NativeInvocation::Worktop(WorktopInvocation::AssertContains(self)).into()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[scrypto(Categorize, Encode, Decode)]
pub struct WorktopAssertContainsAmountInvocation {
    pub resource_address: ResourceAddress,
    pub amount: Decimal,
}
impl Invocation for WorktopAssertContainsAmountInvocation {
    type Output = ();
}

impl SerializableInvocation for WorktopAssertContainsAmountInvocation {
    type ScryptoOutput = ();
}

impl Into<CallTableInvocation> for WorktopAssertContainsAmountInvocation {
    fn into(self) -> CallTableInvocation {
        NativeInvocation::Worktop(WorktopInvocation::AssertContainsAmount(self)).into()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[scrypto(Categorize, Encode, Decode)]
pub struct WorktopAssertContainsNonFungiblesInvocation {
    pub resource_address: ResourceAddress,
    pub ids: BTreeSet<NonFungibleId>,
}

impl Invocation for WorktopAssertContainsNonFungiblesInvocation {
    type Output = ();
}

impl SerializableInvocation for WorktopAssertContainsNonFungiblesInvocation {
    type ScryptoOutput = ();
}

impl Into<CallTableInvocation> for WorktopAssertContainsNonFungiblesInvocation {
    fn into(self) -> CallTableInvocation {
        NativeInvocation::Worktop(WorktopInvocation::AssertContainsNonFungibles(self)).into()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[scrypto(Categorize, Encode, Decode)]
pub struct WorktopDrainInvocation {}

impl Invocation for WorktopDrainInvocation {
    type Output = Vec<Bucket>;
}

impl SerializableInvocation for WorktopDrainInvocation {
    type ScryptoOutput = Vec<Bucket>;
}

impl Into<CallTableInvocation> for WorktopDrainInvocation {
    fn into(self) -> CallTableInvocation {
        NativeInvocation::Worktop(WorktopInvocation::Drain(self)).into()
    }
}

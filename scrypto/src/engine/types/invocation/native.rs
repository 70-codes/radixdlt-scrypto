use crate::engine::types::*;

// Native function identifier used by transaction model
#[derive(Debug, Clone, Eq, PartialEq, Hash, TypeId, Encode, Decode)]
pub struct NativeFunctionIdent {
    pub blueprint_name: String,
    pub function_name: String,
}

// Native method identifier used by transaction model
#[derive(Debug, Clone, Eq, PartialEq, TypeId, Encode, Decode)]
pub struct NativeMethodIdent {
    pub receiver: Receiver,
    pub method_name: String,
}

// Native function enum used by Kernel SystemAPI and WASM
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TypeId, Encode, Decode, PartialOrd, Ord)]
pub enum NativeMethod {
    Component(ComponentMethod),
    System(SystemMethod),
    AuthZone(AuthZoneMethod),
    ResourceManager(ResourceManagerMethod),
    Bucket(BucketMethod),
    Vault(VaultMethod),
    Proof(ProofMethod),
    Worktop(WorktopMethod),
}

// Native method enum used by Kernel SystemAPI and WASM
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TypeId, Encode, Decode, PartialOrd, Ord)]
pub enum NativeFunction {
    System(SystemFunction),
    ResourceManager(ResourceManagerFunction),
    Package(PackageFunction),
    TransactionProcessor(TransactionProcessorFunction),
}

#[derive(Debug, Clone, Eq, PartialEq, Copy, TypeId, Encode, Decode)]
pub enum Receiver {
    Ref(RENodeId),
}

impl Receiver {
    pub fn node_id(&self) -> RENodeId {
        match self {
            Receiver::Ref(node_id) => *node_id,
        }
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    TypeId,
    Encode,
    Decode,
    Describe,
    PartialOrd,
    Ord,
    EnumString,
    EnumVariantNames,
    IntoStaticStr,
    AsRefStr,
    Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum ComponentMethod {
    AddAccessCheck,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    TypeId,
    Encode,
    Decode,
    Describe,
    PartialOrd,
    Ord,
    EnumString,
    EnumVariantNames,
    IntoStaticStr,
    AsRefStr,
    Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum SystemFunction {
    Create,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    TypeId,
    Encode,
    Decode,
    Describe,
    PartialOrd,
    Ord,
    EnumString,
    EnumVariantNames,
    IntoStaticStr,
    AsRefStr,
    Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum SystemMethod {
    GetTransactionHash,
    GetCurrentEpoch,
    SetEpoch,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    TypeId,
    Encode,
    Decode,
    Describe,
    PartialOrd,
    Ord,
    EnumString,
    EnumVariantNames,
    IntoStaticStr,
    AsRefStr,
    Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum AuthZoneMethod {
    Pop,
    Push,
    CreateProof,
    CreateProofByAmount,
    CreateProofByIds,
    Clear,
    Drain,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    TypeId,
    Encode,
    Decode,
    Describe,
    PartialOrd,
    Ord,
    EnumString,
    EnumVariantNames,
    IntoStaticStr,
    AsRefStr,
    Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum ResourceManagerFunction {
    Create,
    BurnBucket,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    TypeId,
    Encode,
    Decode,
    Describe,
    PartialOrd,
    Ord,
    EnumString,
    EnumVariantNames,
    IntoStaticStr,
    AsRefStr,
    Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum ResourceManagerMethod {
    Burn,
    UpdateAuth,
    LockAuth,
    Mint,
    UpdateNonFungibleData,
    GetNonFungible,
    GetMetadata,
    GetResourceType,
    GetTotalSupply,
    UpdateMetadata,
    NonFungibleExists,
    CreateBucket,
    CreateVault,
    SetResourceAddress,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    TypeId,
    Encode,
    Decode,
    Describe,
    PartialOrd,
    Ord,
    EnumString,
    EnumVariantNames,
    IntoStaticStr,
    AsRefStr,
    Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum BucketMethod {
    Take,
    TakeNonFungibles,
    Put,
    GetNonFungibleIds,
    GetAmount,
    GetResourceAddress,
    CreateProof,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    TypeId,
    Encode,
    Decode,
    Describe,
    PartialOrd,
    Ord,
    EnumString,
    EnumVariantNames,
    IntoStaticStr,
    AsRefStr,
    Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum VaultMethod {
    Take,
    LockFee,
    LockContingentFee,
    Put,
    TakeNonFungibles,
    GetAmount,
    GetResourceAddress,
    GetNonFungibleIds,
    CreateProof,
    CreateProofByAmount,
    CreateProofByIds,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    TypeId,
    Encode,
    Decode,
    Describe,
    PartialOrd,
    Ord,
    EnumString,
    EnumVariantNames,
    IntoStaticStr,
    AsRefStr,
    Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum ProofMethod {
    Clone,
    GetAmount,
    GetNonFungibleIds,
    GetResourceAddress,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    TypeId,
    Encode,
    Decode,
    Describe,
    PartialOrd,
    Ord,
    EnumString,
    EnumVariantNames,
    IntoStaticStr,
    AsRefStr,
    Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum WorktopMethod {
    TakeAll,
    TakeAmount,
    TakeNonFungibles,
    Put,
    AssertContains,
    AssertContainsAmount,
    AssertContainsNonFungibles,
    Drain,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    TypeId,
    Encode,
    Decode,
    Describe,
    PartialOrd,
    Ord,
    EnumString,
    EnumVariantNames,
    IntoStaticStr,
    AsRefStr,
    Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum PackageFunction {
    Publish,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    TypeId,
    Encode,
    Decode,
    Describe,
    PartialOrd,
    Ord,
    EnumString,
    EnumVariantNames,
    IntoStaticStr,
    AsRefStr,
    Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum TransactionProcessorFunction {
    Run,
}

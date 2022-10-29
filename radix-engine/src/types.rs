pub use sbor::rust::borrow::ToOwned;
pub use sbor::rust::boxed::Box;
pub use sbor::rust::cell::{Ref, RefCell, RefMut};
pub use sbor::rust::collections::*;
pub use sbor::rust::fmt;
pub use sbor::rust::format;
pub use sbor::rust::marker::PhantomData;
pub use sbor::rust::ops::AddAssign;
pub use sbor::rust::ptr;
pub use sbor::rust::rc::Rc;
pub use sbor::rust::str::FromStr;
pub use sbor::rust::string::String;
pub use sbor::rust::string::ToString;
pub use sbor::rust::vec;
pub use sbor::rust::vec::Vec;
pub use sbor::{Decode, DecodeError, Encode, Type, TypeId, Value};
pub use scrypto::abi::{BlueprintAbi, Fn, ScryptoType};
pub use scrypto::address::{AddressError, Bech32Decoder, Bech32Encoder};
pub use scrypto::component::{
    ComponentAddAccessCheckInput, ComponentAddress, PackageAddress, PackagePublishInput,
};
pub use scrypto::constants::*;
pub use scrypto::core::{
    Blob, EpochManagerCreateInput, EpochManagerGetCurrentEpochInput, EpochManagerSetEpochInput,
    Expression, NetworkDefinition, ScryptoActor, SystemAddress,
};
pub use scrypto::crypto::{
    EcdsaSecp256k1PublicKey, EcdsaSecp256k1Signature, EddsaEd25519PublicKey, EddsaEd25519Signature,
    Hash, PublicKey, Signature,
};
pub use scrypto::engine::{api::RadixEngineInput, types::*};
pub use scrypto::math::{Decimal, RoundingMode, I256};
pub use scrypto::resource::{
    AccessRule, AccessRuleNode, AccessRules, AuthZoneClearInput, AuthZoneCreateProofByAmountInput,
    AuthZoneCreateProofByIdsInput, AuthZoneCreateProofInput, AuthZonePopInput, AuthZonePushInput,
    BucketCreateProofInput, BucketGetAmountInput, BucketGetNonFungibleIdsInput,
    BucketGetResourceAddressInput, BucketPutInput, BucketTakeInput, BucketTakeNonFungiblesInput,
    MintParams, Mutability, NonFungibleAddress, NonFungibleId, ProofCloneInput,
    ProofGetAmountInput, ProofGetNonFungibleIdsInput, ProofGetResourceAddressInput, ProofRule,
    ResourceAddress, ResourceManagerBurnInput, ResourceManagerCreateBucketInput,
    ResourceManagerCreateInput, ResourceManagerCreateVaultInput, ResourceManagerGetMetadataInput,
    ResourceManagerGetNonFungibleInput, ResourceManagerGetResourceTypeInput,
    ResourceManagerGetTotalSupplyInput, ResourceManagerLockAuthInput, ResourceManagerMintInput,
    ResourceManagerNonFungibleExistsInput, ResourceManagerSetResourceAddressInput,
    ResourceManagerUpdateAuthInput, ResourceManagerUpdateMetadataInput,
    ResourceManagerUpdateNonFungibleDataInput, ResourceMethodAuthKey, ResourceType, SoftCount,
    SoftDecimal, SoftResource, SoftResourceOrNonFungible, SoftResourceOrNonFungibleList,
    VaultCreateProofByAmountInput, VaultCreateProofByIdsInput, VaultCreateProofInput,
    VaultGetAmountInput, VaultGetNonFungibleIdsInput, VaultGetResourceAddressInput,
    VaultLockFeeInput, VaultPutInput, VaultTakeInput, VaultTakeNonFungiblesInput, LOCKED, MUTABLE,
};
pub use scrypto::values::{ScryptoValue, ScryptoValueReplaceError};
use std::fmt::Debug;

// methods and macros
use crate::engine::{Invocation, REActor, RENodeLocation, RuntimeError};
pub use sbor::decode_any;
pub use scrypto::buffer::{scrypto_decode, scrypto_encode};
pub use scrypto::crypto::hash;
pub use scrypto::resource::{
    require, require_all_of, require_amount, require_any_of, require_n_of,
};
pub use scrypto::{access_and_or, access_rule_node, args, dec, pdec, rule};

/// Scrypto function/method invocation.
#[derive(Debug)]
pub enum ScryptoInvocation {
    Function(ScryptoFunctionIdent, ScryptoValue),
    Method(ScryptoMethodIdent, ScryptoValue),
}

impl Invocation<ScryptoValue> for ScryptoInvocation {}

impl ScryptoInvocation {
    pub fn args(&self) -> &ScryptoValue {
        match self {
            ScryptoInvocation::Function(_, args) => &args,
            ScryptoInvocation::Method(_, args) => &args,
        }
    }
}

#[derive(Debug)]
pub struct NativeFunctionInvocation(pub NativeFunction, pub ScryptoValue);

impl Invocation<ScryptoValue> for NativeFunctionInvocation {}

impl NativeFunctionInvocation {
    pub fn args(&self) -> &ScryptoValue {
        &self.1
    }
}

#[derive(Debug)]
pub struct NativeMethodInvocation(pub NativeMethod, pub RENodeId, pub ScryptoValue);

impl Invocation<ScryptoValue> for NativeMethodInvocation {}

impl NativeMethodInvocation {
    pub fn args(&self) -> &ScryptoValue {
        &self.2
    }
}

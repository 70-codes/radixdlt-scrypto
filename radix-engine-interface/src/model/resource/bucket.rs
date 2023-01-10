use sbor::rust::collections::BTreeSet;
use sbor::rust::fmt::Debug;
use sbor::*;

use crate::abi::*;
use crate::api::{api::*, types::*};
use crate::data::types::Own;
use crate::data::ScryptoCustomValueKind;
use crate::math::*;
use crate::scrypto;
use crate::wasm::*;

#[derive(Debug, Clone, Eq, PartialEq)]
#[scrypto(Categorize, Encode, Decode)]
pub struct BucketTakeInvocation {
    pub receiver: BucketId,
    pub amount: Decimal,
}

impl Invocation for BucketTakeInvocation {
    type Output = Bucket;
}

impl SerializableInvocation for BucketTakeInvocation {
    type ScryptoOutput = Bucket;
}

impl Into<SerializedInvocation> for BucketTakeInvocation {
    fn into(self) -> SerializedInvocation {
        NativeInvocation::Bucket(BucketInvocation::Take(self)).into()
    }
}

#[derive(Debug, Eq, PartialEq)]
#[scrypto(Categorize, Encode, Decode)]
pub struct BucketPutInvocation {
    pub receiver: BucketId,
    pub bucket: Bucket,
}

impl Clone for BucketPutInvocation {
    fn clone(&self) -> Self {
        Self {
            receiver: self.receiver,
            bucket: Bucket(self.bucket.0),
        }
    }
}

impl Invocation for BucketPutInvocation {
    type Output = ();
}

impl SerializableInvocation for BucketPutInvocation {
    type ScryptoOutput = ();
}

impl Into<SerializedInvocation> for BucketPutInvocation {
    fn into(self) -> SerializedInvocation {
        NativeInvocation::Bucket(BucketInvocation::Put(self)).into()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[scrypto(Categorize, Encode, Decode)]
pub struct BucketTakeNonFungiblesInvocation {
    pub receiver: BucketId,
    pub ids: BTreeSet<NonFungibleId>,
}

impl Invocation for BucketTakeNonFungiblesInvocation {
    type Output = Bucket;
}

impl SerializableInvocation for BucketTakeNonFungiblesInvocation {
    type ScryptoOutput = Bucket;
}

impl Into<SerializedInvocation> for BucketTakeNonFungiblesInvocation {
    fn into(self) -> SerializedInvocation {
        NativeInvocation::Bucket(BucketInvocation::TakeNonFungibles(self)).into()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[scrypto(Categorize, Encode, Decode)]
pub struct BucketGetNonFungibleIdsInvocation {
    pub receiver: BucketId,
}

impl Invocation for BucketGetNonFungibleIdsInvocation {
    type Output = BTreeSet<NonFungibleId>;
}

impl SerializableInvocation for BucketGetNonFungibleIdsInvocation {
    type ScryptoOutput = BTreeSet<NonFungibleId>;
}

impl Into<SerializedInvocation> for BucketGetNonFungibleIdsInvocation {
    fn into(self) -> SerializedInvocation {
        NativeInvocation::Bucket(BucketInvocation::GetNonFungibleIds(self)).into()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[scrypto(Categorize, Encode, Decode)]
pub struct BucketGetAmountInvocation {
    pub receiver: BucketId,
}

impl Invocation for BucketGetAmountInvocation {
    type Output = Decimal;
}

impl SerializableInvocation for BucketGetAmountInvocation {
    type ScryptoOutput = Decimal;
}

impl Into<SerializedInvocation> for BucketGetAmountInvocation {
    fn into(self) -> SerializedInvocation {
        NativeInvocation::Bucket(BucketInvocation::GetAmount(self)).into()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[scrypto(Categorize, Encode, Decode)]
pub struct BucketGetResourceAddressInvocation {
    pub receiver: BucketId,
}

impl Invocation for BucketGetResourceAddressInvocation {
    type Output = ResourceAddress;
}

impl SerializableInvocation for BucketGetResourceAddressInvocation {
    type ScryptoOutput = ResourceAddress;
}

impl Into<SerializedInvocation> for BucketGetResourceAddressInvocation {
    fn into(self) -> SerializedInvocation {
        NativeInvocation::Bucket(BucketInvocation::GetResourceAddress(self)).into()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[scrypto(Categorize, Encode, Decode)]
pub struct BucketCreateProofInvocation {
    pub receiver: BucketId,
}

impl Invocation for BucketCreateProofInvocation {
    type Output = Proof;
}

impl SerializableInvocation for BucketCreateProofInvocation {
    type ScryptoOutput = Proof;
}

impl Into<SerializedInvocation> for BucketCreateProofInvocation {
    fn into(self) -> SerializedInvocation {
        NativeInvocation::Bucket(BucketInvocation::CreateProof(self)).into()
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Bucket(pub BucketId); // scrypto stub

//========
// binary
//========

impl Categorize<ScryptoCustomValueKind> for Bucket {
    #[inline]
    fn value_kind() -> ValueKind<ScryptoCustomValueKind> {
        ValueKind::Custom(ScryptoCustomValueKind::Own)
    }
}

impl<E: Encoder<ScryptoCustomValueKind>> Encode<ScryptoCustomValueKind, E> for Bucket {
    #[inline]
    fn encode_value_kind(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encoder.write_value_kind(Self::value_kind())
    }

    #[inline]
    fn encode_body(&self, encoder: &mut E) -> Result<(), EncodeError> {
        Own::Bucket(self.0).encode_body(encoder)
    }
}

impl<D: Decoder<ScryptoCustomValueKind>> Decode<ScryptoCustomValueKind, D> for Bucket {
    fn decode_body_with_value_kind(
        decoder: &mut D,
        value_kind: ValueKind<ScryptoCustomValueKind>,
    ) -> Result<Self, DecodeError> {
        let o = Own::decode_body_with_value_kind(decoder, value_kind)?;
        match o {
            Own::Bucket(bucket_id) => Ok(Self(bucket_id)),
            _ => Err(DecodeError::InvalidCustomValue),
        }
    }
}

impl scrypto_abi::LegacyDescribe for Bucket {
    fn describe() -> scrypto_abi::Type {
        Type::Bucket
    }
}

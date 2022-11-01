use sbor::rust::borrow::ToOwned;
use sbor::rust::fmt;
use sbor::rust::fmt::Debug;
use sbor::rust::str::FromStr;
use sbor::rust::string::String;
use sbor::rust::string::ToString;
use sbor::rust::vec::Vec;
use sbor::*;
use scrypto::buffer::scrypto_decode;

use crate::abi::*;
use crate::address::*;
use crate::buffer::scrypto_encode;
use crate::component::*;
use crate::core::*;
use crate::crypto::{hash, Hash, PublicKey};
use crate::engine::{api::*, types::*, utils::*};
use crate::misc::*;
use crate::resource::AccessRules;
use crate::values::ScryptoValue;

#[derive(Debug, TypeId, Encode, Decode)]
pub struct ComponentAddAccessCheckInput {
    pub component_id: ComponentId,
    pub access_rules: AccessRules,
}

impl SysInvocation for ComponentAddAccessCheckInput {
    type Output = ();

    fn native_method() -> NativeMethod {
        NativeMethod::Component(ComponentMethod::AddAccessCheck)
    }
}

/// Represents the state of a component.
pub trait ComponentState<C: LocalComponent>: Encode + Decode {
    /// Instantiates a component from this data structure.
    fn instantiate(self) -> C;
}

pub trait LocalComponent {
    fn package_address(&self) -> PackageAddress;
    fn blueprint_name(&self) -> String;
    fn add_access_check(&mut self, access_rules: AccessRules) -> &mut Self;
    fn globalize(self) -> ComponentAddress;
}

/// Represents an instantiated component.
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Component(pub ComponentId);

// TODO: de-duplication
#[derive(Debug, Clone, TypeId, Encode, Decode, Describe, PartialEq, Eq)]
pub struct ComponentInfoSubstate {
    pub package_address: PackageAddress,
    pub blueprint_name: String,
    pub access_rules: Vec<AccessRules>,
}

// TODO: de-duplication
#[derive(Debug, Clone, TypeId, Encode, Decode, Describe, PartialEq, Eq)]
pub struct ComponentStateSubstate {
    pub raw: Vec<u8>,
}

impl Component {
    pub fn call<T: Decode>(&self, method: &str, args: Vec<u8>) -> T {
        let mut sys_calls = Syscalls;
        let rtn = sys_calls.sys_invoke_scrypto_method(
            ScryptoMethodIdent {
                receiver: ScryptoReceiver::Component(self.0),
                method_name: method.to_string(),
            },
            args,
        ).unwrap();
        scrypto_decode(&rtn).unwrap()
    }

    /// Returns the package ID of this component.
    pub fn package_address(&self) -> PackageAddress {
        let pointer = DataPointer::new(
            RENodeId::Component(self.0),
            SubstateOffset::Component(ComponentOffset::Info),
        );
        let state: DataRef<ComponentInfoSubstate> = pointer.get();
        state.package_address
    }

    /// Returns the blueprint name of this component.
    pub fn blueprint_name(&self) -> String {
        let pointer = DataPointer::new(
            RENodeId::Component(self.0),
            SubstateOffset::Component(ComponentOffset::Info),
        );
        let state: DataRef<ComponentInfoSubstate> = pointer.get();
        state.blueprint_name.clone()
    }

    pub fn add_access_check(&mut self, access_rules: AccessRules) -> &mut Self {
        self.sys_add_access_check(access_rules, &mut Syscalls)
            .unwrap()
    }

    pub fn sys_add_access_check<Y, E: Debug + Decode>(
        &mut self,
        access_rules: AccessRules,
        sys_calls: &mut Y,
    ) -> Result<&mut Self, E>
    where
        Y: ScryptoSyscalls<E> + SysInvokable<ComponentAddAccessCheckInput, E>,
    {
        sys_calls.sys_invoke(ComponentAddAccessCheckInput {
            access_rules,
            component_id: self.0,
        })?;

        Ok(self)
    }

    pub fn globalize(self) -> ComponentAddress {
        self.sys_globalize(&mut Syscalls).unwrap()
    }

    pub fn sys_globalize<Y, E: Debug + Decode>(
        self,
        sys_calls: &mut Y,
    ) -> Result<ComponentAddress, E>
    where
        Y: ScryptoSyscalls<E>,
    {
        let node_id: RENodeId =
            sys_calls.sys_create_node(ScryptoRENode::GlobalComponent(self.0))?;
        Ok(node_id.into())
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct BorrowedGlobalComponent(pub ComponentAddress);

impl BorrowedGlobalComponent {
    /// Invokes a method on this component.
    pub fn call<T: Decode>(&self, method: &str, args: Vec<u8>) -> T {
        let input = RadixEngineInput::InvokeScryptoMethod(
            ScryptoMethodIdent {
                receiver: ScryptoReceiver::Global(self.0),
                method_name: method.to_string(),
            },
            args,
        );
        call_engine(input)
    }

    /// Returns the package ID of this component.
    pub fn package_address(&self) -> PackageAddress {
        let pointer = DataPointer::new(
            RENodeId::Global(GlobalAddress::Component(self.0)),
            SubstateOffset::Component(ComponentOffset::Info),
        );
        let state: DataRef<ComponentInfoSubstate> = pointer.get();
        state.package_address
    }

    /// Returns the blueprint name of this component.
    pub fn blueprint_name(&self) -> String {
        let pointer = DataPointer::new(
            RENodeId::Global(GlobalAddress::Component(self.0)),
            SubstateOffset::Component(ComponentOffset::Info),
        );
        let state: DataRef<ComponentInfoSubstate> = pointer.get();
        state.blueprint_name.clone()
    }
}

//========
// binary
//========

/// Represents an error when decoding key value store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseComponentError {
    InvalidHex(String),
    InvalidLength(usize),
}

impl TryFrom<&[u8]> for Component {
    type Error = ParseComponentError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        match slice.len() {
            36 => Ok(Self((
                Hash(copy_u8_array(&slice[0..32])),
                u32::from_le_bytes(copy_u8_array(&slice[32..])),
            ))),
            _ => Err(ParseComponentError::InvalidLength(slice.len())),
        }
    }
}

impl Component {
    pub fn to_vec(&self) -> Vec<u8> {
        let mut v = self.0 .0.to_vec();
        v.extend(self.0 .1.to_le_bytes());
        v
    }
}

scrypto_type!(Component, ScryptoType::Component, Vec::new());

//======
// text
//======

impl FromStr for Component {
    type Err = ParseComponentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s).map_err(|_| ParseComponentError::InvalidHex(s.to_owned()))?;
        Self::try_from(bytes.as_slice())
    }
}

impl fmt::Display for Component {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", hex::encode(self.to_vec()))
    }
}

impl fmt::Debug for Component {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self.0)
    }
}

/// An instance of a blueprint, which lives in the ledger state.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ComponentAddress {
    Normal([u8; 26]),
    Account([u8; 26]),
    EcdsaSecp256k1VirtualAccount([u8; 26]),
    EddsaEd25519VirtualAccount([u8; 26]),
}

//========
// binary
//========

impl TryFrom<&[u8]> for ComponentAddress {
    type Error = AddressError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        match slice.len() {
            27 => match EntityType::try_from(slice[0])
                .map_err(|_| AddressError::InvalidEntityTypeId(slice[0]))?
            {
                EntityType::NormalComponent => Ok(Self::Normal(copy_u8_array(&slice[1..]))),
                EntityType::AccountComponent => Ok(Self::Account(copy_u8_array(&slice[1..]))),
                EntityType::EcdsaSecp256k1VirtualAccountComponent => Ok(
                    Self::EcdsaSecp256k1VirtualAccount(copy_u8_array(&slice[1..])),
                ),
                EntityType::EddsaEd25519VirtualAccountComponent => {
                    Ok(Self::EddsaEd25519VirtualAccount(copy_u8_array(&slice[1..])))
                }
                _ => Err(AddressError::InvalidEntityTypeId(slice[0])),
            },
            _ => Err(AddressError::InvalidLength(slice.len())),
        }
    }
}

impl ComponentAddress {
    pub fn virtual_account_from_public_key<P: Into<PublicKey> + Clone>(public_key: &P) -> Self {
        match public_key.clone().into() {
            PublicKey::EcdsaSecp256k1(public_key) => {
                ComponentAddress::EcdsaSecp256k1VirtualAccount(
                    hash(public_key.to_vec()).lower_26_bytes(),
                )
            }
            PublicKey::EddsaEd25519(public_key) => ComponentAddress::EddsaEd25519VirtualAccount(
                hash(public_key.to_vec()).lower_26_bytes(),
            ),
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(EntityType::component(self).id());
        match self {
            Self::Normal(v)
            | Self::Account(v)
            | Self::EddsaEd25519VirtualAccount(v)
            | Self::EcdsaSecp256k1VirtualAccount(v) => buf.extend(v),
        }
        buf
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.to_vec())
    }

    pub fn try_from_hex(hex_str: &str) -> Result<Self, AddressError> {
        let bytes = hex::decode(hex_str).map_err(|_| AddressError::HexDecodingError)?;

        Self::try_from(bytes.as_ref())
    }
}

scrypto_type!(ComponentAddress, ScryptoType::ComponentAddress, Vec::new());

//======
// text
//======

impl fmt::Debug for ComponentAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.display(NO_NETWORK))
    }
}

impl<'a> ContextualDisplay<AddressDisplayContext<'a>> for ComponentAddress {
    type Error = AddressError;

    fn contextual_format<F: fmt::Write>(
        &self,
        f: &mut F,
        context: &AddressDisplayContext<'a>,
    ) -> Result<(), Self::Error> {
        if let Some(encoder) = context.encoder {
            return encoder.encode_component_address_to_fmt(f, self);
        }

        // This could be made more performant by streaming the hex into the formatter
        match self {
            ComponentAddress::Normal(_) => {
                write!(f, "NormalComponent[{}]", self.to_hex())
            }
            ComponentAddress::Account(_) => {
                write!(f, "AccountComponent[{}]", self.to_hex())
            }
            ComponentAddress::EcdsaSecp256k1VirtualAccount(_) => {
                write!(
                    f,
                    "EcdsaSecp256k1VirtualAccountComponent[{}]",
                    self.to_hex()
                )
            }
            ComponentAddress::EddsaEd25519VirtualAccount(_) => {
                write!(f, "EddsaEd25519VirtualAccountComponent[{}]", self.to_hex())
            }
        }
        .map_err(|err| AddressError::FormatError(err))
    }
}

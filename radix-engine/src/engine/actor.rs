use crate::types::*;
use radix_engine_lib::engine::types::{
    NativeFunction, NativeMethod, PackageId, RENodeId, TransactionProcessorFunction,
};

/// Resolved receiver including info whether receiver was derefed
/// or not
#[derive(Debug, Copy, Clone, Eq, PartialEq, TypeId, Encode, Decode)]
pub struct ResolvedReceiver {
    pub derefed_from: Option<RENodeId>,
    pub receiver: RENodeId,
}

impl ResolvedReceiver {
    pub fn derefed(receiver: RENodeId, from: RENodeId) -> Self {
        Self {
            receiver,
            derefed_from: Some(from),
        }
    }

    pub fn new(receiver: RENodeId) -> Self {
        Self {
            receiver,
            derefed_from: None,
        }
    }
}

#[derive(Clone, Eq, PartialEq, TypeId, Encode, Decode)]
pub enum ResolvedFunction {
    Scrypto {
        package_address: PackageAddress,
        package_id: PackageId,
        blueprint_name: String,
        ident: String,
        export_name: String,
        return_type: Type,
    },
    Native(NativeFunction),
}

#[derive(Clone, Eq, PartialEq, TypeId, Encode, Decode)]
pub enum ResolvedMethod {
    Scrypto {
        package_address: PackageAddress,
        package_id: PackageId,
        blueprint_name: String,
        ident: String,
        export_name: String,
        return_type: Type,
    },
    Native(NativeMethod),
}

#[derive(Debug, Clone, Eq, PartialEq, TypeId, Encode, Decode)]
pub enum REActor {
    Function(ResolvedFunction),
    Method(ResolvedMethod, ResolvedReceiver),
}

impl REActor {
    pub fn is_scrypto_or_transaction(&self) -> bool {
        matches!(
            self,
            REActor::Method(ResolvedMethod::Scrypto { .. }, ..)
                | REActor::Function(ResolvedFunction::Scrypto { .. })
                | REActor::Function(ResolvedFunction::Native(
                    NativeFunction::TransactionProcessor(TransactionProcessorFunction::Run)
                ))
        )
    }
}

impl fmt::Debug for ResolvedFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scrypto {
                package_address,
                blueprint_name,
                ident,
                ..
            } => f
                .debug_struct("Scrypto")
                .field("package_address", package_address)
                .field("blueprint_name", blueprint_name)
                .field("ident", ident)
                .finish(),
            Self::Native(arg0) => f.debug_tuple("Native").field(arg0).finish(),
        }
    }
}

impl fmt::Debug for ResolvedMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scrypto {
                package_address,
                blueprint_name,
                ident,
                ..
            } => f
                .debug_struct("Scrypto")
                .field("package_address", package_address)
                .field("blueprint_name", blueprint_name)
                .field("ident", ident)
                .finish(),
            Self::Native(arg0) => f.debug_tuple("Native").field(arg0).finish(),
        }
    }
}

/// Execution mode
#[derive(Debug, Copy, Clone, Eq, PartialEq, TypeId, Encode, Decode)]
pub enum ExecutionMode {
    Kernel,
    Globalize,
    MoveDownstream,
    MoveUpstream,
    Deref,
    ScryptoInterpreter,
    AuthModule,
    Application,
    DropNode,
}

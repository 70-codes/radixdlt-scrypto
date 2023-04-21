use crate::types::*;
use radix_engine_common::types::*;
use radix_engine_derive::{ManifestSbor, ScryptoSbor};
use radix_engine_interface::api::LockFlags;
use sbor::rust::collections::*;
use sbor::rust::prelude::*;
use sbor::rust::vec::Vec;

#[repr(u8)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    ScryptoSbor,
    ManifestSbor,
    FromRepr,
    EnumIter,
)]
pub enum ObjectModuleId {
    SELF,
    Metadata,
    Royalty,
    AccessRules,
}

/// A high level interface to manipulate objects in the actor's call frame
pub trait ClientObjectApi<E> {
    // TODO: refine the interface
    /// Creates a new object of a given blueprint type
    fn new_object(
        &mut self,
        blueprint_ident: &str,
        object_states: Vec<Vec<u8>>,
    ) -> Result<NodeId, E>;

    fn lock_field(&mut self, field: u8, flags: LockFlags) -> Result<LockHandle, E>;

    // TODO: Add specific object read/write lock apis

    /// Get info regarding a visible object
    fn get_object_info(&mut self, node_id: &NodeId) -> Result<ObjectInfo, E>;

    /// Moves an object currently in the heap into the global space making
    /// it accessible to all. A global address is automatically created and returned.
    fn globalize(&mut self, modules: BTreeMap<ObjectModuleId, NodeId>) -> Result<GlobalAddress, E>;

    /// Moves an object currently in the heap into the global space making
    /// it accessible to all with the provided global address.
    fn globalize_with_address(
        &mut self,
        modules: BTreeMap<ObjectModuleId, NodeId>,
        address: GlobalAddress,
    ) -> Result<(), E>;

    /// Calls a method on an object
    fn call_method(
        &mut self,
        receiver: &NodeId,
        method_name: &str,
        args: Vec<u8>,
    ) -> Result<Vec<u8>, E>;

    // TODO: Add Object Module logic
    /// Calls a method on an object module
    fn call_module_method(
        &mut self,
        receiver: &NodeId,
        module_id: ObjectModuleId,
        method_name: &str,
        args: Vec<u8>,
    ) -> Result<Vec<u8>, E>;

    /// Drops an object
    fn drop_object(&mut self, node_id: NodeId) -> Result<(), E>;
}

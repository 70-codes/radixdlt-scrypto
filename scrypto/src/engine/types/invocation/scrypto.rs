use crate::engine::types::*;
use crate::values::*;

#[derive(Debug, Clone, Eq, PartialEq, TypeId, Encode, Decode)]
#[custom_type_id(ScryptoCustomTypeId)]
pub struct ScryptoFunctionIdent {
    pub package: ScryptoPackage,
    pub blueprint_name: String,
    pub function_name: String,
}

#[derive(Debug, Clone, Eq, PartialEq, TypeId, Encode, Decode)]
#[custom_type_id(ScryptoCustomTypeId)]
pub struct ScryptoMethodIdent {
    pub receiver: ScryptoReceiver,
    pub method_name: String,
}

#[derive(Debug, Clone, Eq, PartialEq, TypeId, Encode, Decode)]
#[custom_type_id(ScryptoCustomTypeId)]
pub enum ScryptoPackage {
    Global(PackageAddress),
    /* The following variant is commented out because all packages are globalized upon instantiation. */
    // Package(PackageId),
}

#[derive(Debug, Clone, Eq, PartialEq, TypeId, Encode, Decode)]
#[custom_type_id(ScryptoCustomTypeId)]
pub enum ScryptoReceiver {
    Global(ComponentAddress),
    Component(ComponentId),
}

use crate::types::*;

// TODO: clean up after `Owned(RENodeId)`?
#[derive(Debug, Clone, PartialEq, Eq, ScryptoCategorize, ScryptoEncode, ScryptoDecode)]
pub enum GlobalAddressSubstate {
    Component(ComponentId),
    Resource(ResourceManagerId),
    Package(PackageId),
    EpochManager(EpochManagerId),
    Validator(ValidatorId),
    Clock(ClockId),
    AccessController(AccessControllerId),
    Identity(IdentityId),
    Account(AccountId),
}

impl GlobalAddressSubstate {
    pub fn node_deref(&self) -> RENodeId {
        match self {
            GlobalAddressSubstate::Component(id) => RENodeId::Component(*id),
            GlobalAddressSubstate::Resource(id) => RENodeId::ResourceManager(*id),
            GlobalAddressSubstate::Package(id) => RENodeId::Package(*id),
            GlobalAddressSubstate::EpochManager(id) => RENodeId::EpochManager(*id),
            GlobalAddressSubstate::Identity(id) => RENodeId::Identity(*id),
            GlobalAddressSubstate::Validator(id) => RENodeId::Validator(*id),
            GlobalAddressSubstate::Clock(id) => RENodeId::Clock(*id),
            GlobalAddressSubstate::Account(id) => RENodeId::Account(*id),
            GlobalAddressSubstate::AccessController(id) => RENodeId::AccessController(*id),
        }
    }
}
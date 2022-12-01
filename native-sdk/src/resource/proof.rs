use radix_engine_interface::api::api::{EngineApi, SysNativeInvokable};
use radix_engine_interface::api::types::RENodeId;
use radix_engine_interface::data::{ScryptoDecode, ScryptoTypeId};
use radix_engine_interface::model::*;
use sbor::rust::fmt::Debug;

pub trait SysProof {
    fn sys_clone<Y, E: Debug + ScryptoTypeId + ScryptoDecode>(
        &self,
        sys_calls: &mut Y,
    ) -> Result<Proof, E>
    where
        Y: EngineApi<E> + SysNativeInvokable<ProofCloneInvocation, E>;
    fn sys_drop<Y, E: Debug + ScryptoTypeId + ScryptoDecode>(
        self,
        sys_calls: &mut Y,
    ) -> Result<(), E>
    where
        Y: EngineApi<E>;
}

impl SysProof for Proof {
    fn sys_clone<Y, E: Debug + ScryptoTypeId + ScryptoDecode>(
        &self,
        sys_calls: &mut Y,
    ) -> Result<Proof, E>
    where
        Y: EngineApi<E> + SysNativeInvokable<ProofCloneInvocation, E>,
    {
        sys_calls.sys_invoke(ProofCloneInvocation { receiver: self.0 })
    }

    fn sys_drop<Y, E: Debug + ScryptoTypeId + ScryptoDecode>(
        self,
        sys_calls: &mut Y,
    ) -> Result<(), E>
    where
        Y: EngineApi<E>,
    {
        sys_calls.sys_drop_node(RENodeId::Proof(self.0))
    }
}
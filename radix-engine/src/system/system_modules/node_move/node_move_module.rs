use crate::blueprints::resource::ProofInfoSubstate;
use crate::errors::{ModuleError, RuntimeError};
use crate::kernel::actor::Actor;
use crate::kernel::call_frame::CallFrameUpdate;
use crate::kernel::kernel_api::{KernelUpstreamApi, KernelUpstream};
use crate::system::module::SystemModule;
use crate::system::node_modules::type_info::{TypeInfoBlueprint, TypeInfoSubstate};
use crate::system::system_downstream::SystemDownstream;
use crate::system::system_upstream::SystemUpstream;
use crate::types::*;
use crate::vm::wasm::WasmEngine;
use radix_engine_interface::api::ClientObjectApi;
use radix_engine_interface::api::LockFlags;
use radix_engine_interface::blueprints::resource::*;
use radix_engine_interface::*;

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub enum NodeMoveError {
    CantMoveDownstream(NodeId),
    CantMoveUpstream(NodeId),
}

#[derive(Debug, Clone)]
pub struct NodeMoveModule {}

impl NodeMoveModule {
    fn prepare_move_downstream<
        'g,
        Y: KernelUpstreamApi<SystemUpstream<'g, W>>,
        W: WasmEngine + 'g,
    >(
        node_id: NodeId,
        callee: &Actor,
        api: &mut Y,
    ) -> Result<(), RuntimeError> {
        let (package_address, blueprint_name) = {
            let mut system = SystemDownstream::new(api);
            let blueprint = system.get_object_info(&node_id)?.blueprint;
            (blueprint.package_address, blueprint.blueprint_name)
        };
        match (package_address, blueprint_name.as_str()) {
            (RESOURCE_MANAGER_PACKAGE, PROOF_BLUEPRINT) => {
                if matches!(callee, Actor::Function { .. })
                    && callee.package_address().eq(&RESOURCE_MANAGER_PACKAGE)
                {
                    return Ok(());
                }

                // Change to restricted unless it's moved to auth zone.
                // TODO: align with barrier design?
                let mut changed_to_restricted = true;
                if let Actor::Method { node_id, .. } = callee {
                    let type_info = TypeInfoBlueprint::get_type(node_id, api)?;
                    if let TypeInfoSubstate::Object(ObjectInfo { blueprint, .. }) = type_info {
                        if blueprint.eq(&Blueprint::new(
                            &RESOURCE_MANAGER_PACKAGE,
                            AUTH_ZONE_BLUEPRINT,
                        )) {
                            changed_to_restricted = false;
                        }
                    }
                }

                let handle = api.kernel_lock_substate(
                    &node_id,
                    SysModuleId::ObjectTuple,
                    &ProofOffset::Info.into(),
                    LockFlags::MUTABLE,
                )?;
                let mut proof: ProofInfoSubstate =
                    api.kernel_read_substate(handle)?.as_typed().unwrap();

                if proof.restricted {
                    return Err(RuntimeError::ModuleError(ModuleError::NodeMoveError(
                        NodeMoveError::CantMoveDownstream(node_id),
                    )));
                }

                if changed_to_restricted {
                    proof.change_to_restricted();
                }

                api.kernel_write_substate(handle, IndexedScryptoValue::from_typed(&proof))?;
                api.kernel_drop_lock(handle)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn prepare_move_upstream<Y: KernelUpstreamApi<M>, M: KernelUpstream>(
        _node_id: NodeId,
        _api: &mut Y,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }
}

impl<'g, W: WasmEngine + 'g> SystemModule<SystemUpstream<'g, W>> for NodeMoveModule {
    fn before_push_frame<Y: KernelUpstreamApi<SystemUpstream<'g, W>>>(
        api: &mut Y,
        callee: &Actor,
        call_frame_update: &mut CallFrameUpdate,
        _args: &IndexedScryptoValue,
    ) -> Result<(), RuntimeError> {
        for node_id in &call_frame_update.nodes_to_move {
            // TODO: Move into system layer
            Self::prepare_move_downstream(*node_id, callee, api)?;
        }

        Ok(())
    }

    fn on_execution_finish<Y: KernelUpstreamApi<SystemUpstream<'g, W>>>(
        api: &mut Y,
        _caller: &Option<Actor>,
        call_frame_update: &CallFrameUpdate,
    ) -> Result<(), RuntimeError> {
        for node_id in &call_frame_update.nodes_to_move {
            Self::prepare_move_upstream(*node_id, api)?;
        }

        Ok(())
    }
}

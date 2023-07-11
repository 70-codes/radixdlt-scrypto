use crate::blueprints::resource::{LocalRef, ProofError, ProofMoveableSubstate};
use crate::errors::RuntimeError;
use crate::kernel::kernel_api::KernelSubstateApi;
use crate::system::system::FieldSubstate;
use crate::system::system_callback::SystemLockData;
use crate::types::*;
use radix_engine_interface::api::field_api::LockFlags;
use radix_engine_interface::api::{ClientApi, FieldValue, OBJECT_HANDLE_SELF};
use radix_engine_interface::blueprints::resource::*;

#[derive(Debug, Clone, ScryptoSbor)]
pub struct FungibleProofSubstate {
    pub total_locked: Decimal,
    /// The supporting containers.
    pub evidence: BTreeMap<LocalRef, Decimal>,
}

impl FungibleProofSubstate {
    pub fn new(
        total_locked: Decimal,
        evidence: BTreeMap<LocalRef, Decimal>,
    ) -> Result<FungibleProofSubstate, ProofError> {
        if total_locked.is_zero() {
            return Err(ProofError::EmptyProofNotAllowed);
        }

        Ok(Self {
            total_locked,
            evidence,
        })
    }

    pub fn clone_proof<Y: ClientApi<RuntimeError>>(
        &self,
        api: &mut Y,
    ) -> Result<Self, RuntimeError> {
        for (container, locked_amount) in &self.evidence {
            api.call_method(
                container.as_node_id(),
                match container {
                    LocalRef::Bucket(_) => FUNGIBLE_BUCKET_LOCK_AMOUNT_IDENT,
                    LocalRef::Vault(_) => FUNGIBLE_VAULT_LOCK_FUNGIBLE_AMOUNT_IDENT,
                },
                scrypto_args!(locked_amount),
            )?;
        }
        Ok(Self {
            total_locked: self.total_locked.clone(),
            evidence: self.evidence.clone(),
        })
    }

    pub fn teardown<Y: ClientApi<RuntimeError>>(self, api: &mut Y) -> Result<(), RuntimeError> {
        for (container, locked_amount) in &self.evidence {
            api.call_method(
                container.as_node_id(),
                match container {
                    LocalRef::Bucket(_) => FUNGIBLE_BUCKET_UNLOCK_AMOUNT_IDENT,
                    LocalRef::Vault(_) => FUNGIBLE_VAULT_UNLOCK_FUNGIBLE_AMOUNT_IDENT,
                },
                scrypto_args!(locked_amount),
            )?;
        }
        Ok(())
    }

    pub fn amount(&self) -> Decimal {
        self.total_locked
    }
}

pub struct FungibleProofBlueprint;

impl FungibleProofBlueprint {
    pub(crate) fn clone<Y>(api: &mut Y) -> Result<Proof, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let moveable = {
            let handle = api.actor_open_field(
                OBJECT_HANDLE_SELF,
                FungibleProofField::Moveable.into(),
                LockFlags::read_only(),
            )?;
            let substate_ref: ProofMoveableSubstate = api.field_read_typed(handle)?;
            let moveable = substate_ref.clone();
            api.field_close(handle)?;
            moveable
        };

        let handle = api.actor_open_field(
            OBJECT_HANDLE_SELF,
            FungibleProofField::ProofRefs.into(),
            LockFlags::read_only(),
        )?;
        let substate_ref: FungibleProofSubstate = api.field_read_typed(handle)?;
        let proof = substate_ref.clone();
        let clone = proof.clone_proof(api)?;

        let proof_id = api.new_simple_object(
            FUNGIBLE_PROOF_BLUEPRINT,
            vec![FieldValue::new(&moveable), FieldValue::new(&clone)],
        )?;

        // Drop after object creation to keep the reference alive
        api.field_close(handle)?;

        Ok(Proof(Own(proof_id)))
    }

    pub(crate) fn get_amount<Y>(api: &mut Y) -> Result<Decimal, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let handle = api.actor_open_field(
            OBJECT_HANDLE_SELF,
            FungibleProofField::ProofRefs.into(),
            LockFlags::read_only(),
        )?;
        let substate_ref: FungibleProofSubstate = api.field_read_typed(handle)?;
        let amount = substate_ref.amount();
        api.field_close(handle)?;
        Ok(amount)
    }

    pub(crate) fn get_resource_address<Y>(api: &mut Y) -> Result<ResourceAddress, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let address =
            ResourceAddress::new_or_panic(api.actor_get_object_info()?.get_outer_object().into());
        Ok(address)
    }

    pub(crate) fn drop<Y>(proof: Proof, api: &mut Y) -> Result<(), RuntimeError>
    where
        Y: KernelSubstateApi<SystemLockData> + ClientApi<RuntimeError>,
    {
        api.drop_object(proof.0.as_node_id())?;

        Ok(())
    }

    pub(crate) fn on_drop<Y>(proof: Proof, api: &mut Y) -> Result<(), RuntimeError>
    where
        Y: KernelSubstateApi<SystemLockData> + ClientApi<RuntimeError>,
    {
        let handle = api.kernel_open_substate(
            proof.0.as_node_id(),
            MAIN_BASE_PARTITION,
            &NonFungibleProofField::ProofRefs.into(),
            LockFlags::read_only(),
            SystemLockData::Default,
        )?;
        let proof_substate: FieldSubstate<FungibleProofSubstate> =
            api.kernel_read_substate(handle)?.as_typed().unwrap();
        proof_substate.value.0.teardown(api)?;
        api.kernel_close_substate(handle)?;

        Ok(())
    }
}

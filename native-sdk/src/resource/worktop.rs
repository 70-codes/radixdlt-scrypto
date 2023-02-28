use radix_engine_interface::api::types::*;
use radix_engine_interface::api::ClientApi;
use radix_engine_interface::blueprints::resource::*;
use radix_engine_interface::constants::RESOURCE_MANAGER_PACKAGE;
use radix_engine_interface::data::{
    scrypto_decode, scrypto_encode, ScryptoCategorize, ScryptoDecode,
};
use radix_engine_interface::math::Decimal;
use sbor::rust::collections::BTreeSet;
use sbor::rust::fmt::Debug;
use sbor::rust::vec::Vec;

pub struct Worktop;

impl Worktop {
    pub fn sys_drop<Y, E: Debug + ScryptoCategorize + ScryptoDecode>(api: &mut Y) -> Result<(), E>
    where
        Y: ClientApi<E>,
    {
        let _rtn = api.call_function(
            RESOURCE_MANAGER_PACKAGE,
            WORKTOP_BLUEPRINT,
            WORKTOP_DROP_IDENT,
            scrypto_encode(&WorktopDropInput {}).unwrap(),
        )?;

        Ok(())
    }

    pub fn sys_put<Y, E: Debug + ScryptoCategorize + ScryptoDecode>(
        bucket: Bucket,
        api: &mut Y,
    ) -> Result<(), E>
    where
        Y: ClientApi<E>,
    {
        let _rtn = api.call_method(
            RENodeId::Worktop,
            WORKTOP_PUT_IDENT,
            scrypto_encode(&WorktopPutInput { bucket }).unwrap(),
        )?;

        Ok(())
    }

    pub fn sys_take<Y, E: Debug + ScryptoCategorize + ScryptoDecode>(
        resource_address: ResourceAddress,
        amount: Decimal,
        api: &mut Y,
    ) -> Result<Bucket, E>
    where
        Y: ClientApi<E>,
    {
        let rtn = api.call_method(
            RENodeId::Worktop,
            WORKTOP_TAKE_IDENT,
            scrypto_encode(&WorktopTakeInput {
                resource_address,
                amount,
            })
            .unwrap(),
        )?;

        Ok(scrypto_decode(&rtn).unwrap())
    }

    pub fn sys_take_non_fungibles<Y, E: Debug + ScryptoCategorize + ScryptoDecode>(
        resource_address: ResourceAddress,
        ids: BTreeSet<NonFungibleLocalId>,
        api: &mut Y,
    ) -> Result<Bucket, E>
    where
        Y: ClientApi<E>,
    {
        let rtn = api.call_method(
            RENodeId::Worktop,
            WORKTOP_TAKE_NON_FUNGIBLES_IDENT,
            scrypto_encode(&WorktopTakeNonFungiblesInput {
                resource_address,
                ids,
            })
            .unwrap(),
        )?;

        Ok(scrypto_decode(&rtn).unwrap())
    }

    pub fn sys_take_all<Y, E: Debug + ScryptoCategorize + ScryptoDecode>(
        resource_address: ResourceAddress,
        api: &mut Y,
    ) -> Result<Bucket, E>
    where
        Y: ClientApi<E>,
    {
        let rtn = api.call_method(
            RENodeId::Worktop,
            WORKTOP_TAKE_ALL_IDENT,
            scrypto_encode(&WorktopTakeAllInput { resource_address }).unwrap(),
        )?;
        Ok(scrypto_decode(&rtn).unwrap())
    }

    pub fn sys_assert_contains<Y, E: Debug + ScryptoCategorize + ScryptoDecode>(
        resource_address: ResourceAddress,
        api: &mut Y,
    ) -> Result<(), E>
    where
        Y: ClientApi<E>,
    {
        let _rtn = api.call_method(
            RENodeId::Worktop,
            WORKTOP_ASSERT_CONTAINS_IDENT,
            scrypto_encode(&WorktopAssertContainsInput { resource_address }).unwrap(),
        )?;
        Ok(())
    }

    pub fn sys_assert_contains_amount<Y, E: Debug + ScryptoCategorize + ScryptoDecode>(
        resource_address: ResourceAddress,
        amount: Decimal,
        api: &mut Y,
    ) -> Result<(), E>
    where
        Y: ClientApi<E>,
    {
        let _rtn = api.call_method(
            RENodeId::Worktop,
            WORKTOP_ASSERT_CONTAINS_AMOUNT_IDENT,
            scrypto_encode(&WorktopAssertContainsAmountInput {
                resource_address,
                amount,
            })
            .unwrap(),
        )?;
        Ok(())
    }

    pub fn sys_assert_contains_non_fungibles<Y, E: Debug + ScryptoCategorize + ScryptoDecode>(
        resource_address: ResourceAddress,
        ids: BTreeSet<NonFungibleLocalId>,
        api: &mut Y,
    ) -> Result<(), E>
    where
        Y: ClientApi<E>,
    {
        let _rtn = api.call_method(
            RENodeId::Worktop,
            WORKTOP_ASSERT_CONTAINS_NON_FUNGIBLES_IDENT,
            scrypto_encode(&WorktopAssertContainsNonFungiblesInput {
                resource_address,
                ids,
            })
            .unwrap(),
        )?;
        Ok(())
    }

    pub fn sys_drain<Y, E: Debug + ScryptoCategorize + ScryptoDecode>(
        api: &mut Y,
    ) -> Result<Vec<Bucket>, E>
    where
        Y: ClientApi<E>,
    {
        let rtn = api.call_method(
            RENodeId::Worktop,
            WORKTOP_DRAIN_IDENT,
            scrypto_encode(&WorktopDrainInput {}).unwrap(),
        )?;
        Ok(scrypto_decode(&rtn).unwrap())
    }
}

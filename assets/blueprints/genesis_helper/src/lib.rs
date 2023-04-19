use scrypto::api::node_modules::metadata::MetadataValue;
use scrypto::blueprints::account::AccountDepositInput;
use scrypto::blueprints::epoch_manager::*;
use scrypto::prelude::*;

// Important: the types defined here must match those in bootstrap.rs
#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct GenesisValidator {
    pub key: EcdsaSecp256k1PublicKey,
    pub accept_delegated_stake: bool,
    pub is_registered: bool,
    pub metadata: Vec<(String, String)>,
    pub owner: ComponentAddress,
}

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct GenesisStakeAllocation {
    pub account_index: u32,
    pub xrd_amount: Decimal,
}

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct GenesisResource {
    pub address_seed_bytes: [u8; 26],
    pub initial_supply: Decimal,
    pub metadata: Vec<(String, String)>,
    pub owner: Option<ComponentAddress>,
}

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct GenesisResourceAllocation {
    pub account_index: u32,
    pub amount: Decimal,
}

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub enum GenesisDataChunk {
    Validators(Vec<GenesisValidator>),
    Stakes {
        accounts: Vec<ComponentAddress>,
        allocations: BTreeMap<EcdsaSecp256k1PublicKey, Vec<GenesisStakeAllocation>>,
    },
    Resources(Vec<GenesisResource>),
    ResourceBalances {
        accounts: Vec<ComponentAddress>,
        allocations: BTreeMap<ResourceAddress, Vec<GenesisResourceAllocation>>,
    },
    XrdBalances(BTreeMap<ComponentAddress, Decimal>),
}

#[blueprint]
mod genesis_helper {
    struct GenesisHelper {
        epoch_manager: ComponentAddress,
        rounds_per_epoch: u64,
        xrd_vault: Vault,
        resource_vaults: KeyValueStore<ResourceAddress, Vault>,
        validators: KeyValueStore<EcdsaSecp256k1PublicKey, ComponentAddress>,
    }

    impl GenesisHelper {
        pub fn new(
            whole_lotta_xrd: Bucket,
            epoch_manager: ComponentAddress,
            rounds_per_epoch: u64,
        ) -> ComponentAddress {
            Self {
                epoch_manager,
                rounds_per_epoch,
                xrd_vault: Vault::with_bucket(whole_lotta_xrd),
                resource_vaults: KeyValueStore::new(),
                validators: KeyValueStore::new(),
            }
            .instantiate()
            .globalize()
        }

        pub fn ingest_data_chunk(&mut self, chunk: GenesisDataChunk) {
            match chunk {
                GenesisDataChunk::Validators(validators) => self.create_validators(validators),
                GenesisDataChunk::Stakes {
                    accounts,
                    allocations,
                } => self.allocate_stakes(accounts, allocations),
                GenesisDataChunk::Resources(resources) => self.create_resources(resources),
                GenesisDataChunk::ResourceBalances {
                    accounts,
                    allocations,
                } => self.allocate_resources(accounts, allocations),
                GenesisDataChunk::XrdBalances(allocations) => self.allocate_xrd(allocations),
            }
        }

        fn create_validators(&mut self, validators: Vec<GenesisValidator>) {
            for validator in validators.into_iter() {
                self.create_validator(validator);
            }
        }

        fn create_validator(&mut self, validator: GenesisValidator) {
            let (validator_address, owner_token_bucket): (ComponentAddress, Bucket) =
                Runtime::call_method(
                    self.epoch_manager,
                    "create_validator",
                    scrypto_encode(&EpochManagerCreateValidatorInput { key: validator.key })
                        .unwrap(),
                );

            // Deposit the badge to the owner account
            let _: () = Runtime::call_method(
                validator.owner,
                "deposit",
                scrypto_encode(&AccountDepositInput {
                    bucket: owner_token_bucket,
                })
                .unwrap(),
            );

            if validator.is_registered {
                let _: () = Runtime::call_method(
                    validator_address,
                    "register",
                    scrypto_encode(&ValidatorRegisterInput {}).unwrap(),
                );
            }

            let _: () = Runtime::call_method(
                validator_address,
                "update_accept_delegated_stake",
                scrypto_encode(&ValidatorUpdateAcceptDelegatedStakeInput {
                    accept_delegated_stake: validator.accept_delegated_stake,
                })
                .unwrap(),
            );

            self.validators.insert(validator.key, validator_address);
        }

        fn allocate_stakes(
            &mut self,
            accounts: Vec<ComponentAddress>,
            allocations: BTreeMap<EcdsaSecp256k1PublicKey, Vec<GenesisStakeAllocation>>,
        ) {
            for (validator_key, stake_allocations) in allocations.into_iter() {
                let validator_address = self.validators.get(&validator_key).unwrap();
                for GenesisStakeAllocation {
                    account_index,
                    xrd_amount,
                } in stake_allocations.into_iter()
                {
                    let staker_account_address = accounts[account_index as usize].clone();
                    let stake_bucket = self.xrd_vault.take(xrd_amount);
                    let lp_bucket: Bucket = Runtime::call_method(
                        validator_address.clone(),
                        "stake",
                        scrypto_encode(&ValidatorStakeInput {
                            stake: stake_bucket,
                        })
                        .unwrap(),
                    );
                    let _: () = Runtime::call_method(
                        staker_account_address,
                        "deposit",
                        scrypto_encode(&AccountDepositInput { bucket: lp_bucket }).unwrap(),
                    );
                }
            }
        }

        fn create_resources(&mut self, resources: Vec<GenesisResource>) {
            for resource in resources {
                let (resource_address, initial_supply_bucket) = Self::create_resource(resource);
                self.resource_vaults
                    .insert(resource_address, Vault::with_bucket(initial_supply_bucket));
            }
        }

        fn create_resource(resource: GenesisResource) -> (ResourceAddress, Bucket) {
            let metadata: BTreeMap<String, String> = resource.metadata.into_iter().collect();

            let address_bytes = NodeId::new(
                EntityType::GlobalFungibleResource as u8,
                &resource.address_seed_bytes,
            )
            .0;
            let resource_address = ResourceAddress::new_unchecked(address_bytes.clone());
            let mut access_rules = BTreeMap::new();
            access_rules.insert(Deposit, (rule!(allow_all), rule!(deny_all)));
            access_rules.insert(Withdraw, (rule!(allow_all), rule!(deny_all)));

            if let Some(owner) = resource.owner {
                // TODO: Should we use securify style non fungible resource for the owner badge?
                let owner_badge = ResourceBuilder::new_fungible()
                    .divisibility(DIVISIBILITY_NONE)
                    .metadata(
                        "name",
                        format!("Resource Owner Badge ({})", metadata.get("symbol").unwrap()),
                    )
                    .mint_initial_supply(1);

                borrow_resource_manager!(owner_badge.resource_address())
                    .metadata()
                    .set_list("tags", vec![MetadataValue::String("badge".to_string())]);

                access_rules.insert(
                    Mint,
                    (
                        rule!(require(owner_badge.resource_address())),
                        rule!(deny_all),
                    ),
                );
                access_rules.insert(
                    Burn,
                    (
                        rule!(require(owner_badge.resource_address())),
                        rule!(deny_all),
                    ),
                );
                access_rules.insert(
                    UpdateMetadata,
                    (
                        rule!(require(owner_badge.resource_address())),
                        rule!(deny_all),
                    ),
                );

                let _: () = Runtime::call_method(
                    owner,
                    "deposit",
                    scrypto_encode(&AccountDepositInput {
                        bucket: owner_badge,
                    })
                    .unwrap(),
                );
            }

            let (_, initial_supply_bucket): (ResourceAddress, Bucket) = Runtime::call_function(
                RESOURCE_MANAGER_PACKAGE,
                FUNGIBLE_RESOURCE_MANAGER_BLUEPRINT,
                FUNGIBLE_RESOURCE_MANAGER_CREATE_WITH_INITIAL_SUPPLY_AND_ADDRESS_IDENT,
                scrypto_encode(
                    &FungibleResourceManagerCreateWithInitialSupplyAndAddressInput {
                        divisibility: 18,
                        metadata,
                        access_rules,
                        initial_supply: resource.initial_supply,
                        resource_address: address_bytes,
                    },
                )
                .unwrap(),
            );

            (resource_address, initial_supply_bucket)
        }

        fn allocate_resources(
            &mut self,
            accounts: Vec<ComponentAddress>,
            allocations: BTreeMap<ResourceAddress, Vec<GenesisResourceAllocation>>,
        ) {
            for (resource_address, allocations) in allocations.into_iter() {
                let mut resource_vault = self.resource_vaults.get_mut(&resource_address).unwrap();
                for GenesisResourceAllocation {
                    account_index,
                    amount,
                } in allocations.into_iter()
                {
                    let account_address = accounts[account_index as usize].clone();
                    let allocation_bucket = resource_vault.take(amount);
                    let _: () = Runtime::call_method(
                        account_address,
                        "deposit",
                        scrypto_encode(&AccountDepositInput {
                            bucket: allocation_bucket,
                        })
                        .unwrap(),
                    );
                }
            }
        }

        fn allocate_xrd(&mut self, allocations: BTreeMap<ComponentAddress, Decimal>) {
            for (account_address, amount) in allocations.into_iter() {
                let bucket = self.xrd_vault.take(amount);
                let _: () = Runtime::call_method(
                    account_address,
                    "deposit",
                    scrypto_encode(&AccountDepositInput { bucket }).unwrap(),
                );
            }
        }

        pub fn wrap_up(&mut self) -> Bucket {
            // A little hack to move to the next epoch,
            // which also updates the validator set
            // TODO: clean this up (add a dedicated method in epoch manager?)
            let _: () = Runtime::call_method(
                self.epoch_manager,
                "next_round",
                scrypto_encode(&EpochManagerNextRoundInput {
                    round: self.rounds_per_epoch,
                })
                .unwrap(),
            );

            // TODO: assert all resource vaults are empty
            // i.e. that for all resources: initial_supply == sum(allocations)

            // return any unused XRD
            self.xrd_vault.take_all()
        }
    }
}

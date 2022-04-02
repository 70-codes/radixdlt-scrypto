use scrypto::prelude::*;

blueprint! {
    struct NonExistentVault {
        vault: Option<Vault>,
        vaults: LazyMap<u128, Vault>,
    }

    impl NonExistentVault {
        pub fn create_component_with_non_existent_vault() -> ComponentId {
            NonExistentVault {
                vault: Option::Some(Vault((Transaction::transaction_hash(), 1025))),
                vaults: LazyMap::new(),
            }
            .instantiate()
            .auth("create_non_existent_vault", auth!(allow_all))
            .auth("create_lazy_map_with_non_existent_vault", auth!(allow_all))
            .auth("create_non_existent_vault_in_lazy_map", auth!(allow_all))
            .globalize()
        }

        pub fn new() -> ComponentId {
            NonExistentVault {
                vault: Option::None,
                vaults: LazyMap::new(),
            }
            .instantiate()
            .auth("create_non_existent_vault", auth!(allow_all))
            .auth("create_lazy_map_with_non_existent_vault", auth!(allow_all))
            .auth("create_non_existent_vault_in_lazy_map", auth!(allow_all))
            .globalize()
        }

        pub fn create_non_existent_vault(&mut self) {
            self.vault = Option::Some(Vault((Transaction::transaction_hash(), 1025)))
        }

        pub fn create_lazy_map_with_non_existent_vault() -> ComponentId {
            let vaults = LazyMap::new();
            vaults.insert(0, Vault((Transaction::transaction_hash(), 1025)));
            NonExistentVault {
                vault: Option::None,
                vaults,
            }
            .instantiate()
            .globalize()
        }

        pub fn create_non_existent_vault_in_lazy_map(&mut self) {
            self.vaults
                .insert(0, Vault((Transaction::transaction_hash(), 1025)));
        }
    }
}

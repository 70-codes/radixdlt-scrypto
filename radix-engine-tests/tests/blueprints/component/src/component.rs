use scrypto::prelude::*;

#[blueprint]
mod component_test {
    struct ComponentTest {
        test_vault: Vault,
        secret: String,
    }

    impl ComponentTest {
        fn create_test_token(amount: u32) -> Bucket {
            ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_MAXIMUM)
                .metadata("name", "TestToken")
                .mint_initial_supply(amount)
        }

        pub fn create_component() -> ComponentAddress {
            Self {
                test_vault: Vault::with_bucket(Self::create_test_token(1000)),
                secret: "Secret".to_owned(),
            }
            .instantiate()
            .globalize()
        }

        pub fn get_component_info(component_address: ComponentAddress) -> (PackageAddress, String) {
            let component: Global<AnyComponent> = component_address.into();
            (
                component.package_address(),
                component.blueprint_name(),
            )
        }

        pub fn get_component_state(&self) -> String {
            self.secret.clone()
        }

        pub fn put_component_state(&mut self) -> Bucket {
            // Take resource from vault
            let bucket = self.test_vault.take(1);

            // Update state
            self.secret = "New secret".to_owned();

            bucket
        }
    }
}

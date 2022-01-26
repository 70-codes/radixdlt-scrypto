use scrypto::prelude::*;

blueprint! {
    struct ContextTest;

    impl ContextTest {
        pub fn query() -> (Address, H256, u64, u128) {
            (
                Context::package_address(),
                Context::transaction_hash(),
                Context::current_epoch(),
                Uuid::generate(),
            )
        }
    }
}

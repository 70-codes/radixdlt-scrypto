use scrypto::prelude::*;

// TODO: Change this to be a stub
#[blueprint]
mod preallocated {
    struct Preallocated {
        secret: String,
    }

    impl Preallocated {
        pub fn new(preallocated_address_bytes: [u8; 30], secret: String) -> Global<Preallocated> {
            Self { secret }
                .instantiate()
                .globalize_at_address(ComponentAddress::new_or_panic(preallocated_address_bytes))
        }

        pub fn get_secret(&self) -> String {
            self.secret.clone()
        }
    }
}

#[blueprint]
mod preallocated_call {
    use super::preallocated::*;

    const PREALLOCATED: ComponentAddress = ComponentAddress::new_or_panic([
        192, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1,
    ]);

    struct PreallocatedCall {}

    impl PreallocatedCall {
        pub fn call_preallocated() -> String {
            let preallocated: Global<Preallocated> = PREALLOCATED.into();
            preallocated.get_secret()
        }
    }
}

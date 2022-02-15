use scrypto::prelude::*;

import! {
r#"
{
    "package": "01bda8686d6c2fa45dce04fac71a09b54efbc8028c23aac74bc00e",
    "name": "Airdrop",
    "functions": [
        {
            "name": "new",
            "inputs": [],
            "output": {
                "type": "Custom",
                "name": "ComponentRef",
                "generics": []
            }
        }
    ],
    "methods": [
        {
            "name": "free_token",
            "mutability": "Immutable",
            "inputs": [],
            "output": {
                "type": "Custom",
                "name": "Bucket",
                "generics": []
            }
        }
    ]
}
"#
}

blueprint! {
    struct Proxy1 {
        airdrop: Airdrop,
    }

    impl Proxy1 {
        pub fn new() -> ComponentRef {
            Self {
                // The new() function returns a generic Component. We use `.into()` to convert it into an `Airdrop`.
                airdrop: Airdrop::new().into(),
            }
            .instantiate()
        }

        pub fn free_token(&self) -> Bucket {
            // Calling a method on a component using `.method_name()`.
            self.airdrop.free_token()
        }
    }
}

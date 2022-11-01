use sbor::Decode;

use super::api::RadixEngineInput;
use crate::values::*;

/// Utility function for making a radix engine call.
#[cfg(target_arch = "wasm32")]
pub fn call_engine<V: Decode<ScryptoCustomTypeId>>(input: RadixEngineInput) -> V {
    use crate::buffer::{scrypto_decode_from_buffer, *};
    use crate::engine::api::radix_engine;

    unsafe {
        let input_ptr = scrypto_encode_to_buffer(&input);
        let output_ptr = radix_engine(input_ptr);
        scrypto_decode_from_buffer::<V>(output_ptr).unwrap()
    }
}

/// Utility function for making a radix engine call.
#[cfg(not(target_arch = "wasm32"))]
pub fn call_engine<V: Decode<ScryptoCustomTypeId>>(_input: RadixEngineInput) -> V {
    todo!()
}

#[macro_export]
macro_rules! native_methods {
    ($receiver:expr, $type_ident:expr => { $($vis:vis $fn:ident $method_name:ident $s:tt -> $rtn:ty { $fn_ident:expr, $arg:expr })* } ) => {
        $(
            $vis $fn $method_name $s -> $rtn {
                let input = RadixEngineInput::InvokeNativeMethod(
                    $type_ident($fn_ident),
                    $receiver,
                    scrypto::buffer::scrypto_encode(&$arg)
                );
                call_engine(input)
            }
        )+
    };
}

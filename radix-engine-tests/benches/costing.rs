use criterion::{criterion_group, criterion_main, Criterion};
use radix_engine::{
    system::system_modules::costing::SystemLoanFeeReserve,
    types::*,
    utils::ExtractSchemaError,
    vm::{
        wasm::{
            DefaultWasmEngine, InstrumentedCode, WasmEngine, WasmInstance, WasmRuntime,
            WasmValidator,
        },
        wasm_runtime::NoOpWasmRuntime,
    },
};
use sbor::rust::iter;
use sbor::rust::sync::Arc;
use transaction::{
    prelude::Secp256k1PrivateKey,
    validation::{recover_secp256k1, verify_secp256k1},
};
use wabt::wat2wasm;

fn bench_decode_sbor(c: &mut Criterion) {
    let payload = include_bytes!("../../assets/radiswap.schema");
    println!("Payload size: {}", payload.len());
    c.bench_function("costing::decode_sbor", |b| {
        b.iter(|| manifest_decode::<ManifestValue>(payload))
    });
}

fn bench_validate_secp256k1(c: &mut Criterion) {
    let message = "m".repeat(1_000_000);
    let message_hash = hash(message.as_bytes());
    let signer = Secp256k1PrivateKey::from_u64(123123123123).unwrap();
    let signature = signer.sign(&message_hash);

    c.bench_function("costing::validate_secp256k1", |b| {
        b.iter(|| {
            let public_key = recover_secp256k1(&message_hash, &signature).unwrap();
            verify_secp256k1(&message_hash, &public_key, &signature);
        })
    });
}

fn bench_spin_loop(c: &mut Criterion) {
    // Prepare code
    let code = wat2wasm(&include_str!("../tests/wasm/loop.wat").replace("${n}", "1000")).unwrap();

    // Instrument
    let validator = WasmValidator::default();
    let instrumented_code = InstrumentedCode {
        metered_code_key: (
            PackageAddress::new_or_panic([EntityType::GlobalPackage as u8; NodeId::LENGTH]),
            validator.metering_config,
        ),
        code: Arc::new(
            validator
                .validate(&code, iter::empty())
                .map_err(|e| ExtractSchemaError::InvalidWasm(e))
                .unwrap()
                .0,
        ),
    };

    let mut gas_consumed = 0u32;
    c.bench_function("costing::spin_loop", |b| {
        b.iter(|| {
            let wasm_engine = DefaultWasmEngine::default();
            let fee_reserve = SystemLoanFeeReserve::default()
                .with_free_credit(Decimal::try_from(DEFAULT_FREE_CREDIT_IN_XRD).unwrap());
            gas_consumed = 0;
            let mut runtime: Box<dyn WasmRuntime> =
                Box::new(NoOpWasmRuntime::new(fee_reserve, &mut gas_consumed));
            let mut instance = wasm_engine.instantiate(&instrumented_code);
            instance
                .invoke_export("Test_f", vec![Buffer(0)], &mut runtime)
                .unwrap();
        })
    });

    println!("Gas consumed: {}", gas_consumed);
}

criterion_group!(
    costing,
    bench_decode_sbor,
    bench_validate_secp256k1,
    bench_spin_loop
);
criterion_main!(costing);
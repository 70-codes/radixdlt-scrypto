use radix_engine::blueprints::package::PackageError;
use radix_engine::errors::{ApplicationError, RuntimeError, SystemModuleError, VmError};
use radix_engine::system::system_modules::auth::AuthError;
use radix_engine::types::*;
use radix_engine::vm::wasm::PrepareError;
use radix_engine::vm::wasm::*;
use radix_engine_interface::blueprints::package::{
    AuthConfig, BlueprintDefinitionInit, BlueprintType, PackageDefinition,
    PackagePublishNativeManifestInput, PACKAGE_BLUEPRINT,
};
use radix_engine_interface::metadata_init;
use radix_engine_interface::schema::{
    BlueprintEventSchemaInit, BlueprintFunctionsSchemaInit, BlueprintSchemaInit,
    BlueprintStateSchemaInit, FieldSchema, FunctionSchemaInit, TypeRef,
};
use sbor::basic_well_known_types::{ANY_TYPE, UNIT_TYPE};
use scrypto_unit::*;
use transaction::prelude::*;

#[test]
fn missing_memory_should_cause_error() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();

    // Act
    let code = wat2wasm(
        r#"
            (module
                (func (export "test") (result i32)
                    i32.const 1337
                )
            )
            "#,
    );
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .publish_package_advanced(
            None,
            code,
            PackageDefinition::default(),
            BTreeMap::new(),
            OwnerRole::None,
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            &RuntimeError::ApplicationError(ApplicationError::PackageError(
                PackageError::InvalidWasm(PrepareError::InvalidMemory(
                    InvalidMemory::MissingMemorySection
                ))
            ))
        )
    });
}

#[test]
fn large_return_len_should_cause_memory_access_error() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let package = test_runner.compile_and_publish("./tests/blueprints/package");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(package, "LargeReturnSize", "f", manifest_args!())
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| {
        if let RuntimeError::VmError(VmError::Wasm(b)) = e {
            matches!(*b, WasmRuntimeError::MemoryAccessError)
        } else {
            false
        }
    });
}

#[test]
fn overflow_return_len_should_cause_memory_access_error() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let package = test_runner.compile_and_publish("./tests/blueprints/package");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(package, "MaxReturnSize", "f", manifest_args!())
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| {
        if let RuntimeError::VmError(VmError::Wasm(b)) = e {
            matches!(*b, WasmRuntimeError::MemoryAccessError)
        } else {
            false
        }
    });
}

#[test]
fn zero_return_len_should_cause_data_validation_error() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let package = test_runner.compile_and_publish("./tests/blueprints/package");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(package, "ZeroReturnSize", "f", manifest_args!())
        .build();

    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| matches!(e, RuntimeError::SystemUpstreamError(_)));
}

#[test]
fn test_basic_package() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();

    // Act
    let code = wat2wasm(include_str!("wasm/basic_package.wat"));
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .publish_package_advanced(
            None,
            code,
            single_function_package_definition("Test", "f"),
            BTreeMap::new(),
            OwnerRole::None,
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn test_basic_package_missing_export() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let mut blueprints = index_map_new();
    blueprints.insert(
        "Test".to_string(),
        BlueprintDefinitionInit {
            blueprint_type: BlueprintType::default(),
            is_transient: false,
            feature_set: indexset!(),
            dependencies: indexset!(),

            schema: BlueprintSchemaInit {
                generics: vec![],
                schema: VersionedScryptoSchema::V1(SchemaV1 {
                    type_kinds: vec![],
                    type_metadata: vec![],
                    type_validations: vec![],
                }),
                state: BlueprintStateSchemaInit {
                    fields: vec![FieldSchema::static_field(LocalTypeIndex::WellKnown(
                        UNIT_TYPE,
                    ))],
                    collections: vec![],
                },
                events: BlueprintEventSchemaInit::default(),
                functions: BlueprintFunctionsSchemaInit {
                    functions: indexmap!(
                        "f".to_string() => FunctionSchemaInit {
                            receiver: Option::None,
                            input: TypeRef::Static(LocalTypeIndex::WellKnown(ANY_TYPE)),
                            output: TypeRef::Static(LocalTypeIndex::WellKnown(ANY_TYPE)),
                            export: "not_exist".to_string(),
                        }
                    ),
                },
                hooks: BlueprintHooksInit::default(),
            },

            royalty_config: PackageRoyaltyConfig::default(),
            auth_config: AuthConfig::default(),
        },
    );
    // Act
    let code = wat2wasm(include_str!("wasm/basic_package.wat"));
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .publish_package_advanced(
            None,
            code,
            PackageDefinition { blueprints },
            BTreeMap::new(),
            OwnerRole::None,
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::ApplicationError(ApplicationError::PackageError(
                PackageError::InvalidWasm(PrepareError::MissingExport { .. })
            ))
        )
    });
}

#[test]
fn bad_function_schema_should_fail() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();

    // Act
    let (code, definition) = Compile::compile("./tests/blueprints/package_invalid");
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .publish_package_advanced(None, code, definition, BTreeMap::new(), OwnerRole::None)
        .build();

    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::ApplicationError(ApplicationError::PackageError(
                PackageError::InvalidLocalTypeIndex(_)
            ))
        )
    });
}

#[test]
fn should_not_be_able_to_publish_wasm_package_outside_of_transaction_processor() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let package = test_runner.compile_and_publish("./tests/blueprints/publish_package");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(
            package,
            "PublishPackage",
            "publish_package",
            manifest_args!(),
        )
        .build();

    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::SystemModuleError(SystemModuleError::AuthError(AuthError::Unauthorized(
                ..
            )))
        )
    });
}

#[test]
fn should_not_be_able_to_publish_advanced_wasm_package_outside_of_transaction_processor() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let package = test_runner.compile_and_publish("./tests/blueprints/publish_package");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(
            package,
            "PublishPackage",
            "publish_package_advanced",
            manifest_args!(),
        )
        .build();

    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::SystemModuleError(SystemModuleError::AuthError(AuthError::Unauthorized(
                ..
            )))
        )
    });
}

#[test]
fn should_not_be_able_to_publish_native_packages() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(
            PACKAGE_PACKAGE,
            PACKAGE_BLUEPRINT,
            "publish_native",
            PackagePublishNativeManifestInput {
                package_address: None,
                native_package_code_id: 0u64,
                definition: PackageDefinition::default(),
                metadata: metadata_init!(),
            },
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::SystemModuleError(SystemModuleError::AuthError(AuthError::Unauthorized(
                ..
            )))
        )
    });
}

#[test]
fn should_not_be_able_to_publish_native_packages_in_scrypto() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let package = test_runner.compile_and_publish("./tests/blueprints/publish_package");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(
            package,
            "PublishPackage",
            "publish_native",
            manifest_args!(),
        )
        .build();

    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::SystemModuleError(SystemModuleError::AuthError(AuthError::Unauthorized(
                ..
            )))
        )
    });
}

#[test]
fn name_validation_blueprint() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let (code, mut definition) = Compile::compile("./tests/blueprints/publish_package");

    definition.blueprints = indexmap![
       String::from("wrong_bluepint_name_*") =>
            definition
                .blueprints
                .values_mut()
                .next()
                .unwrap()
                .to_owned(),
    ];

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .publish_package_advanced(None, code, definition, BTreeMap::new(), OwnerRole::None)
        .build();

    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::ApplicationError(ApplicationError::PackageError(
                PackageError::InvalidName(..)
            ))
        )
    });
}

#[test]
fn name_validation_feature_set() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let (code, mut definition) = Compile::compile("./tests/blueprints/publish_package");

    definition
        .blueprints
        .values_mut()
        .next()
        .unwrap()
        .feature_set
        .insert(String::from("wrong-feature"));

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .publish_package_advanced(None, code, definition, BTreeMap::new(), OwnerRole::None)
        .build();

    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::ApplicationError(ApplicationError::PackageError(
                PackageError::InvalidName(..)
            ))
        )
    });
}

#[test]
fn name_validation_function() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();

    let (code, mut definition) = Compile::compile("./tests/blueprints/publish_package");

    definition
        .blueprints
        .values_mut()
        .next()
        .unwrap()
        .schema
        .functions
        .functions
        .insert(
            String::from("self"),
            FunctionSchemaInit {
                receiver: None,
                input: TypeRef::Static(LocalTypeIndex::WellKnown(ANY_TYPE)),
                output: TypeRef::Static(LocalTypeIndex::WellKnown(ANY_TYPE)),
                export: String::from("self"),
            },
        );

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .publish_package_advanced(None, code, definition, BTreeMap::new(), OwnerRole::None)
        .build();

    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::ApplicationError(ApplicationError::PackageError(
                PackageError::InvalidName(..)
            ))
        )
    });
}

#[test]
fn well_known_types_in_schema_are_validated() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();

    let (code, mut definition) = Compile::compile("./tests/blueprints/publish_package");

    let method_definition = definition
        .blueprints
        .values_mut()
        .next()
        .unwrap()
        .schema
        .functions
        .functions
        .get_mut("some_method".into())
        .unwrap();

    // Invalid well known type
    method_definition.input = TypeRef::Static(LocalTypeIndex::WellKnown(WellKnownTypeIndex::of(0)));

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .publish_package_advanced(None, code, definition, BTreeMap::new(), OwnerRole::None)
        .build();

    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::ApplicationError(ApplicationError::PackageError(
                PackageError::InvalidLocalTypeIndex(..)
            ))
        )
    });
}

use crate::traversal::*;
use crate::*;

pub trait ValidatableCustomTypeExtension: CustomTypeExtension {
    type ValidationContext;

    // Note that the current SBOR extension only supports terminal custom type,
    // i.e., no custom value can be container.

    fn validate_custom_value<'de, L: SchemaTypeLink>(
        custom_value_ref: &<Self::CustomTraversal as CustomTraversal>::CustomTerminalValueRef<'de>,
        custom_type_kind: &Self::CustomTypeKind<L>,
        context: &Self::ValidationContext,
    );
}

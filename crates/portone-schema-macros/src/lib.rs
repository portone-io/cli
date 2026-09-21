//! Import named string enums and object fields from the committed PortOne OpenAPI schema.

mod expand;
mod fields;

use std::sync::LazyLock;

use proc_macro::TokenStream;
use serde_json::Value;

// Embedding in the macro crate makes schema edits a compiler-tracked dependency.
// No filesystem or network access is performed during macro expansion.
static SCHEMA: LazyLock<Result<Value, serde_json::Error>> =
    LazyLock::new(|| serde_json::from_str(include_str!("../schema/v2.openapi.json")));

/// Generate a named string enum with clap value parsing and serde serialization.
///
/// ```text
/// use portone_schema_macros::schema_enum;
///
/// schema_enum!(pub PaymentStatus);
/// schema_enum!(pub Currency, cli_case = "preserve");
/// ```
///
/// CLI values default to lowercase kebab-case. `cli_case = "preserve"` keeps
/// the schema spelling. API serialization always preserves the original value.
#[proc_macro]
pub fn schema_enum(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as expand::Input);
    let result = match &*SCHEMA {
        Ok(schema) => expand::generate(schema, &input),
        Err(error) => Err(syn::Error::new(
            input.name.span(),
            format!("invalid embedded OpenAPI schema: {error}"),
        )),
    };
    result.unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Generate a sorted, deduplicated slice of top-level JSON field names.
///
/// ```text
/// use portone_schema_macros::schema_fields;
///
/// const PAYMENT_FIELDS: &[&str] = schema_fields!(Payment);
/// ```
///
/// Local schema references and `oneOf` variants are followed to collect the union
/// of object properties. Nested properties are not traversed. Missing schemas,
/// invalid or cyclic references, and unsupported schema shapes are compile errors.
#[proc_macro]
pub fn schema_fields(input: TokenStream) -> TokenStream {
    let name = syn::parse_macro_input!(input as syn::Ident);
    let result = match &*SCHEMA {
        Ok(schema) => fields::generate(schema, &name),
        Err(error) => Err(syn::Error::new(
            name.span(),
            format!("invalid embedded OpenAPI schema: {error}"),
        )),
    };
    result.unwrap_or_else(syn::Error::into_compile_error).into()
}

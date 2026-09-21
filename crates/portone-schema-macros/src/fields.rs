use std::collections::{BTreeSet, HashSet};

use proc_macro2::TokenStream;
use quote::quote;
use serde_json::{Map, Value};
use syn::Ident;

pub(super) fn generate(schema: &Value, name: &Ident) -> syn::Result<TokenStream> {
    let type_name = name.to_string();
    let schemas = schema
        .get("components")
        .and_then(|value| value.get("schemas"))
        .and_then(Value::as_object)
        .ok_or_else(|| syn::Error::new(name.span(), "OpenAPI schema has no named schemas"))?;
    let definition = schemas.get(&type_name).ok_or_else(|| {
        syn::Error::new(
            name.span(),
            format!("schema type `{type_name}` does not exist"),
        )
    })?;
    let mut active = HashSet::from([type_name]);
    let mut fields = BTreeSet::new();
    collect(definition, schemas, name, &mut active, &mut fields)?;
    Ok(quote! { &[#(#fields),*] })
}

fn collect(
    definition: &Value,
    schemas: &Map<String, Value>,
    name: &Ident,
    active: &mut HashSet<String>,
    fields: &mut BTreeSet<String>,
) -> syn::Result<()> {
    let error =
        |message: String| syn::Error::new(name.span(), format!("schema type `{name}`: {message}"));
    let definition = definition
        .as_object()
        .ok_or_else(|| error("expected an object schema definition".to_owned()))?;
    for keyword in ["allOf", "anyOf", "not"] {
        if definition.contains_key(keyword) {
            return Err(error(format!("unsupported schema keyword `{keyword}`")));
        }
    }
    if let Some(kind) = definition.get("type")
        && kind.as_str() != Some("object")
    {
        return Err(error("expected an object schema".to_owned()));
    }

    let mut recognized = false;
    if let Some(properties) = definition.get("properties") {
        let properties = properties
            .as_object()
            .ok_or_else(|| error("`properties` must be an object".to_owned()))?;
        // Only root keys are selectable; nested property schemas are irrelevant.
        fields.extend(properties.keys().cloned());
        recognized = true;
    }

    if let Some(reference) = definition.get("$ref") {
        let reference = reference
            .as_str()
            .ok_or_else(|| error("`$ref` must be a string".to_owned()))?;
        let target = reference
            .strip_prefix("#/components/schemas/")
            .filter(|target| {
                !target.is_empty()
                    && !target.contains('/')
                    && target
                        .split('~')
                        .skip(1)
                        .all(|escape| escape.starts_with('0') || escape.starts_with('1'))
            })
            .ok_or_else(|| {
                error(format!(
                    "expected a local reference to a named schema, found `{reference}`"
                ))
            })?
            .replace("~1", "/")
            .replace("~0", "~");
        let target_definition = schemas
            .get(&target)
            .ok_or_else(|| error(format!("referenced schema `{target}` does not exist")))?;
        if !active.insert(target.clone()) {
            return Err(error(format!("cyclic schema reference to `{target}`")));
        }
        collect(target_definition, schemas, name, active, fields)?;
        active.remove(&target);
        recognized = true;
    }

    if let Some(variants) = definition.get("oneOf") {
        let variants = variants
            .as_array()
            .filter(|variants| !variants.is_empty())
            .ok_or_else(|| error("`oneOf` must be a nonempty array".to_owned()))?;
        for variant in variants {
            collect(variant, schemas, name, active, fields)?;
        }
        recognized = true;
    }

    if !recognized {
        return Err(error(
            "expected object properties, a local schema reference, or `oneOf`".to_owned(),
        ));
    }
    Ok(())
}

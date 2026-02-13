//! Response model field generation for HTTP API responses.
//!
//! This module generates the field definitions and assignments for Response structs
//! that are returned from GET endpoints. Join fields are included to enable
//! relationship data in API responses.

use crate::attribute_parser::get_crudcrate_bool;
use crate::codegen::joins::config::get_join_config;
use crate::fields::relations::{detect_relation_field, RelationType};
use quote::{quote, ToTokens};

/// Extract module path from entity path like `super::fruit::Entity` → `super::fruit`
fn extract_module_path_from_entity(entity_path: &proc_macro2::TokenStream) -> Option<proc_macro2::TokenStream> {
    let path_str = entity_path.to_string();
    // Clean up spaces from token stream formatting (e.g., "super :: fruit :: Entity")
    let cleaned_path = path_str.replace(" ", "");
    let cleaned = cleaned_path
        .trim_end_matches("Entity")
        .trim_end_matches("::")
        .trim()
        .trim_end_matches("::")
        .trim();

    if cleaned.is_empty() {
        None
    } else {
        match syn::parse_str::<syn::Path>(cleaned) {
            Ok(path) => Some(quote! { #path }),
            Err(_) => None,
        }
    }
}

/// Generate field assignment expressions for converting API struct to Response.
///
/// Join fields are included so relationship data appears in HTTP responses.
/// Relation fields are initialized with default values (empty Vec or None).
pub(crate) fn generate_response_from_assignments(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .filter(|field| {
            // Filter by one_model attribute (allows excluding fields from single-item responses)
            get_crudcrate_bool(field, "one_model").unwrap_or(true)
        })
        .filter_map(|field| {
            let ident = &field.ident;
            
            // Check if this is a relation field
            if let Some(relation_info) = detect_relation_field(field) {
                // Generate default value based on relation type
                let default_value = match relation_info.relation_type {
                    RelationType::HasMany | RelationType::HasManyVia { .. } | RelationType::SelfRef { .. } => {
                        quote! { Vec::new() }
                    }
                    RelationType::BelongsTo | RelationType::HasOne => {
                        quote! { None }
                    }
                };
                return Some(quote! {
                    #ident: #default_value
                });
            }
            
            // Regular field - copy from model
            Some(quote! {
                #ident: model.#ident
            })
        })
        .collect()
}

/// Generate field definitions for Response struct.
///
/// Relation fields are transformed:
/// - HasMany<Entity> → Vec<Model>
/// - HasOne<Entity> → Option<Model>
pub(crate) fn generate_response_struct_fields(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    api_struct_name: &syn::Ident,
    skip_utoipa: bool,
) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .filter(|field| {
            // Filter by one_model attribute
            get_crudcrate_bool(field, "one_model").unwrap_or(true)
        })
        .map(|field| {
            let ident = &field.ident;
            let ty = &field.ty;

            // Check if this is a SeaORM 2.0 relation field
            if let Some(relation_info) = detect_relation_field(field) {
                let module_path = extract_module_path_from_entity(&relation_info.related_entity_path);
                
                // Use Model type for all relations - we can't reliably know the target Response
                // type name since it depends on the target entity's table_name/api_struct_name
                let response_ty = match relation_info.relation_type {
                    RelationType::HasMany | RelationType::HasManyVia { .. } | RelationType::SelfRef { .. } => {
                        // HasMany uses Vec<Model> type
                        match &module_path {
                            Some(path) => quote! { Vec<#path::Model> },
                            None => quote! { Vec<Model> },
                        }
                    }
                    RelationType::BelongsTo | RelationType::HasOne => {
                        // BelongsTo/HasOne uses Option<Model> type
                        match &module_path {
                            Some(path) => quote! { Option<#path::Model> },
                            None => quote! { Option<Model> },
                        }
                    }
                };

                // Add serde attributes for optional/empty fields
                let serde_attr = match relation_info.relation_type {
                    RelationType::HasMany | RelationType::HasManyVia { .. } | RelationType::SelfRef { .. } => {
                        quote! { #[serde(default, skip_serializing_if = "Vec::is_empty")] }
                    }
                    RelationType::HasOne | RelationType::BelongsTo => {
                        quote! { #[serde(default, skip_serializing_if = "Option::is_none")] }
                    }
                };

                // Add schema attribute only if not skipping utoipa
                let schema_attr = if skip_utoipa {
                    quote! {}
                } else {
                    quote! { #[schema(value_type = Object)] }
                };

                return quote! {
                    #schema_attr
                    #serde_attr
                    pub #ident: #response_ty
                };
            }

            // Copy non-crudcrate attributes to the response field
            let attrs: Vec<_> = field
                .attrs
                .iter()
                .filter(|attr| !attr.path().is_ident("crudcrate") && !attr.path().is_ident("sea_orm"))
                .collect();

            // Check if this is a self-referencing or join field
            let field_type_string = ty.to_token_stream().to_string();
            let is_self_referencing = field_type_string.contains(&api_struct_name.to_string());
            let is_join_field = get_join_config(field).is_some();

            // Add schema(no_recursion) for self-referencing or join fields to prevent
            // infinite recursion in OpenAPI schema generation (only if not skipping utoipa)
            let schema_attr = if !skip_utoipa && (is_self_referencing || is_join_field) {
                Some(quote! {
                    #[schema(no_recursion)]
                })
            } else {
                None
            };

            let final_ty = quote! { #ty };

            quote! {
                #schema_attr
                #(#attrs)*
                pub #ident: #final_ty
            }
        })
        .collect()
}

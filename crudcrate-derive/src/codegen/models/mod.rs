pub mod api_struct;
pub mod create;
pub mod list;
pub mod list_response;
pub mod merge;
pub mod response;
pub mod shared;
pub mod update;

use crate::attribute_parser::get_crudcrate_bool;
use crate::codegen::joins::get_join_config;
use crate::fields::relations::is_relation_field;

/// Check if a field is an auto-increment primary key
/// Auto-increment PKs are excluded from Create/Update models
/// Composite PKs (auto_increment = false) are included in Create model
fn is_auto_increment_primary_key(field: &syn::Field) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident("sea_orm") {
            if let syn::Meta::List(meta_list) = &attr.meta {
                let tokens = meta_list.tokens.to_string();
                if tokens.contains("primary_key") {
                    // Check if auto_increment = false - these should be included
                    if tokens.contains("auto_increment = false") || tokens.contains("auto_increment=false") {
                        return false; // Composite PK - not auto-increment
                    }
                    return true; // Default: auto-increment PK
                }
            }
        }
    }
    false
}

/// Check if a field is any type of primary key (auto-increment or composite)
fn is_any_primary_key(field: &syn::Field) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident("sea_orm") {
            if let syn::Meta::List(meta_list) = &attr.meta {
                let tokens = meta_list.tokens.to_string();
                if tokens.contains("primary_key") {
                    return true;
                }
            }
        }
    }
    false
}

/// Shared field filtering logic for model generation
/// Determines if a field should be included in a specific model type
pub(crate) fn should_include_in_model(field: &syn::Field, model_type: &str) -> bool {
    // Check the model-specific attribute (create_model, update_model, list_model)
    let include_in_model = get_crudcrate_bool(field, model_type).unwrap_or(true);

    // Handle primary key fields based on model type
    match model_type {
        "create_model" => {
            // Only exclude auto-increment PKs from Create
            // Composite PKs (auto_increment = false) are included
            if is_auto_increment_primary_key(field) {
                return false;
            }
        }
        "update_model" => {
            // Exclude ALL primary keys from Update (both auto-increment and composite)
            if is_any_primary_key(field) {
                return false;
            }
        }
        _ => {}
    }

    // SeaORM 2.0 relation fields (HasOne, HasMany) are handled separately
    // They should not be included through the normal field generation path
    // as they have special type transformations (e.g., HasOne<E> -> Option<ECreate>)
    if is_relation_field(field) {
        match model_type {
            "create_model" | "update_model" => {
                // Relation fields in create/update are handled by generate_create_relation_fields
                // and generate_update_relation_fields respectively
                return false;
            }
            "list_model" => {
                // For list model, we might want to include loaded relations
                // but with transformed types - handled separately
                return false;
            }
            _ => {}
        }
    }

    // Handle join field exclusion based on model type (legacy crudcrate join system)
    if let Some(join_config) = get_join_config(field).config {
        match model_type {
            "create_model" | "update_model" => {
                // Create/Update models: exclude ALL join fields
                return false;
            }
            "list_model" => {
                // List model: only exclude join(one) fields, keep join(all)
                // Exclude if NOT loading in get_all (on_all = false)
                if !join_config.on_all {
                    return false;
                }
            }
            _ => {}
        }
    }

    include_in_model
}

use crate::attribute_parser::{get_crudcrate_bool, get_crudcrate_expr};
use crate::codegen::models::shared::{
    generate_active_value_set, generate_field_with_optional_default,
    resolve_field_type_with_target_models,
};
use crate::codegen::models::should_include_in_model;
use crate::fields::field_is_optional;
use crate::fields::relations::{
    RelationFieldInfo, RelationType, detect_relation_field, generate_create_field_type,
    should_include_relation_in_create,
};
use quote::quote;

/// Check if a field has the sea_orm(primary_key) attribute AND is auto-increment
/// Auto-increment PKs are excluded from Create model
/// Composite PKs (auto_increment = false) are included
fn is_auto_increment_primary_key(field: &syn::Field) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident("sea_orm") {
            if let syn::Meta::List(meta_list) = &attr.meta {
                let tokens = meta_list.tokens.to_string();
                if tokens.contains("primary_key") {
                    // Check if auto_increment = false
                    if tokens.contains("auto_increment = false") || tokens.contains("auto_increment=false") {
                        return false; // Composite PK - should be included in Create
                    }
                    return true; // Default: auto-increment PK - exclude from Create
                }
            }
        }
    }
    false
}

/// Generates the conversion lines for a create model to active model conversion
pub(crate) fn generate_create_conversion_lines(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> Vec<proc_macro2::TokenStream> {
    let mut conv_lines = Vec::new();
    for field in fields {
        if get_crudcrate_bool(field, "non_db_attr").unwrap_or(false) {
            continue;
        }
        // Skip relation fields - they don't exist on ActiveModel
        if detect_relation_field(field).is_some() {
            continue;
        }
        // Skip auto-increment primary_key fields - they are auto-generated
        // Composite PKs (auto_increment = false) are handled like regular fields
        if is_auto_increment_primary_key(field) {
            let ident = field.ident.as_ref().unwrap();
            conv_lines.push(quote! {
                #ident: sea_orm::ActiveValue::NotSet
            });
            continue;
        }
        let ident = field.ident.as_ref().unwrap();
        let include = get_crudcrate_bool(field, "create_model").unwrap_or(true);
        let is_optional = field_is_optional(field);

        if include {
            if let Some(expr) = get_crudcrate_expr(field, "on_create") {
                if is_optional {
                    conv_lines.push(quote! {
                        #ident: sea_orm::ActiveValue::Set(match create.#ident {
                            Some(Some(inner)) => Some(inner.into()),
                            Some(None)         => None,
                            None               => Some((#expr).into()),
                        })
                    });
                } else {
                    conv_lines.push(quote! {
                        #ident: sea_orm::ActiveValue::Set(match create.#ident {
                            Some(val) => val.into(),
                            None      => (#expr).into(),
                        })
                    });
                }
            } else if is_optional {
                conv_lines.push(quote! {
                    #ident: sea_orm::ActiveValue::Set(create.#ident.map(|v| v.into()))
                });
            } else {
                conv_lines.push(quote! {
                    #ident: sea_orm::ActiveValue::Set(create.#ident.into())
                });
            }
        } else if let Some(expr) = get_crudcrate_expr(field, "on_create") {
            conv_lines.push(generate_active_value_set(ident, &expr, is_optional));
        } else {
            // Field is excluded from Create model and has no on_create - set to NotSet
            // This allows the field to be set manually later in custom create functions
            conv_lines.push(quote! {
                #ident: sea_orm::ActiveValue::NotSet
            });
        }
    }
    conv_lines
}

pub(crate) fn generate_create_struct_fields(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .filter_map(|field| {
            let ident = &field.ident;
            let ty = &field.ty;

            // Check if this is a SeaORM 2.0 relation field - handle separately
            if let Some(relation_info) = detect_relation_field(field) {
                if should_include_relation_in_create(&relation_info) {
                    let relation_ty = generate_create_field_type(&relation_info);
                    // Use value_type = Object to avoid utoipa requiring ToSchema on nested Model types
                    return Some(quote! {
                        #[schema(value_type = Object)]
                        #[serde(default, skip_serializing_if = "Option::is_none")]
                        pub #ident: #relation_ty
                    });
                }
                // Relation field that shouldn't be included
                return None;
            }

            // Non-relation field - check if it should be included
            if !should_include_in_model(field, "create_model") {
                return None;
            }

            if get_crudcrate_bool(field, "non_db_attr").unwrap_or(false) {
                // Resolve type with target models (create model)
                let final_ty =
                    resolve_field_type_with_target_models(ty, field, |create, _, _| create.clone());
                Some(generate_field_with_optional_default(ident.as_ref(), &final_ty, field))
            } else if get_crudcrate_expr(field, "on_create").is_some() {
                Some(quote! {
                    #[serde(default)]
                    pub #ident: Option<#ty>
                })
            } else {
                Some(quote! {
                    pub #ident: #ty
                })
            }
        })
        .collect()
}

/// Generate relation field struct fields for create model
/// This is called separately to add relation fields that aren't part of the regular DB fields
#[allow(dead_code)]
pub(crate) fn generate_create_relation_fields(
    relation_fields: &[RelationFieldInfo],
) -> Vec<proc_macro2::TokenStream> {
    relation_fields
        .iter()
        .filter(|info| should_include_relation_in_create(info))
        .map(|info| {
            let ident = &info.field_name;
            let relation_ty = generate_create_field_type(info);
            quote! {
                #[serde(default, skip_serializing_if = "Option::is_none")]
                pub #ident: #relation_ty
            }
        })
        .collect()
}

/// Generate conversion lines for relation fields in create model
#[allow(dead_code)]
pub(crate) fn generate_create_relation_conversion_lines(
    relation_fields: &[RelationFieldInfo],
) -> Vec<proc_macro2::TokenStream> {
    relation_fields
        .iter()
        .filter(|info| should_include_relation_in_create(info))
        .map(|info| {
            let ident = &info.field_name;
            match &info.relation_type {
                RelationType::HasOne => {
                    // HasOne fields use HasOneModel::Set or HasOneModel::NotSet
                    quote! {
                        #ident: match create.#ident {
                            Some(nested) => sea_orm::HasOneModel::Set(Box::new((*nested).into())),
                            None => sea_orm::HasOneModel::NotSet,
                        }
                    }
                }
                RelationType::HasMany | RelationType::HasManyVia { .. } => {
                    // HasMany fields use HasManyModel::Append or HasManyModel::NotSet
                    quote! {
                        #ident: match create.#ident {
                            Some(nested_vec) => sea_orm::HasManyModel::Append(
                                nested_vec.into_iter().map(Into::into).collect()
                            ),
                            None => sea_orm::HasManyModel::NotSet,
                        }
                    }
                }
                RelationType::SelfRef { .. } => {
                    // Self-referential relations also use HasManyModel
                    quote! {
                        #ident: match create.#ident {
                            Some(nested_vec) => sea_orm::HasManyModel::Append(
                                nested_vec.into_iter().map(Into::into).collect()
                            ),
                            None => sea_orm::HasManyModel::NotSet,
                        }
                    }
                }
                RelationType::BelongsTo => {
                    // BelongsTo is typically not included in create, but handle it anyway
                    quote! {
                        #ident: sea_orm::HasOneModel::NotSet
                    }
                }
            }
        })
        .collect()
}

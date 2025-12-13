use crate::attribute_parser::get_crudcrate_bool;
use crate::codegen::models::shared::{
    generate_field_with_optional_default, resolve_field_type_with_target_models,
};
use crate::codegen::models::should_include_in_model;
use crate::fields::relations::{
    RelationFieldInfo, RelationType, detect_relation_field, generate_update_field_type,
    should_include_relation_in_update,
};
use quote::quote;

/// Generates the field declarations for an update struct
pub(crate) fn generate_update_struct_fields(
    included_fields: &[&syn::Field],
) -> Vec<proc_macro2::TokenStream> {
    included_fields
        .iter()
        .map(|field| {
            let ident = &field.ident;
            let ty = &field.ty;

            // Check if this is a SeaORM 2.0 relation field
            if let Some(relation_info) = detect_relation_field(field) {
                if should_include_relation_in_update(&relation_info) {
                    let relation_ty = generate_update_field_type(&relation_info);
                    return quote! {
                        #[serde(default, skip_serializing_if = "Option::is_none")]
                        pub #ident: #relation_ty
                    };
                }
            }

            if get_crudcrate_bool(field, "non_db_attr").unwrap_or(false) {
                // Resolve type with target models (update model)
                let final_ty =
                    resolve_field_type_with_target_models(ty, field, |_, update, _| update.clone());
                generate_field_with_optional_default(ident.as_ref(), &final_ty, field)
            } else {
                // Extract inner type from Option<T> - inline replacement for extract_inner_type_for_update
                let inner_ty = if let syn::Type::Path(type_path) = ty
                    && let Some(last_seg) = type_path.path.segments.last()
                    && last_seg.ident == "Option"
                    && let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments
                    && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
                {
                    inner.clone()
                } else {
                    ty.clone()
                };
                quote! {
                    #[serde(
                        default,
                        skip_serializing_if = "Option::is_none",
                        with = "crudcrate::serde_with::rust::double_option"
                    )]
                    pub #ident: Option<Option<#inner_ty>>
                }
            }
        })
        .collect()
}

/// Filters fields that should be included in update model
pub(crate) fn filter_update_fields(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> Vec<&syn::Field> {
    fields
        .iter()
        .filter(|field| should_include_in_model(field, "update_model"))
        .collect()
}

/// Generate relation field struct fields for update model
/// This is called separately to add relation fields that aren't part of the regular DB fields
#[allow(dead_code)]
pub(crate) fn generate_update_relation_fields(
    relation_fields: &[RelationFieldInfo],
) -> Vec<proc_macro2::TokenStream> {
    relation_fields
        .iter()
        .filter(|info| should_include_relation_in_update(info))
        .map(|info| {
            let ident = &info.field_name;
            let relation_ty = generate_update_field_type(info);
            quote! {
                #[serde(default, skip_serializing_if = "Option::is_none")]
                pub #ident: #relation_ty
            }
        })
        .collect()
}

/// Generate merge lines for relation fields in update model
#[allow(dead_code)]
pub(crate) fn generate_update_relation_merge_lines(
    relation_fields: &[RelationFieldInfo],
) -> Vec<proc_macro2::TokenStream> {
    relation_fields
        .iter()
        .filter(|info| should_include_relation_in_update(info))
        .map(|info| {
            let ident = &info.field_name;
            match &info.relation_type {
                RelationType::HasOne => {
                    // HasOne update: Some(Some(data)) = update, Some(None) = remove, None = skip
                    quote! {
                        if let Some(relation_update) = self.#ident {
                            model.#ident = match relation_update {
                                Some(nested) => sea_orm::HasOneModel::Set(Box::new((*nested).into())),
                                None => sea_orm::HasOneModel::NotSet, // or handle deletion
                            };
                        }
                    }
                }
                RelationType::HasMany | RelationType::HasManyVia { .. } => {
                    // HasMany update: Some(vec) = replace/append, None = skip
                    quote! {
                        if let Some(nested_vec) = self.#ident {
                            model.#ident = sea_orm::HasManyModel::Replace(
                                nested_vec.into_iter().map(Into::into).collect()
                            );
                        }
                    }
                }
                RelationType::SelfRef { .. } => {
                    quote! {
                        if let Some(nested_vec) = self.#ident {
                            model.#ident = sea_orm::HasManyModel::Replace(
                                nested_vec.into_iter().map(Into::into).collect()
                            );
                        }
                    }
                }
                RelationType::BelongsTo => {
                    // BelongsTo typically not updated via nested model
                    quote! {}
                }
            }
        })
        .collect()
}

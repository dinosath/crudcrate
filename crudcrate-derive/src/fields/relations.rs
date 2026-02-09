//! SeaORM 2.0 relation field detection and parsing
//!
//! This module handles detection and parsing of SeaORM 2.0's new relation field format:
//! - `HasOne<Entity>` - One-to-one or many-to-one relationships
//! - `HasMany<Entity>` - One-to-many relationships
//! - Fields with `#[sea_orm(has_one)]`, `#[sea_orm(has_many)]`, `#[sea_orm(belongs_to)]` attributes
//!
//! # SeaORM 2.0 Relation Support
//!
//! SeaORM 2.0 introduced a new entity format that allows relations to be defined directly
//! on model fields using `HasOne<E>` and `HasMany<E>` types:
//!
//! ```ignore
//! #[sea_orm::model]
//! #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
//! #[sea_orm(table_name = "user")]
//! pub struct Model {
//!     #[sea_orm(primary_key)]
//!     pub id: i32,
//!     pub name: String,
//!     #[sea_orm(has_one)]
//!     pub profile: HasOne<super::profile::Entity>,
//!     #[sea_orm(has_many)]
//!     pub posts: HasMany<super::post::Entity>,
//! }
//! ```
//!
//! # Generated Create/Update Models
//!
//! When using `EntityToModels` derive macro, relation fields are automatically transformed:
//!
//! - `HasOne<Entity>` in Create model becomes `Option<Box<EntityCreate>>` (for nested creation)
//! - `HasMany<Entity>` in Create model becomes `Option<Vec<EntityCreate>>` (for nested creation)
//! - `BelongsTo` relation in Create model becomes `Option<Box<Entity>>` (references existing entity)
//! - `HasOne<Entity>` in Update model becomes `Option<Option<Box<EntityUpdate>>>` (for nested updates)
//! - `HasMany<Entity>` in Update model becomes `Option<Vec<EntityUpdate>>` (for nested updates)
//! - `BelongsTo` relation in Update model becomes `Option<Option<Box<Entity>>>` (references existing entity)
//!
//! # Example Usage
//!
//! ```ignore
//! // Creating a user with nested profile and posts:
//! let user_create = UserCreate {
//!     name: "Alice".to_string(),
//!     email: "alice@example.com".to_string(),
//!     profile: Some(Box::new(ProfileCreate {
//!         picture: "avatar.jpg".to_string(),
//!     })),
//!     posts: Some(vec![
//!         PostCreate { title: "First Post".to_string() },
//!         PostCreate { title: "Second Post".to_string() },
//!     ]),
//! };
//!
//! // Creating an invoice with reference to existing purchase tax series:
//! let invoice_create = InvoiceCreate {
//!     invoice_number: "INV-001".to_string(),
//!     purchase_tax_series_id: Some(123),  // FK to existing entity
//!     purchase_tax_series: Some(Box::new(PurchaseTaxSeries {
//!         id: 123,
//!         name: "Series A".to_string(),
//!         // ... other fields from existing entity
//!     })),
//! };
//! ```
//!
//! # Relation Types
//!
//! - **HasOne**: One-to-one relationship where this entity "has one" of another (uses Create/Update models for nested ops)
//! - **HasMany**: One-to-many relationship where this entity "has many" of another (uses Create/Update models for nested ops)
//! - **BelongsTo**: Inverse relationship (uses base Model to reference existing entities)
//! - **HasManyVia**: Many-to-many relationship via a junction table
//! - **SelfRef**: Self-referential relationship (e.g., followers/following)

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Field, GenericArgument, Meta, PathArguments, Type};

/// Types of relations supported by SeaORM 2.0
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationType {
    /// One-to-one relationship (has_one)
    HasOne,
    /// One-to-many relationship (has_many)
    HasMany,
    /// Belongs to relationship (inverse of has_one/has_many)
    BelongsTo,
    /// Many-to-many relationship via junction table
    HasManyVia { via: String },
    /// Self-referential relationship
    SelfRef { via: Option<String>, reverse: bool },
}

/// Information about a relation field
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RelationFieldInfo {
    /// The field identifier
    pub field_name: syn::Ident,
    /// The type of relation
    pub relation_type: RelationType,
    /// The related entity path (e.g., `super::profile::Entity`)
    pub related_entity_path: TokenStream,
    /// The related entity's model name (e.g., `Profile`)
    pub related_model_name: String,
    /// Foreign key information for belongs_to relations
    pub foreign_key: Option<ForeignKeyInfo>,
}

/// Foreign key information for belongs_to relations
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ForeignKeyInfo {
    /// The local column name (from)
    pub from_column: String,
    /// The remote column name (to)
    pub to_column: String,
}

/// Detects if a field is a SeaORM 2.0 relation field and extracts relation info
pub fn detect_relation_field(field: &Field) -> Option<RelationFieldInfo> {
    let field_name = field.ident.as_ref()?.clone();

    // First check the field type for HasOne<E> or HasMany<E>
    if let Some((relation_type, entity_path, model_name)) = parse_relation_type(&field.ty) {
        // Check for sea_orm attributes to get more details
        let (refined_relation_type, foreign_key) = parse_sea_orm_relation_attrs(field);

        return Some(RelationFieldInfo {
            field_name,
            relation_type: refined_relation_type.unwrap_or(relation_type),
            related_entity_path: entity_path,
            related_model_name: model_name,
            foreign_key,
        });
    }

    // If no relation type found in field type, check attributes only
    if let Some((relation_type, foreign_key)) = parse_sea_orm_relation_attrs(field)
        .0
        .zip(Some(parse_sea_orm_relation_attrs(field).1))
    {
        // Try to extract entity from the type even if it's not HasOne/HasMany wrapper
        if let Some((_, entity_path, model_name)) = extract_entity_from_option_type(&field.ty) {
            return Some(RelationFieldInfo {
                field_name,
                relation_type,
                related_entity_path: entity_path,
                related_model_name: model_name,
                foreign_key,
            });
        }
    }

    None
}

/// Parse the field type to detect HasOne<E> or HasMany<E> patterns
fn parse_relation_type(ty: &Type) -> Option<(RelationType, TokenStream, String)> {
    if let Type::Path(type_path) = ty {
        let last_segment = type_path.path.segments.last()?;
        let ident = &last_segment.ident;

        let relation_type = if ident == "HasOne" {
            RelationType::HasOne
        } else if ident == "HasMany" {
            RelationType::HasMany
        } else {
            return None;
        };

        // Extract the generic argument (the Entity type)
        if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
            if let Some(GenericArgument::Type(inner_type)) = args.args.first() {
                let entity_path = quote! { #inner_type };
                let model_name = extract_model_name_from_entity_path(inner_type);
                return Some((relation_type, entity_path, model_name));
            }
        }
    }

    None
}

/// Extract entity from Option<Box<T>> or similar wrapper types
fn extract_entity_from_option_type(ty: &Type) -> Option<(RelationType, TokenStream, String)> {
    if let Type::Path(type_path) = ty {
        let last_segment = type_path.path.segments.last()?;

        // Handle Option<Entity> or Option<Box<Entity>>
        if last_segment.ident == "Option" {
            if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                if let Some(GenericArgument::Type(inner_type)) = args.args.first() {
                    // Check if it's Option<Box<Entity>>
                    if let Type::Path(inner_path) = inner_type {
                        if let Some(inner_seg) = inner_path.path.segments.last() {
                            if inner_seg.ident == "Box" {
                                if let PathArguments::AngleBracketed(box_args) =
                                    &inner_seg.arguments
                                {
                                    if let Some(GenericArgument::Type(entity_type)) =
                                        box_args.args.first()
                                    {
                                        let entity_path = quote! { #entity_type };
                                        let model_name =
                                            extract_model_name_from_entity_path(entity_type);
                                        return Some((
                                            RelationType::BelongsTo,
                                            entity_path,
                                            model_name,
                                        ));
                                    }
                                }
                            }
                        }
                    }

                    // Direct Option<Entity>
                    let entity_path = quote! { #inner_type };
                    let model_name = extract_model_name_from_entity_path(inner_type);
                    return Some((RelationType::BelongsTo, entity_path, model_name));
                }
            }
        }
    }

    None
}

/// Extract the model name from an entity path like `super::profile::Entity`
fn extract_model_name_from_entity_path(ty: &Type) -> String {
    if let Type::Path(type_path) = ty {
        // Look for the module name before "Entity"
        let segments: Vec<_> = type_path.path.segments.iter().collect();

        // Find the module name (second to last segment, or handle special cases)
        for (i, segment) in segments.iter().enumerate() {
            if segment.ident == "Entity" && i > 0 {
                // The previous segment is the module name
                let module_name = segments[i - 1].ident.to_string();
                return to_pascal_case(&module_name);
            }
        }

        // Fallback: use the last segment if it's not "Entity"
        if let Some(last) = segments.last() {
            if last.ident != "Entity" {
                return to_pascal_case(&last.ident.to_string());
            }
        }
    }

    "Unknown".to_string()
}

/// Convert snake_case to PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

/// Parse sea_orm attributes to detect relation type and foreign key info
fn parse_sea_orm_relation_attrs(field: &Field) -> (Option<RelationType>, Option<ForeignKeyInfo>) {
    let mut relation_type = None;
    let mut foreign_key = None;

    for attr in &field.attrs {
        if !attr.path().is_ident("sea_orm") {
            continue;
        }

        if let Meta::List(meta_list) = &attr.meta {
            // Parse the tokens inside sea_orm(...)
            let tokens_str = meta_list.tokens.to_string();

            // Check for has_one
            if tokens_str.contains("has_one") {
                relation_type = Some(RelationType::HasOne);
            }

            // Check for has_many
            if tokens_str.contains("has_many") {
                // Check for via attribute (many-to-many)
                if let Some(via) = extract_attr_value(&tokens_str, "via") {
                    relation_type = Some(RelationType::HasManyVia { via });
                } else {
                    relation_type = Some(RelationType::HasMany);
                }
            }

            // Check for belongs_to
            if tokens_str.contains("belongs_to") {
                relation_type = Some(RelationType::BelongsTo);

                // Extract from and to columns
                let from_col = extract_attr_value(&tokens_str, "from");
                let to_col = extract_attr_value(&tokens_str, "to");

                if let (Some(from), Some(to)) = (from_col, to_col) {
                    foreign_key = Some(ForeignKeyInfo {
                        from_column: from,
                        to_column: to,
                    });
                }
            }

            // Check for self_ref
            if tokens_str.contains("self_ref") {
                let via = extract_attr_value(&tokens_str, "via");
                let reverse = tokens_str.contains("reverse");
                relation_type = Some(RelationType::SelfRef { via, reverse });
            }
        }
    }

    (relation_type, foreign_key)
}

/// Extract a string value from an attribute like `via = "junction_table"`
fn extract_attr_value(tokens_str: &str, attr_name: &str) -> Option<String> {
    // Look for pattern: attr_name = "value"
    let pattern = format!("{attr_name} = \"");
    if let Some(start) = tokens_str.find(&pattern) {
        let value_start = start + pattern.len();
        if let Some(end) = tokens_str[value_start..].find('"') {
            return Some(tokens_str[value_start..value_start + end].to_string());
        }
    }

    // Also try pattern: attr_name = 'value' (single quotes)
    let pattern_single = format!("{attr_name} = '");
    if let Some(start) = tokens_str.find(&pattern_single) {
        let value_start = start + pattern_single.len();
        if let Some(end) = tokens_str[value_start..].find('\'') {
            return Some(tokens_str[value_start..value_start + end].to_string());
        }
    }

    None
}

/// Check if a field is a relation field (has HasOne, HasMany, or belongs_to)
pub fn is_relation_field(field: &Field) -> bool {
    detect_relation_field(field).is_some()
}

/// Check if a field should be included in create model based on relation type
pub fn should_include_relation_in_create(info: &RelationFieldInfo) -> bool {
    // Include HasOne, HasMany, and BelongsTo relations in create model
    matches!(
        info.relation_type,
        RelationType::HasOne | RelationType::HasMany | RelationType::BelongsTo | RelationType::HasManyVia { .. }
    )
}

/// Check if a field should be included in update model based on relation type
pub fn should_include_relation_in_update(info: &RelationFieldInfo) -> bool {
    // Include HasOne, HasMany, and BelongsTo relations in update model
    matches!(
        info.relation_type,
        RelationType::HasOne | RelationType::HasMany | RelationType::BelongsTo | RelationType::HasManyVia { .. }
    )
}

/// Generate the create model field type for a relation
pub fn generate_create_field_type(info: &RelationFieldInfo) -> TokenStream {
    match info.relation_type {
        RelationType::BelongsTo => {
            // BelongsTo uses the base model (ModelEx) since it references an existing entity
            let model_name = syn::Ident::new(
                &info.related_model_name,
                proc_macro2::Span::call_site(),
            );
            quote! { Option<Box<#model_name>> }
        }
        RelationType::HasOne => {
            // HasOne uses Create model for nested creation
            let model_name = syn::Ident::new(
                &format!("{}Create", info.related_model_name),
                proc_macro2::Span::call_site(),
            );
            quote! { Option<Box<#model_name>> }
        }
        RelationType::HasMany | RelationType::HasManyVia { .. } => {
            let model_name = syn::Ident::new(
                &format!("{}Create", info.related_model_name),
                proc_macro2::Span::call_site(),
            );
            quote! { Option<Vec<#model_name>> }
        }
        RelationType::SelfRef { .. } => {
            // Self-referential relations use the same model
            let model_name = syn::Ident::new(
                &format!("{}Create", info.related_model_name),
                proc_macro2::Span::call_site(),
            );
            quote! { Option<Vec<#model_name>> }
        }
    }
}

/// Generate the update model field type for a relation
pub fn generate_update_field_type(info: &RelationFieldInfo) -> TokenStream {
    match info.relation_type {
        RelationType::BelongsTo => {
            // BelongsTo uses the base model (ModelEx) since it references an existing entity
            let model_name = syn::Ident::new(
                &info.related_model_name,
                proc_macro2::Span::call_site(),
            );
            // Option<Option<T>> - outer Option for "skip", inner Option for "null"
            quote! { Option<Option<Box<#model_name>>> }
        }
        RelationType::HasOne => {
            // HasOne uses Update model for nested updates
            let model_name = syn::Ident::new(
                &format!("{}Update", info.related_model_name),
                proc_macro2::Span::call_site(),
            );
            quote! { Option<Option<Box<#model_name>>> }
        }
        RelationType::HasMany | RelationType::HasManyVia { .. } => {
            let model_name = syn::Ident::new(
                &format!("{}Update", info.related_model_name),
                proc_macro2::Span::call_site(),
            );
            quote! { Option<Vec<#model_name>> }
        }
        RelationType::SelfRef { .. } => {
            let model_name = syn::Ident::new(
                &format!("{}Update", info.related_model_name),
                proc_macro2::Span::call_site(),
            );
            quote! { Option<Vec<#model_name>> }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("user"), "User");
        assert_eq!(to_pascal_case("user_profile"), "UserProfile");
        assert_eq!(to_pascal_case("vehicle_part"), "VehiclePart");
        assert_eq!(to_pascal_case("maintenance_record"), "MaintenanceRecord");
    }

    #[test]
    fn test_parse_has_one_relation_type() {
        let ty: syn::Type = parse_quote!(HasOne<super::profile::Entity>);
        let result = parse_relation_type(&ty);
        assert!(result.is_some());
        let (rel_type, _, model_name) = result.unwrap();
        assert_eq!(rel_type, RelationType::HasOne);
        assert_eq!(model_name, "Profile");
    }

    #[test]
    fn test_parse_has_many_relation_type() {
        let ty: syn::Type = parse_quote!(HasMany<super::post::Entity>);
        let result = parse_relation_type(&ty);
        assert!(result.is_some());
        let (rel_type, _, model_name) = result.unwrap();
        assert_eq!(rel_type, RelationType::HasMany);
        assert_eq!(model_name, "Post");
    }

    #[test]
    fn test_parse_non_relation_type() {
        let ty: syn::Type = parse_quote!(String);
        let result = parse_relation_type(&ty);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_attr_value() {
        let tokens = r#"has_many, via = "post_tag""#;
        assert_eq!(
            extract_attr_value(tokens, "via"),
            Some("post_tag".to_string())
        );

        let tokens = r#"belongs_to, from = "user_id", to = "id""#;
        assert_eq!(
            extract_attr_value(tokens, "from"),
            Some("user_id".to_string())
        );
        assert_eq!(extract_attr_value(tokens, "to"), Some("id".to_string()));
    }

    #[test]
    fn test_detect_relation_field_has_one() {
        let field: syn::Field = parse_quote! {
            #[sea_orm(has_one)]
            pub profile: HasOne<super::profile::Entity>
        };

        let info = detect_relation_field(&field);
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.field_name, "profile");
        assert_eq!(info.relation_type, RelationType::HasOne);
        assert_eq!(info.related_model_name, "Profile");
    }

    #[test]
    fn test_detect_relation_field_has_many() {
        let field: syn::Field = parse_quote! {
            #[sea_orm(has_many)]
            pub posts: HasMany<super::post::Entity>
        };

        let info = detect_relation_field(&field);
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.field_name, "posts");
        assert_eq!(info.relation_type, RelationType::HasMany);
        assert_eq!(info.related_model_name, "Post");
    }

    #[test]
    fn test_detect_non_relation_field() {
        let field: syn::Field = parse_quote! {
            pub name: String
        };

        let info = detect_relation_field(&field);
        assert!(info.is_none());
    }

    #[test]
    fn test_generate_create_field_type_has_one() {
        let info = RelationFieldInfo {
            field_name: syn::Ident::new("profile", proc_macro2::Span::call_site()),
            relation_type: RelationType::HasOne,
            related_entity_path: quote! { super::profile::Entity },
            related_model_name: "Profile".to_string(),
            foreign_key: None,
        };

        let result = generate_create_field_type(&info);
        assert!(result.to_string().contains("Option"));
        assert!(result.to_string().contains("ProfileCreate"));
    }

    #[test]
    fn test_generate_create_field_type_has_many() {
        let info = RelationFieldInfo {
            field_name: syn::Ident::new("posts", proc_macro2::Span::call_site()),
            relation_type: RelationType::HasMany,
            related_entity_path: quote! { super::post::Entity },
            related_model_name: "Post".to_string(),
            foreign_key: None,
        };

        let result = generate_create_field_type(&info);
        assert!(result.to_string().contains("Vec"));
        assert!(result.to_string().contains("PostCreate"));
    }

    #[test]
    fn test_should_include_relation_in_create() {
        let has_one_info = RelationFieldInfo {
            field_name: syn::Ident::new("profile", proc_macro2::Span::call_site()),
            relation_type: RelationType::HasOne,
            related_entity_path: quote! {},
            related_model_name: "Profile".to_string(),
            foreign_key: None,
        };
        assert!(should_include_relation_in_create(&has_one_info));

        let belongs_to_info = RelationFieldInfo {
            field_name: syn::Ident::new("user", proc_macro2::Span::call_site()),
            relation_type: RelationType::BelongsTo,
            related_entity_path: quote! {},
            related_model_name: "User".to_string(),
            foreign_key: Some(ForeignKeyInfo {
                from_column: "user_id".to_string(),
                to_column: "id".to_string(),
            }),
        };
        assert!(!should_include_relation_in_create(&belongs_to_info));
    }
}

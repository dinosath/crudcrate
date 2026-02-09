//! Bakery Models Integration Test
//!
//! This test module verifies that the `EntityToModels` derive macro correctly generates
//! Create, Update, List, and Response models for bakery-related entities with SeaORM 2.0
//! relation field syntax.
//!
//! Test entities:
//! - `Cake` - has many Fruits, has many Fillings (via CakeFilling)
//! - `Fruit` - belongs to Cake
//! - `Filling` - has many Cakes (via CakeFilling junction)
//! - `CakeFilling` - junction table linking Cake and Filling
//!
//! The tests verify:
//! 1. Generated Create models have correct fields (relation fields excluded)
//! 2. Generated Update models have correct Option<Option<T>> pattern
//! 3. Generated List models exist and have correct fields
//! 4. Generated Response models include all entity fields
//! 5. Serialization/deserialization works correctly
//! 6. Conversion to ActiveModel works correctly
//! 7. Generated models match manually defined expected models

use sea_orm::entity::prelude::*;

// ============================================================================
// Manually Defined Expected Models
// These are the models we expect the derive macro to generate
// ============================================================================

/// Expected models for Cake entity
pub mod expected_cake {
    use serde::{Deserialize, Serialize};

    /// Expected CakeCreate - excludes id (primary key), includes nested relation with Model type
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct CakeCreate {
        pub name: Option<String>,
        /// HasMany relation - optional nested fruits (uses Fruit Model)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub fruits: Option<Vec<super::fruit::Model>>,
    }

    /// Expected CakeUpdate - Option<Option<T>> pattern for nullable fields
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
    pub struct CakeUpdate {
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "::serde_with::rust::double_option"
        )]
        pub name: Option<Option<String>>,
        /// HasMany relation - optional nested fruits (uses Fruit Model)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub fruits: Option<Vec<super::fruit::Model>>,
    }

    /// Expected CakeList - all db columns, excludes relation fields for efficiency
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct CakeList {
        pub id: i32,
        pub name: Option<String>,
    }

    /// Expected CakeResponse - includes relation data for detail view
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct CakeResponse {
        pub id: i32,
        pub name: Option<String>,
        /// HasMany relation - loaded fruits (uses Fruit Model)
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub fruits: Vec<super::fruit::Model>,
        /// HasMany via relation - loaded fillings (uses Filling Model)
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub fillings: Vec<super::filling::Model>,
    }
}

/// Expected models for Fruit entity
pub mod expected_fruit {
    use serde::{Deserialize, Serialize};

    /// Expected FruitCreate - excludes id (primary key), cake relation is via FK
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct FruitCreate {
        pub name: String,
        /// FK for belongs_to relation
        pub cake_id: Option<i32>,
    }

    /// Expected FruitUpdate - Option<Option<T>> pattern
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
    pub struct FruitUpdate {
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "::serde_with::rust::double_option"
        )]
        pub name: Option<Option<String>>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "::serde_with::rust::double_option"
        )]
        pub cake_id: Option<Option<i32>>,
    }

    /// Expected FruitList - all db columns, excludes relation fields
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct FruitList {
        pub id: i32,
        pub name: String,
        pub cake_id: Option<i32>,
    }

    /// Expected FruitResponse - includes belongs_to relation data
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct FruitResponse {
        pub id: i32,
        pub name: String,
        pub cake_id: Option<i32>,
        /// BelongsTo relation - loaded cake (uses Cake Model, boxed to avoid circular reference)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub cake: Option<Box<super::cake::Model>>,
    }
}

/// Expected models for Filling entity
pub mod expected_filling {
    use serde::{Deserialize, Serialize};

    /// Expected FillingCreate - excludes id (primary key)
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct FillingCreate {
        pub name: String,
    }

    /// Expected FillingUpdate - Option<Option<T>> pattern
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
    pub struct FillingUpdate {
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "::serde_with::rust::double_option"
        )]
        pub name: Option<Option<String>>,
    }

    /// Expected FillingList - all db columns, excludes relation fields for efficiency
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct FillingList {
        pub id: i32,
        pub name: String,
    }

    /// Expected FillingResponse - includes has_many via relation data
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct FillingResponse {
        pub id: i32,
        pub name: String,
        /// HasMany via relation - loaded cakes (uses Cake Model)
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub cakes: Vec<super::cake::Model>,
    }
}

/// Expected models for CakeFilling junction table entity
pub mod expected_cake_filling {
    use serde::{Deserialize, Serialize};

    /// Expected CakeFillingCreate - composite PK fields included (they are FKs)
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct CakeFillingCreate {
        pub cake_id: i32,
        pub filling_id: i32,
    }

    /// Expected CakeFillingUpdate - empty since both fields are PKs (excluded from update)
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
    pub struct CakeFillingUpdate {}

    /// Expected CakeFillingList - all db columns, excludes relation fields for efficiency
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct CakeFillingList {
        pub cake_id: i32,
        pub filling_id: i32,
    }

    /// Expected CakeFillingResponse - includes belongs_to relation data
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct CakeFillingResponse {
        pub cake_id: i32,
        pub filling_id: i32,
        /// BelongsTo relation - loaded cake (uses Cake Model)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub cake: Option<Box<super::cake::Model>>,
        /// BelongsTo relation - loaded filling (uses Filling Model)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub filling: Option<Box<super::filling::Model>>,
    }
}

// ============================================================================
// Bakery Model Definitions
// ============================================================================

/// Cake entity - has many Fruits and many Fillings via junction table
pub mod cake {
    use super::*;

    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, ToCreateModel, ToUpdateModel, ToListModel, ToResponseModel)]
    #[sea_orm(table_name = "cake")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        #[sea_orm(column_type = "Text", nullable)]
        pub name: Option<String>,
        #[sea_orm(has_many)]
        pub fruits: HasMany<super::fruit::Entity>,
        #[sea_orm(has_many, via = "cake_filling")]
        pub fillings: HasMany<super::filling::Entity>,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// Fruit entity - belongs to Cake
pub mod fruit {
    use super::*;

    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, ToCreateModel, ToUpdateModel, ToListModel)]
    #[sea_orm(table_name = "fruit")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub name: String,
        pub cake_id: Option<i32>,
        #[sea_orm(belongs_to, from = "cake_id", to = "id")]
        pub cake: HasOne<super::cake::Entity>,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// Filling entity - has many Cakes via junction table
pub mod filling {
    use super::*;

    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, ToCreateModel, ToUpdateModel, ToListModel)]
    #[sea_orm(table_name = "filling")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub name: String,
        #[sea_orm(has_many, via = "cake_filling")]
        pub cakes: HasMany<super::cake::Entity>,
    }

    impl ActiveModelBehavior for ActiveModel {}

}

/// CakeFilling junction table - links Cake and Filling
pub mod cake_filling {
    use super::*;

    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, ToCreateModel, ToUpdateModel, ToListModel)]
    #[sea_orm(table_name = "cake_filling")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub cake_id: i32,
        #[sea_orm(primary_key, auto_increment = false)]
        pub filling_id: i32,
        #[sea_orm(
            belongs_to,
            from = "cake_id",
            to = "id",
            on_update = "Cascade",
            on_delete = "Cascade"
        )]
        pub cake: HasOne<super::cake::Entity>,
        #[sea_orm(
            belongs_to,
            from = "filling_id",
            to = "id",
            on_update = "Cascade",
            on_delete = "Cascade"
        )]
        pub filling: HasOne<super::filling::Entity>,
    }

    impl ActiveModelBehavior for ActiveModel {}

    
}

// ============================================================================
// Create Model Tests
// ============================================================================

#[test]
fn test_cake_create_model_exists() {
    // Verify CakeCreate struct was generated with correct fields
    // id is excluded (has on_create or primary_key), relation fields are excluded
    let _create: cake::CakeCreate = cake::CakeCreate {
        name: Some("Chocolate Cake".to_string()),
    };
}

#[test]
fn test_cake_create_with_null_name() {
    // Cake name is nullable - can be created with None
    let _create: cake::CakeCreate = cake::CakeCreate {
        name: None,
    };
}

#[test]
fn test_fruit_create_model_exists() {
    // Verify FruitCreate struct was generated
    // id is excluded, cake_id (FK) is included
    let _create: fruit::FruitCreate = fruit::FruitCreate {
        name: "Apple".to_string(),
        cake_id: Some(1),
    };
}

#[test]
fn test_fruit_create_without_cake() {
    // Fruit can be created without a cake (cake_id is optional)
    let _create: fruit::FruitCreate = fruit::FruitCreate {
        name: "Banana".to_string(),
        cake_id: None,
    };
}

#[test]
fn test_filling_create_model_exists() {
    // Verify FillingCreate struct was generated
    // id is excluded, relation fields are excluded
    let _create: filling::FillingCreate = filling::FillingCreate {
        name: "Strawberry".to_string(),
    };
}

#[test]
fn test_cake_filling_create_model_exists() {
    // Verify CakeFillingCreate struct was generated
    // Both cake_id and filling_id are included as they form composite primary key
    let _create: cake_filling::CakeFillingCreate = cake_filling::CakeFillingCreate {
        cake_id: 1,
        filling_id: 2,
    };
}

// ============================================================================
// Update Model Tests
// ============================================================================

#[test]
fn test_cake_update_model_exists() {
    // Verify CakeUpdate struct was generated with Option<Option<T>> pattern
    let _update: cake::CakeUpdate = cake::CakeUpdate {
        name: Some(Some("Updated Cake Name".to_string())),
    };
}

#[test]
fn test_cake_update_set_to_null() {
    // Setting name to None (null) in update
    let _update: cake::CakeUpdate = cake::CakeUpdate {
        name: Some(None),
    };
}

#[test]
fn test_cake_update_no_change() {
    // Not updating name at all
    let _update: cake::CakeUpdate = cake::CakeUpdate {
        name: None,
    };
}

#[test]
fn test_fruit_update_model_exists() {
    // Verify FruitUpdate struct was generated
    let _update: fruit::FruitUpdate = fruit::FruitUpdate {
        name: Some(Some("Updated Fruit".to_string())),
        cake_id: Some(Some(2)),
    };
}

#[test]
fn test_fruit_update_remove_cake() {
    // Remove fruit from cake (set cake_id to null)
    let _update: fruit::FruitUpdate = fruit::FruitUpdate {
        name: None,
        cake_id: Some(None),
    };
}

#[test]
fn test_filling_update_model_exists() {
    // Verify FillingUpdate struct was generated
    let _update: filling::FillingUpdate = filling::FillingUpdate {
        name: Some(Some("Updated Filling".to_string())),
    };
}

#[test]
fn test_cake_filling_update_model_exists() {
    // CakeFilling has no updatable fields (both PKs are excluded from update)
    // The update model should exist but be empty or minimal
    let _update: cake_filling::CakeFillingUpdate = cake_filling::CakeFillingUpdate {};
}

// ============================================================================
// List Model Tests
// ============================================================================

#[test]
fn test_cake_list_model_exists() {
    // Verify CakeList struct was generated
    let _list: cake::CakeList = cake::CakeList {
        id: 1,
        name: Some("Cake".to_string()),
    };
}

#[test]
fn test_fruit_list_model_exists() {
    // Verify FruitList struct was generated
    let _list: fruit::FruitList = fruit::FruitList {
        id: 1,
        name: "Fruit".to_string(),
        cake_id: Some(1),
    };
}

#[test]
fn test_filling_list_model_exists() {
    // Verify FillingList struct was generated
    let _list: filling::FillingList = filling::FillingList {
        id: 1,
        name: "Filling".to_string(),
    };
}

#[test]
fn test_cake_filling_list_model_exists() {
    // Verify CakeFillingList struct was generated
    let _list: cake_filling::CakeFillingList = cake_filling::CakeFillingList {
        cake_id: 1,
        filling_id: 2,
    };
}

// ============================================================================
// Response Model Tests
// ============================================================================

#[test]
fn test_cake_response_model_exists() {
    // Verify CakeResponse struct was generated
    let _response: cake::CakeResponse = cake::CakeResponse {
        id: 1,
        name: Some("Cake".to_string()),
    };
}

#[test]
fn test_fruit_response_model_exists() {
    // Verify FruitResponse struct was generated
    let _response: fruit::FruitResponse = fruit::FruitResponse {
        id: 1,
        name: "Fruit".to_string(),
        cake_id: Some(1),
    };
}

#[test]
fn test_filling_response_model_exists() {
    // Verify FillingResponse struct was generated
    let _response: filling::FillingResponse = filling::FillingResponse {
        id: 1,
        name: "Filling".to_string(),
    };
}

#[test]
fn test_cake_filling_response_model_exists() {
    // Verify CakeFillingResponse struct was generated
    let _response: cake_filling::CakeFillingResponse = cake_filling::CakeFillingResponse {
        cake_id: 1,
        filling_id: 2,
    };
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_cake_create_serialization() {
    let create = cake::CakeCreate {
        name: Some("Chocolate Dream".to_string()),
    };

    let json = serde_json::to_string(&create).expect("Failed to serialize CakeCreate");
    let parsed: cake::CakeCreate = serde_json::from_str(&json).expect("Failed to deserialize CakeCreate");

    assert_eq!(parsed.name, create.name);
}

#[test]
fn test_cake_create_from_json() {
    let json = r#"{"name": "Vanilla Supreme"}"#;
    let create: cake::CakeCreate = serde_json::from_str(json).expect("Failed to parse CakeCreate");

    assert_eq!(create.name, Some("Vanilla Supreme".to_string()));
}

#[test]
fn test_cake_create_from_json_with_null_name() {
    let json = r#"{"name": null}"#;
    let create: cake::CakeCreate = serde_json::from_str(json).expect("Failed to parse CakeCreate");

    assert_eq!(create.name, None);
}

#[test]
fn test_fruit_create_serialization() {
    let create = fruit::FruitCreate {
        name: "Strawberry".to_string(),
        cake_id: Some(5),
    };

    let json = serde_json::to_string(&create).expect("Failed to serialize FruitCreate");
    let parsed: fruit::FruitCreate = serde_json::from_str(&json).expect("Failed to deserialize FruitCreate");

    assert_eq!(parsed.name, create.name);
    assert_eq!(parsed.cake_id, create.cake_id);
}

#[test]
fn test_fruit_create_from_json() {
    let json = r#"{"name": "Blueberry", "cake_id": 3}"#;
    let create: fruit::FruitCreate = serde_json::from_str(json).expect("Failed to parse FruitCreate");

    assert_eq!(create.name, "Blueberry");
    assert_eq!(create.cake_id, Some(3));
}

#[test]
fn test_fruit_create_from_json_without_cake() {
    let json = r#"{"name": "Raspberry"}"#;
    let create: fruit::FruitCreate = serde_json::from_str(json).expect("Failed to parse FruitCreate");

    assert_eq!(create.name, "Raspberry");
    assert_eq!(create.cake_id, None);
}

#[test]
fn test_filling_create_serialization() {
    let create = filling::FillingCreate {
        name: "Cream".to_string(),
    };

    let json = serde_json::to_string(&create).expect("Failed to serialize FillingCreate");
    let parsed: filling::FillingCreate = serde_json::from_str(&json).expect("Failed to deserialize FillingCreate");

    assert_eq!(parsed.name, create.name);
}

#[test]
fn test_cake_filling_create_serialization() {
    let create = cake_filling::CakeFillingCreate {
        cake_id: 10,
        filling_id: 20,
    };

    let json = serde_json::to_string(&create).expect("Failed to serialize CakeFillingCreate");
    let parsed: cake_filling::CakeFillingCreate = serde_json::from_str(&json).expect("Failed to deserialize CakeFillingCreate");

    assert_eq!(parsed.cake_id, create.cake_id);
    assert_eq!(parsed.filling_id, create.filling_id);
}

#[test]
fn test_cake_update_serialization() {
    let update = cake::CakeUpdate {
        name: Some(Some("Updated Name".to_string())),
    };

    let json = serde_json::to_string(&update).expect("Failed to serialize CakeUpdate");
    let parsed: cake::CakeUpdate = serde_json::from_str(&json).expect("Failed to deserialize CakeUpdate");

    assert_eq!(parsed.name, update.name);
}

#[test]
fn test_cake_update_from_partial_json() {
    // Update with only name field present
    let json = r#"{"name": "New Name"}"#;
    let update: cake::CakeUpdate = serde_json::from_str(json).expect("Failed to parse CakeUpdate");

    // With double_option serde, "New Name" becomes Some(Some("New Name"))
    assert_eq!(update.name, Some(Some("New Name".to_string())));
}

#[test]
fn test_cake_update_from_json_set_null() {
    // Setting name to null in update
    let json = r#"{"name": null}"#;
    let update: cake::CakeUpdate = serde_json::from_str(json).expect("Failed to parse CakeUpdate");

    // With double_option serde, null becomes Some(None)
    assert_eq!(update.name, Some(None));
}

#[test]
fn test_fruit_update_serialization() {
    let update = fruit::FruitUpdate {
        name: Some(Some("Cherry".to_string())),
        cake_id: Some(Some(7)),
    };

    let json = serde_json::to_string(&update).expect("Failed to serialize FruitUpdate");
    let parsed: fruit::FruitUpdate = serde_json::from_str(&json).expect("Failed to deserialize FruitUpdate");

    assert_eq!(parsed.name, update.name);
    assert_eq!(parsed.cake_id, update.cake_id);
}

// ============================================================================
// Response Serialization Tests
// ============================================================================

#[test]
fn test_cake_response_serialization() {
    let response = cake::CakeResponse {
        id: 42,
        name: Some("Special Cake".to_string()),
    };

    let json = serde_json::to_string(&response).expect("Failed to serialize CakeResponse");
    let parsed: cake::CakeResponse = serde_json::from_str(&json).expect("Failed to deserialize CakeResponse");

    assert_eq!(parsed.id, response.id);
    assert_eq!(parsed.name, response.name);
}

#[test]
fn test_fruit_response_serialization() {
    let response = fruit::FruitResponse {
        id: 100,
        name: "Mango".to_string(),
        cake_id: Some(42),
    };

    let json = serde_json::to_string(&response).expect("Failed to serialize FruitResponse");
    let parsed: fruit::FruitResponse = serde_json::from_str(&json).expect("Failed to deserialize FruitResponse");

    assert_eq!(parsed.id, response.id);
    assert_eq!(parsed.name, response.name);
    assert_eq!(parsed.cake_id, response.cake_id);
}

// ============================================================================
// List Serialization Tests
// ============================================================================

#[test]
fn test_cake_list_serialization() {
    let list = cake::CakeList {
        id: 1,
        name: Some("Cake for List".to_string()),
    };

    let json = serde_json::to_string(&list).expect("Failed to serialize CakeList");
    let parsed: cake::CakeList = serde_json::from_str(&json).expect("Failed to deserialize CakeList");

    assert_eq!(parsed.id, list.id);
    assert_eq!(parsed.name, list.name);
}

#[test]
fn test_fruit_list_serialization() {
    let list = fruit::FruitList {
        id: 2,
        name: "Fruit for List".to_string(),
        cake_id: None,
    };

    let json = serde_json::to_string(&list).expect("Failed to serialize FruitList");
    let parsed: fruit::FruitList = serde_json::from_str(&json).expect("Failed to deserialize FruitList");

    assert_eq!(parsed.id, list.id);
    assert_eq!(parsed.name, list.name);
    assert_eq!(parsed.cake_id, list.cake_id);
}

// ============================================================================
// Conversion Tests (Create -> ActiveModel)
// ============================================================================

#[test]
fn test_cake_create_to_active_model() {
    let create = cake::CakeCreate {
        name: Some("Birthday Cake".to_string()),
    };

    let active_model: cake::ActiveModel = create.into();

    match &active_model.name {
        sea_orm::ActiveValue::Set(name) => assert_eq!(name, &Some("Birthday Cake".to_string())),
        _ => panic!("Expected name to be Set"),
    }
}

#[test]
fn test_cake_create_to_active_model_with_null_name() {
    let create = cake::CakeCreate {
        name: None,
    };

    let active_model: cake::ActiveModel = create.into();

    match &active_model.name {
        sea_orm::ActiveValue::Set(name) => assert_eq!(name, &None),
        _ => panic!("Expected name to be Set to None"),
    }
}

#[test]
fn test_fruit_create_to_active_model() {
    let create = fruit::FruitCreate {
        name: "Peach".to_string(),
        cake_id: Some(12),
    };

    let active_model: fruit::ActiveModel = create.into();

    match &active_model.name {
        sea_orm::ActiveValue::Set(name) => assert_eq!(name, "Peach"),
        _ => panic!("Expected name to be Set"),
    }

    match &active_model.cake_id {
        sea_orm::ActiveValue::Set(cake_id) => assert_eq!(cake_id, &Some(12)),
        _ => panic!("Expected cake_id to be Set"),
    }
}

#[test]
fn test_fruit_create_to_active_model_without_cake() {
    let create = fruit::FruitCreate {
        name: "Grape".to_string(),
        cake_id: None,
    };

    let active_model: fruit::ActiveModel = create.into();

    match &active_model.cake_id {
        sea_orm::ActiveValue::Set(cake_id) => assert_eq!(cake_id, &None),
        _ => panic!("Expected cake_id to be Set to None"),
    }
}

#[test]
fn test_filling_create_to_active_model() {
    let create = filling::FillingCreate {
        name: "Chocolate Mousse".to_string(),
    };

    let active_model: filling::ActiveModel = create.into();

    match &active_model.name {
        sea_orm::ActiveValue::Set(name) => assert_eq!(name, "Chocolate Mousse"),
        _ => panic!("Expected name to be Set"),
    }
}

#[test]
fn test_cake_filling_create_to_active_model() {
    let create = cake_filling::CakeFillingCreate {
        cake_id: 5,
        filling_id: 10,
    };

    let active_model: cake_filling::ActiveModel = create.into();

    match &active_model.cake_id {
        sea_orm::ActiveValue::Set(cake_id) => assert_eq!(cake_id, &5),
        _ => panic!("Expected cake_id to be Set"),
    }

    match &active_model.filling_id {
        sea_orm::ActiveValue::Set(filling_id) => assert_eq!(filling_id, &10),
        _ => panic!("Expected filling_id to be Set"),
    }
}

// ============================================================================
// Type Constraint Tests
// ============================================================================

fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}
fn assert_debug<T: std::fmt::Debug>() {}
fn assert_clone<T: Clone>() {}
fn assert_partial_eq<T: PartialEq>() {}

#[test]
fn test_generated_types_are_send() {
    assert_send::<cake::CakeCreate>();
    assert_send::<cake::CakeUpdate>();
    assert_send::<cake::CakeResponse>();
    assert_send::<cake::CakeList>();

    assert_send::<fruit::FruitCreate>();
    assert_send::<fruit::FruitUpdate>();
    assert_send::<fruit::FruitResponse>();
    assert_send::<fruit::FruitList>();

    assert_send::<filling::FillingCreate>();
    assert_send::<filling::FillingUpdate>();
    assert_send::<filling::FillingResponse>();
    assert_send::<filling::FillingList>();

    assert_send::<cake_filling::CakeFillingCreate>();
    assert_send::<cake_filling::CakeFillingUpdate>();
    assert_send::<cake_filling::CakeFillingResponse>();
    assert_send::<cake_filling::CakeFillingList>();
}

#[test]
fn test_generated_types_are_sync() {
    assert_sync::<cake::CakeCreate>();
    assert_sync::<cake::CakeUpdate>();
    assert_sync::<cake::CakeResponse>();
    assert_sync::<cake::CakeList>();

    assert_sync::<fruit::FruitCreate>();
    assert_sync::<fruit::FruitUpdate>();
    assert_sync::<fruit::FruitResponse>();
    assert_sync::<fruit::FruitList>();

    assert_sync::<filling::FillingCreate>();
    assert_sync::<filling::FillingUpdate>();
    assert_sync::<filling::FillingResponse>();
    assert_sync::<filling::FillingList>();

    assert_sync::<cake_filling::CakeFillingCreate>();
    assert_sync::<cake_filling::CakeFillingUpdate>();
    assert_sync::<cake_filling::CakeFillingResponse>();
    assert_sync::<cake_filling::CakeFillingList>();
}

#[test]
fn test_generated_types_are_debug() {
    assert_debug::<cake::CakeCreate>();
    assert_debug::<cake::CakeUpdate>();
    assert_debug::<cake::CakeResponse>();
    assert_debug::<cake::CakeList>();

    assert_debug::<fruit::FruitCreate>();
    assert_debug::<fruit::FruitUpdate>();
    assert_debug::<fruit::FruitResponse>();
    assert_debug::<fruit::FruitList>();

    assert_debug::<filling::FillingCreate>();
    assert_debug::<filling::FillingUpdate>();
    assert_debug::<filling::FillingResponse>();
    assert_debug::<filling::FillingList>();

    assert_debug::<cake_filling::CakeFillingCreate>();
    assert_debug::<cake_filling::CakeFillingUpdate>();
    assert_debug::<cake_filling::CakeFillingResponse>();
    assert_debug::<cake_filling::CakeFillingList>();
}

#[test]
fn test_generated_types_are_clone() {
    assert_clone::<cake::CakeCreate>();
    assert_clone::<cake::CakeUpdate>();
    assert_clone::<cake::CakeResponse>();
    assert_clone::<cake::CakeList>();

    assert_clone::<fruit::FruitCreate>();
    assert_clone::<fruit::FruitUpdate>();
    assert_clone::<fruit::FruitResponse>();
    assert_clone::<fruit::FruitList>();

    assert_clone::<filling::FillingCreate>();
    assert_clone::<filling::FillingUpdate>();
    assert_clone::<filling::FillingResponse>();
    assert_clone::<filling::FillingList>();

    assert_clone::<cake_filling::CakeFillingCreate>();
    assert_clone::<cake_filling::CakeFillingUpdate>();
    assert_clone::<cake_filling::CakeFillingResponse>();
    assert_clone::<cake_filling::CakeFillingList>();
}

#[test]
fn test_generated_types_are_partial_eq() {
    // derive_partial_eq is enabled for all models
    assert_partial_eq::<cake::CakeCreate>();
    assert_partial_eq::<cake::CakeUpdate>();
    assert_partial_eq::<cake::CakeResponse>();
    assert_partial_eq::<cake::CakeList>();

    assert_partial_eq::<fruit::FruitCreate>();
    assert_partial_eq::<fruit::FruitUpdate>();
    assert_partial_eq::<fruit::FruitResponse>();
    assert_partial_eq::<fruit::FruitList>();

    assert_partial_eq::<filling::FillingCreate>();
    assert_partial_eq::<filling::FillingUpdate>();
    assert_partial_eq::<filling::FillingResponse>();
    assert_partial_eq::<filling::FillingList>();

    assert_partial_eq::<cake_filling::CakeFillingCreate>();
    assert_partial_eq::<cake_filling::CakeFillingUpdate>();
    assert_partial_eq::<cake_filling::CakeFillingResponse>();
    assert_partial_eq::<cake_filling::CakeFillingList>();
}

// ============================================================================
// Semantic Relationship Tests
// ============================================================================

#[test]
fn test_one_to_many_cake_fruits_semantic() {
    // A Cake can have many Fruits
    // Fruit references Cake via cake_id
    let _fruit1 = fruit::FruitCreate {
        name: "Apple".to_string(),
        cake_id: Some(1),
    };

    let _fruit2 = fruit::FruitCreate {
        name: "Cherry".to_string(),
        cake_id: Some(1), // Same cake
    };

    let _fruit3 = fruit::FruitCreate {
        name: "Banana".to_string(),
        cake_id: Some(1), // Same cake
    };
}

#[test]
fn test_many_to_many_cake_filling_semantic() {
    // Cakes and Fillings have many-to-many relationship via CakeFilling junction
    // Create links between cakes and fillings

    // Cake 1 has Filling 1 and Filling 2
    let _link1 = cake_filling::CakeFillingCreate {
        cake_id: 1,
        filling_id: 1,
    };
    let _link2 = cake_filling::CakeFillingCreate {
        cake_id: 1,
        filling_id: 2,
    };

    // Cake 2 also has Filling 1 (shared filling)
    let _link3 = cake_filling::CakeFillingCreate {
        cake_id: 2,
        filling_id: 1,
    };
}

#[test]
fn test_belongs_to_fruit_cake_semantic() {
    // Fruit belongs to Cake - the FK (cake_id) is optional
    let _orphan_fruit = fruit::FruitCreate {
        name: "Orphan Berry".to_string(),
        cake_id: None,
    };

    let _cake_fruit = fruit::FruitCreate {
        name: "Cake Fruit".to_string(),
        cake_id: Some(5),
    };
}

// ============================================================================
// Model Equality Tests (with derive_partial_eq)
// ============================================================================

#[test]
fn test_cake_create_equality() {
    let create1 = cake::CakeCreate {
        name: Some("Test".to_string()),
    };
    let create2 = cake::CakeCreate {
        name: Some("Test".to_string()),
    };
    let create3 = cake::CakeCreate {
        name: Some("Different".to_string()),
    };

    assert_eq!(create1, create2);
    assert_ne!(create1, create3);
}

#[test]
fn test_fruit_create_equality() {
    let create1 = fruit::FruitCreate {
        name: "Apple".to_string(),
        cake_id: Some(1),
    };
    let create2 = fruit::FruitCreate {
        name: "Apple".to_string(),
        cake_id: Some(1),
    };
    let create3 = fruit::FruitCreate {
        name: "Apple".to_string(),
        cake_id: Some(2),
    };

    assert_eq!(create1, create2);
    assert_ne!(create1, create3);
}

#[test]
fn test_cake_response_equality() {
    let resp1 = cake::CakeResponse {
        id: 1,
        name: Some("Cake".to_string()),
    };
    let resp2 = cake::CakeResponse {
        id: 1,
        name: Some("Cake".to_string()),
    };
    let resp3 = cake::CakeResponse {
        id: 2,
        name: Some("Cake".to_string()),
    };

    assert_eq!(resp1, resp2);
    assert_ne!(resp1, resp3);
}

#[test]
fn test_cake_list_equality() {
    let list1 = cake::CakeList {
        id: 1,
        name: Some("Cake".to_string()),
    };
    let list2 = cake::CakeList {
        id: 1,
        name: Some("Cake".to_string()),
    };
    let list3 = cake::CakeList {
        id: 1,
        name: None,
    };

    assert_eq!(list1, list2);
    assert_ne!(list1, list3);
}

// ============================================================================
// Comparison Tests: Generated vs Expected Models
// These tests verify that the derive macro generates models matching our expectations
// ============================================================================

/// Compare generated CakeCreate with expected CakeCreate via JSON serialization
#[test]
fn test_cake_create_matches_expected() {
    // Create instances with same data
    let generated = cake::CakeCreate {
        name: Some("Test Cake".to_string()),
    };
    let expected = expected_cake::CakeCreate {
        name: Some("Test Cake".to_string()),
    };

    // Serialize both
    let generated_json = serde_json::to_value(&generated).expect("Failed to serialize generated");
    let expected_json = serde_json::to_value(&expected).expect("Failed to serialize expected");

    // Compare JSON representations
    assert_eq!(generated_json, expected_json, "CakeCreate JSON mismatch");

    // Test with null name
    let generated_null = cake::CakeCreate { name: None };
    let expected_null = expected_cake::CakeCreate { name: None };
    assert_eq!(
        serde_json::to_value(&generated_null).unwrap(),
        serde_json::to_value(&expected_null).unwrap(),
        "CakeCreate with null name JSON mismatch"
    );
}

/// Compare generated CakeUpdate with expected CakeUpdate via JSON serialization
#[test]
fn test_cake_update_matches_expected() {
    // Test with value set
    let generated = cake::CakeUpdate {
        name: Some(Some("Updated".to_string())),
    };
    let expected = expected_cake::CakeUpdate {
        name: Some(Some("Updated".to_string())),
    };

    let generated_json = serde_json::to_value(&generated).expect("Failed to serialize generated");
    let expected_json = serde_json::to_value(&expected).expect("Failed to serialize expected");
    assert_eq!(generated_json, expected_json, "CakeUpdate JSON mismatch");

    // Test with null (set to null)
    let generated_null = cake::CakeUpdate { name: Some(None) };
    let expected_null = expected_cake::CakeUpdate { name: Some(None) };
    assert_eq!(
        serde_json::to_value(&generated_null).unwrap(),
        serde_json::to_value(&expected_null).unwrap(),
        "CakeUpdate set-to-null JSON mismatch"
    );

    // Test with no change (None)
    let generated_no_change = cake::CakeUpdate { name: None };
    let expected_no_change = expected_cake::CakeUpdate { name: None };
    assert_eq!(
        serde_json::to_value(&generated_no_change).unwrap(),
        serde_json::to_value(&expected_no_change).unwrap(),
        "CakeUpdate no-change JSON mismatch"
    );
}

/// Compare generated CakeList with expected CakeList via JSON serialization
#[test]
fn test_cake_list_matches_expected() {
    let generated = cake::CakeList {
        id: 42,
        name: Some("List Cake".to_string()),
    };
    let expected = expected_cake::CakeList {
        id: 42,
        name: Some("List Cake".to_string()),
    };

    let generated_json = serde_json::to_value(&generated).expect("Failed to serialize generated");
    let expected_json = serde_json::to_value(&expected).expect("Failed to serialize expected");
    assert_eq!(generated_json, expected_json, "CakeList JSON mismatch");
}

/// Compare generated CakeResponse with expected CakeResponse via JSON serialization
#[test]
fn test_cake_response_matches_expected() {
    let generated = cake::CakeResponse {
        id: 42,
        name: Some("Response Cake".to_string()),
    };
    let expected = expected_cake::CakeResponse {
        id: 42,
        name: Some("Response Cake".to_string()),
    };

    let generated_json = serde_json::to_value(&generated).expect("Failed to serialize generated");
    let expected_json = serde_json::to_value(&expected).expect("Failed to serialize expected");
    assert_eq!(generated_json, expected_json, "CakeResponse JSON mismatch");
}

/// Compare generated FruitCreate with expected FruitCreate via JSON serialization
#[test]
fn test_fruit_create_matches_expected() {
    let generated = fruit::FruitCreate {
        name: "Apple".to_string(),
        cake_id: Some(5),
    };
    let expected = expected_fruit::FruitCreate {
        name: "Apple".to_string(),
        cake_id: Some(5),
    };

    let generated_json = serde_json::to_value(&generated).expect("Failed to serialize generated");
    let expected_json = serde_json::to_value(&expected).expect("Failed to serialize expected");
    assert_eq!(generated_json, expected_json, "FruitCreate JSON mismatch");

    // Test without cake_id
    let generated_no_cake = fruit::FruitCreate {
        name: "Banana".to_string(),
        cake_id: None,
    };
    let expected_no_cake = expected_fruit::FruitCreate {
        name: "Banana".to_string(),
        cake_id: None,
    };
    assert_eq!(
        serde_json::to_value(&generated_no_cake).unwrap(),
        serde_json::to_value(&expected_no_cake).unwrap(),
        "FruitCreate without cake_id JSON mismatch"
    );
}

/// Compare generated FruitUpdate with expected FruitUpdate via JSON serialization
#[test]
fn test_fruit_update_matches_expected() {
    let generated = fruit::FruitUpdate {
        name: Some(Some("Updated Fruit".to_string())),
        cake_id: Some(Some(10)),
    };
    let expected = expected_fruit::FruitUpdate {
        name: Some(Some("Updated Fruit".to_string())),
        cake_id: Some(Some(10)),
    };

    let generated_json = serde_json::to_value(&generated).expect("Failed to serialize generated");
    let expected_json = serde_json::to_value(&expected).expect("Failed to serialize expected");
    assert_eq!(generated_json, expected_json, "FruitUpdate JSON mismatch");

    // Test setting cake_id to null
    let generated_remove_cake = fruit::FruitUpdate {
        name: None,
        cake_id: Some(None),
    };
    let expected_remove_cake = expected_fruit::FruitUpdate {
        name: None,
        cake_id: Some(None),
    };
    assert_eq!(
        serde_json::to_value(&generated_remove_cake).unwrap(),
        serde_json::to_value(&expected_remove_cake).unwrap(),
        "FruitUpdate remove cake JSON mismatch"
    );
}

/// Compare generated FruitList with expected FruitList via JSON serialization
#[test]
fn test_fruit_list_matches_expected() {
    let generated = fruit::FruitList {
        id: 1,
        name: "Cherry".to_string(),
        cake_id: Some(3),
    };
    let expected = expected_fruit::FruitList {
        id: 1,
        name: "Cherry".to_string(),
        cake_id: Some(3),
    };

    let generated_json = serde_json::to_value(&generated).expect("Failed to serialize generated");
    let expected_json = serde_json::to_value(&expected).expect("Failed to serialize expected");
    assert_eq!(generated_json, expected_json, "FruitList JSON mismatch");
}

/// Compare generated FruitResponse with expected FruitResponse via JSON serialization
#[test]
fn test_fruit_response_matches_expected() {
    let generated = fruit::FruitResponse {
        id: 1,
        name: "Mango".to_string(),
        cake_id: None,
    };
    let expected = expected_fruit::FruitResponse {
        id: 1,
        name: "Mango".to_string(),
        cake_id: None,
    };

    let generated_json = serde_json::to_value(&generated).expect("Failed to serialize generated");
    let expected_json = serde_json::to_value(&expected).expect("Failed to serialize expected");
    assert_eq!(generated_json, expected_json, "FruitResponse JSON mismatch");
}

/// Compare generated FillingCreate with expected FillingCreate via JSON serialization
#[test]
fn test_filling_create_matches_expected() {
    let generated = filling::FillingCreate {
        name: "Chocolate".to_string(),
    };
    let expected = expected_filling::FillingCreate {
        name: "Chocolate".to_string(),
    };

    let generated_json = serde_json::to_value(&generated).expect("Failed to serialize generated");
    let expected_json = serde_json::to_value(&expected).expect("Failed to serialize expected");
    assert_eq!(generated_json, expected_json, "FillingCreate JSON mismatch");
}

/// Compare generated FillingUpdate with expected FillingUpdate via JSON serialization
#[test]
fn test_filling_update_matches_expected() {
    let generated = filling::FillingUpdate {
        name: Some(Some("Vanilla".to_string())),
    };
    let expected = expected_filling::FillingUpdate {
        name: Some(Some("Vanilla".to_string())),
    };

    let generated_json = serde_json::to_value(&generated).expect("Failed to serialize generated");
    let expected_json = serde_json::to_value(&expected).expect("Failed to serialize expected");
    assert_eq!(generated_json, expected_json, "FillingUpdate JSON mismatch");
}

/// Compare generated FillingList with expected FillingList via JSON serialization
#[test]
fn test_filling_list_matches_expected() {
    let generated = filling::FillingList {
        id: 7,
        name: "Cream".to_string(),
    };
    let expected = expected_filling::FillingList {
        id: 7,
        name: "Cream".to_string(),
    };

    let generated_json = serde_json::to_value(&generated).expect("Failed to serialize generated");
    let expected_json = serde_json::to_value(&expected).expect("Failed to serialize expected");
    assert_eq!(generated_json, expected_json, "FillingList JSON mismatch");
}

/// Compare generated FillingResponse with expected FillingResponse via JSON serialization
#[test]
fn test_filling_response_matches_expected() {
    let generated = filling::FillingResponse {
        id: 7,
        name: "Strawberry".to_string(),
    };
    let expected = expected_filling::FillingResponse {
        id: 7,
        name: "Strawberry".to_string(),
    };

    let generated_json = serde_json::to_value(&generated).expect("Failed to serialize generated");
    let expected_json = serde_json::to_value(&expected).expect("Failed to serialize expected");
    assert_eq!(generated_json, expected_json, "FillingResponse JSON mismatch");
}

/// Compare generated CakeFillingCreate with expected CakeFillingCreate via JSON serialization
#[test]
fn test_cake_filling_create_matches_expected() {
    let generated = cake_filling::CakeFillingCreate {
        cake_id: 1,
        filling_id: 2,
    };
    let expected = expected_cake_filling::CakeFillingCreate {
        cake_id: 1,
        filling_id: 2,
    };

    let generated_json = serde_json::to_value(&generated).expect("Failed to serialize generated");
    let expected_json = serde_json::to_value(&expected).expect("Failed to serialize expected");
    assert_eq!(generated_json, expected_json, "CakeFillingCreate JSON mismatch");
}

/// Compare generated CakeFillingUpdate with expected CakeFillingUpdate via JSON serialization
#[test]
fn test_cake_filling_update_matches_expected() {
    let generated = cake_filling::CakeFillingUpdate {};
    let expected = expected_cake_filling::CakeFillingUpdate {};

    let generated_json = serde_json::to_value(&generated).expect("Failed to serialize generated");
    let expected_json = serde_json::to_value(&expected).expect("Failed to serialize expected");
    assert_eq!(generated_json, expected_json, "CakeFillingUpdate JSON mismatch");
}

/// Compare generated CakeFillingList with expected CakeFillingList via JSON serialization
#[test]
fn test_cake_filling_list_matches_expected() {
    let generated = cake_filling::CakeFillingList {
        cake_id: 3,
        filling_id: 4,
    };
    let expected = expected_cake_filling::CakeFillingList {
        cake_id: 3,
        filling_id: 4,
    };

    let generated_json = serde_json::to_value(&generated).expect("Failed to serialize generated");
    let expected_json = serde_json::to_value(&expected).expect("Failed to serialize expected");
    assert_eq!(generated_json, expected_json, "CakeFillingList JSON mismatch");
}

/// Compare generated CakeFillingResponse with expected CakeFillingResponse via JSON serialization
#[test]
fn test_cake_filling_response_matches_expected() {
    let generated = cake_filling::CakeFillingResponse {
        cake_id: 3,
        filling_id: 4,
    };
    let expected = expected_cake_filling::CakeFillingResponse {
        cake_id: 3,
        filling_id: 4,
    };

    let generated_json = serde_json::to_value(&generated).expect("Failed to serialize generated");
    let expected_json = serde_json::to_value(&expected).expect("Failed to serialize expected");
    assert_eq!(generated_json, expected_json, "CakeFillingResponse JSON mismatch");
}

// ============================================================================
// Cross-Deserialization Tests
// Verify that JSON from expected models can be deserialized into generated models and vice versa
// ============================================================================

#[test]
fn test_cake_create_cross_deserialization() {
    let expected = expected_cake::CakeCreate {
        name: Some("Cross Test".to_string()),
    };
    let json = serde_json::to_string(&expected).unwrap();

    // Deserialize expected JSON into generated type
    let generated: cake::CakeCreate = serde_json::from_str(&json).expect("Failed cross-deserialization");
    assert_eq!(generated.name, expected.name);
}

#[test]
fn test_fruit_create_cross_deserialization() {
    let expected = expected_fruit::FruitCreate {
        name: "Cross Fruit".to_string(),
        cake_id: Some(99),
    };
    let json = serde_json::to_string(&expected).unwrap();

    let generated: fruit::FruitCreate = serde_json::from_str(&json).expect("Failed cross-deserialization");
    assert_eq!(generated.name, expected.name);
    assert_eq!(generated.cake_id, expected.cake_id);
}

#[test]
fn test_filling_create_cross_deserialization() {
    let expected = expected_filling::FillingCreate {
        name: "Cross Filling".to_string(),
    };
    let json = serde_json::to_string(&expected).unwrap();

    let generated: filling::FillingCreate = serde_json::from_str(&json).expect("Failed cross-deserialization");
    assert_eq!(generated.name, expected.name);
}

#[test]
fn test_cake_filling_create_cross_deserialization() {
    let expected = expected_cake_filling::CakeFillingCreate {
        cake_id: 100,
        filling_id: 200,
    };
    let json = serde_json::to_string(&expected).unwrap();

    let generated: cake_filling::CakeFillingCreate = serde_json::from_str(&json).expect("Failed cross-deserialization");
    assert_eq!(generated.cake_id, expected.cake_id);
    assert_eq!(generated.filling_id, expected.filling_id);
}

#[test]
fn test_cake_update_cross_deserialization() {
    let expected = expected_cake::CakeUpdate {
        name: Some(Some("Cross Update".to_string())),
    };
    let json = serde_json::to_string(&expected).unwrap();

    let generated: cake::CakeUpdate = serde_json::from_str(&json).expect("Failed cross-deserialization");
    assert_eq!(generated.name, expected.name);
}

#[test]
fn test_fruit_update_cross_deserialization() {
    let expected = expected_fruit::FruitUpdate {
        name: Some(Some("Cross Update Fruit".to_string())),
        cake_id: Some(None), // Set to null
    };
    let json = serde_json::to_string(&expected).unwrap();

    let generated: fruit::FruitUpdate = serde_json::from_str(&json).expect("Failed cross-deserialization");
    assert_eq!(generated.name, expected.name);
    assert_eq!(generated.cake_id, expected.cake_id);
}

#[test]
fn test_cake_list_cross_deserialization() {
    let expected = expected_cake::CakeList {
        id: 55,
        name: Some("Cross List".to_string()),
    };
    let json = serde_json::to_string(&expected).unwrap();

    let generated: cake::CakeList = serde_json::from_str(&json).expect("Failed cross-deserialization");
    assert_eq!(generated.id, expected.id);
    assert_eq!(generated.name, expected.name);
}

#[test]
fn test_cake_response_cross_deserialization() {
    let expected = expected_cake::CakeResponse {
        id: 77,
        name: None,
    };
    let json = serde_json::to_string(&expected).unwrap();

    let generated: cake::CakeResponse = serde_json::from_str(&json).expect("Failed cross-deserialization");
    assert_eq!(generated.id, expected.id);
    assert_eq!(generated.name, expected.name);
}

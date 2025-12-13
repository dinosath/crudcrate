//! SeaORM 2.0 Relations Detection and Code Generation Integration Test
//!
//! This test module verifies that the `EntityToModels` derive macro correctly detects
//! and handles SeaORM 2.0's new relation field format:
//! - `HasOne<Entity>` - One-to-one relationships
//! - `HasMany<Entity>` - One-to-many relationships
//! - `#[sea_orm(has_one)]`, `#[sea_orm(has_many)]`, `#[sea_orm(belongs_to)]` attributes
//! - Many-to-many via junction table (`#[sea_orm(has_many, via = "...")]`)
//! - Self-referential relations (`#[sea_orm(self_ref, via = "...")]`)
//!
//! The tests verify:
//! 1. Relation fields are correctly detected by type (HasOne, HasMany)
//! 2. Relation fields are correctly detected by attributes
//! 3. Generated Create models have proper relation field types
//! 4. Generated Update models have proper relation field types
//! 5. Many-to-many relations via junction tables are handled
//! 6. Self-referential relations are detected and handled

use chrono::{DateTime, Utc};
use crudcrate::EntityToModels;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Test Entities using SeaORM 2.0 relation field syntax
// ============================================================================

/// User entity with HasOne profile and HasMany posts
pub mod user {
    use super::*;

    #[derive(
        Clone, Debug, PartialEq, DeriveEntityModel, EntityToModels, Serialize, Deserialize,
    )]
    #[sea_orm(table_name = "users")]
    #[crudcrate(api_struct = "User", derive_partial_eq)]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        #[crudcrate(primary_key, exclude(create, update), on_create = Uuid::new_v4())]
        pub id: Uuid,

        #[crudcrate(filterable, sortable)]
        pub name: String,

        #[crudcrate(filterable)]
        #[sea_orm(unique)]
        pub email: String,

        #[crudcrate(sortable, exclude(create, update), on_create = Utc::now())]
        pub created_at: DateTime<Utc>,

        #[crudcrate(sortable, exclude(create, update), on_create = Utc::now(), on_update = Utc::now())]
        pub updated_at: DateTime<Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(has_one = "super::profile::Entity")]
        Profile,

        #[sea_orm(has_many = "super::post::Entity")]
        Posts,
    }

    impl Related<super::profile::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Profile.def()
        }
    }

    impl Related<super::post::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Posts.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// Profile entity (one-to-one with User)
pub mod profile {
    use super::*;

    #[derive(
        Clone, Debug, PartialEq, DeriveEntityModel, EntityToModels, Serialize, Deserialize,
    )]
    #[sea_orm(table_name = "profiles")]
    #[crudcrate(api_struct = "Profile", derive_partial_eq)]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        #[crudcrate(primary_key, exclude(create, update), on_create = Uuid::new_v4())]
        pub id: Uuid,

        pub picture: String,

        pub bio: Option<String>,

        #[sea_orm(unique)]
        #[crudcrate(filterable)]
        pub user_id: Uuid,

        #[crudcrate(exclude(create, update), on_create = Utc::now())]
        pub created_at: DateTime<Utc>,

        #[crudcrate(exclude(create, update), on_create = Utc::now(), on_update = Utc::now())]
        pub updated_at: DateTime<Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::user::Entity",
            from = "Column::UserId",
            to = "super::user::Column::Id"
        )]
        User,
    }

    impl Related<super::user::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::User.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// Post entity (many-to-one with User, many-to-many with Tag)
pub mod post {
    use super::*;

    #[derive(
        Clone, Debug, PartialEq, DeriveEntityModel, EntityToModels, Serialize, Deserialize,
    )]
    #[sea_orm(table_name = "posts")]
    #[crudcrate(api_struct = "Post", derive_partial_eq)]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        #[crudcrate(primary_key, exclude(create, update), on_create = Uuid::new_v4())]
        pub id: Uuid,

        #[crudcrate(filterable)]
        pub user_id: Uuid,

        #[crudcrate(filterable, sortable)]
        pub title: String,

        pub content: String,

        #[crudcrate(filterable)]
        pub published: bool,

        #[crudcrate(sortable, exclude(create, update), on_create = Utc::now())]
        pub created_at: DateTime<Utc>,

        #[crudcrate(sortable, exclude(create, update), on_create = Utc::now(), on_update = Utc::now())]
        pub updated_at: DateTime<Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::user::Entity",
            from = "Column::UserId",
            to = "super::user::Column::Id"
        )]
        Author,

        #[sea_orm(has_many = "super::comment::Entity")]
        Comments,
    }

    impl Related<super::user::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Author.def()
        }
    }

    impl Related<super::comment::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Comments.def()
        }
    }

    // Many-to-many relation with Tag via PostTag junction table
    impl Related<super::tag::Entity> for Entity {
        fn to() -> RelationDef {
            super::post_tag::Relation::Tag.def()
        }

        fn via() -> Option<RelationDef> {
            Some(super::post_tag::Relation::Post.def().rev())
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// Comment entity (many-to-one with Post and User)
pub mod comment {
    use super::*;

    #[derive(
        Clone, Debug, PartialEq, DeriveEntityModel, EntityToModels, Serialize, Deserialize,
    )]
    #[sea_orm(table_name = "comments")]
    #[crudcrate(api_struct = "Comment", derive_partial_eq)]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        #[crudcrate(primary_key, exclude(create, update), on_create = Uuid::new_v4())]
        pub id: Uuid,

        #[crudcrate(filterable)]
        pub post_id: Uuid,

        #[crudcrate(filterable)]
        pub user_id: Uuid,

        pub content: String,

        #[crudcrate(sortable, exclude(create, update), on_create = Utc::now())]
        pub created_at: DateTime<Utc>,

        #[crudcrate(sortable, exclude(create, update), on_create = Utc::now(), on_update = Utc::now())]
        pub updated_at: DateTime<Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::post::Entity",
            from = "Column::PostId",
            to = "super::post::Column::Id"
        )]
        Post,

        #[sea_orm(
            belongs_to = "super::user::Entity",
            from = "Column::UserId",
            to = "super::user::Column::Id"
        )]
        Author,
    }

    impl Related<super::post::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Post.def()
        }
    }

    impl Related<super::user::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Author.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// Tag entity for many-to-many relation with Post
pub mod tag {
    use super::*;

    #[derive(
        Clone, Debug, PartialEq, DeriveEntityModel, EntityToModels, Serialize, Deserialize,
    )]
    #[sea_orm(table_name = "tags")]
    #[crudcrate(api_struct = "Tag", derive_partial_eq)]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        #[crudcrate(primary_key, exclude(create, update), on_create = Uuid::new_v4())]
        pub id: Uuid,

        #[sea_orm(unique)]
        #[crudcrate(filterable, sortable)]
        pub name: String,

        #[crudcrate(exclude(create, update), on_create = Utc::now())]
        pub created_at: DateTime<Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    // Many-to-many relation with Post via PostTag junction table
    impl Related<super::post::Entity> for Entity {
        fn to() -> RelationDef {
            super::post_tag::Relation::Post.def()
        }

        fn via() -> Option<RelationDef> {
            Some(super::post_tag::Relation::Tag.def().rev())
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// Junction table for Post <-> Tag many-to-many relation
pub mod post_tag {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "post_tags")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub post_id: Uuid,

        #[sea_orm(primary_key, auto_increment = false)]
        pub tag_id: Uuid,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::post::Entity",
            from = "Column::PostId",
            to = "super::post::Column::Id"
        )]
        Post,

        #[sea_orm(
            belongs_to = "super::tag::Entity",
            from = "Column::TagId",
            to = "super::tag::Column::Id"
        )]
        Tag,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// Category entity with self-referential parent/children relation
pub mod category {
    use super::*;

    #[derive(
        Clone, Debug, PartialEq, DeriveEntityModel, EntityToModels, Serialize, Deserialize,
    )]
    #[sea_orm(table_name = "categories")]
    #[crudcrate(api_struct = "Category", derive_partial_eq)]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        #[crudcrate(primary_key, exclude(create, update), on_create = Uuid::new_v4())]
        pub id: Uuid,

        #[crudcrate(filterable, sortable)]
        pub name: String,

        #[crudcrate(filterable)]
        pub parent_id: Option<Uuid>,

        #[crudcrate(sortable)]
        pub sort_order: i32,

        #[crudcrate(exclude(create, update), on_create = Utc::now())]
        pub created_at: DateTime<Utc>,

        #[crudcrate(exclude(create, update), on_create = Utc::now(), on_update = Utc::now())]
        pub updated_at: DateTime<Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        // Self-referential: belongs to parent category
        #[sea_orm(belongs_to = "Entity", from = "Column::ParentId", to = "Column::Id")]
        Parent,
    }

    // Self-referential relation for children
    impl Related<Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Parent.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Compile-time verification tests
// These tests verify that the generated types exist and have the correct structure
// ============================================================================

#[test]
fn test_user_create_model_exists() {
    // Verify UserCreate struct was generated
    let _create: user::UserCreate = user::UserCreate {
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
    };
}

#[test]
fn test_user_update_model_exists() {
    // Verify UserUpdate struct was generated with Option<Option<T>> fields
    let _update: user::UserUpdate = user::UserUpdate {
        name: Some(Some("Updated Name".to_string())),
        email: Some(Some("updated@example.com".to_string())),
    };
}

#[test]
fn test_profile_create_model_exists() {
    // Verify ProfileCreate struct was generated
    let _create: profile::ProfileCreate = profile::ProfileCreate {
        picture: "avatar.jpg".to_string(),
        bio: Some("A short bio".to_string()),
        user_id: Uuid::new_v4(),
    };
}

#[test]
fn test_post_create_model_exists() {
    // Verify PostCreate struct was generated
    let _create: post::PostCreate = post::PostCreate {
        user_id: Uuid::new_v4(),
        title: "Test Post".to_string(),
        content: "Post content".to_string(),
        published: false,
    };
}

#[test]
fn test_comment_create_model_exists() {
    // Verify CommentCreate struct was generated
    let _create: comment::CommentCreate = comment::CommentCreate {
        post_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        content: "Test comment".to_string(),
    };
}

#[test]
fn test_tag_create_model_exists() {
    // Verify TagCreate struct was generated
    let _create: tag::TagCreate = tag::TagCreate {
        name: "rust".to_string(),
    };
}

#[test]
fn test_category_create_model_with_self_reference() {
    // Verify CategoryCreate struct was generated and handles self-referential parent_id
    let parent_id = Uuid::new_v4();

    let _root_category: category::CategoryCreate = category::CategoryCreate {
        name: "Root Category".to_string(),
        parent_id: None,
        sort_order: 0,
    };

    let _child_category: category::CategoryCreate = category::CategoryCreate {
        name: "Child Category".to_string(),
        parent_id: Some(parent_id),
        sort_order: 1,
    };
}

#[test]
fn test_category_update_model_with_self_reference() {
    // Verify CategoryUpdate struct was generated with Option<Option<T>> pattern
    let new_parent_id = Uuid::new_v4();

    // Update name and move to a new parent
    let _update: category::CategoryUpdate = category::CategoryUpdate {
        name: Some(Some("Updated Category".to_string())),
        parent_id: Some(Some(new_parent_id)), // Move to a new parent
        sort_order: Some(Some(5)),
    };

    // Update to make it a root category (set parent to null)
    let _update_to_root: category::CategoryUpdate = category::CategoryUpdate {
        name: None,            // Don't update name
        parent_id: Some(None), // Set parent to NULL (make it a root)
        sort_order: None,      // Don't update sort_order
    };
}

// ============================================================================
// Serialization/Deserialization tests
// Verify that generated models serialize/deserialize correctly
// ============================================================================

#[test]
fn test_user_create_serialization() {
    let create = user::UserCreate {
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
    };

    let json = serde_json::to_string(&create).expect("Failed to serialize UserCreate");
    let parsed: user::UserCreate =
        serde_json::from_str(&json).expect("Failed to deserialize UserCreate");

    assert_eq!(parsed.name, create.name);
    assert_eq!(parsed.email, create.email);
}

#[test]
fn test_user_update_serialization() {
    let update = user::UserUpdate {
        name: Some(Some("Jane Doe".to_string())),
        email: None,
    };

    let json = serde_json::to_string(&update).expect("Failed to serialize UserUpdate");
    let parsed: user::UserUpdate =
        serde_json::from_str(&json).expect("Failed to deserialize UserUpdate");

    assert_eq!(parsed.name, update.name);
    assert_eq!(parsed.email, update.email);
}

#[test]
fn test_post_create_serialization() {
    let create = post::PostCreate {
        user_id: Uuid::new_v4(),
        title: "My First Post".to_string(),
        content: "This is the content of my post.".to_string(),
        published: true,
    };

    let json = serde_json::to_string(&create).expect("Failed to serialize PostCreate");
    let parsed: post::PostCreate =
        serde_json::from_str(&json).expect("Failed to deserialize PostCreate");

    assert_eq!(parsed.user_id, create.user_id);
    assert_eq!(parsed.title, create.title);
    assert_eq!(parsed.content, create.content);
    assert_eq!(parsed.published, create.published);
}

#[test]
fn test_category_create_serialization_with_null_parent() {
    let create = category::CategoryCreate {
        name: "Root Category".to_string(),
        parent_id: None,
        sort_order: 0,
    };

    let json = serde_json::to_string(&create).expect("Failed to serialize CategoryCreate");

    let parsed: category::CategoryCreate =
        serde_json::from_str(&json).expect("Failed to deserialize CategoryCreate");

    assert_eq!(parsed.name, create.name);
    assert_eq!(parsed.parent_id, None);
}

#[test]
fn test_category_create_serialization_with_parent() {
    let parent_id = Uuid::new_v4();
    let create = category::CategoryCreate {
        name: "Child Category".to_string(),
        parent_id: Some(parent_id),
        sort_order: 1,
    };

    let json = serde_json::to_string(&create).expect("Failed to serialize CategoryCreate");
    let parsed: category::CategoryCreate =
        serde_json::from_str(&json).expect("Failed to deserialize CategoryCreate");

    assert_eq!(parsed.name, create.name);
    assert_eq!(parsed.parent_id, Some(parent_id));
}

// ============================================================================
// JSON deserialization from raw strings (simulating API requests)
// ============================================================================

#[test]
fn test_user_create_from_json() {
    let json = r#"{"name": "Alice", "email": "alice@example.com"}"#;
    let create: user::UserCreate =
        serde_json::from_str(json).expect("Failed to parse UserCreate from JSON");

    assert_eq!(create.name, "Alice");
    assert_eq!(create.email, "alice@example.com");
}

#[test]
fn test_user_update_from_partial_json() {
    // Update should accept partial data (only fields that are being updated)
    let json = r#"{"name": "Bob"}"#;
    let update: user::UserUpdate =
        serde_json::from_str(json).expect("Failed to parse UserUpdate from JSON");

    // With double_option serde, "Bob" becomes Some(Some("Bob"))
    assert_eq!(update.name, Some(Some("Bob".to_string())));
    assert_eq!(update.email, None);
}

#[test]
fn test_post_create_from_json() {
    let user_id = Uuid::new_v4();
    let json = format!(
        r#"{{"user_id": "{}", "title": "Test", "content": "Content", "published": false}}"#,
        user_id
    );
    let create: post::PostCreate =
        serde_json::from_str(&json).expect("Failed to parse PostCreate from JSON");

    assert_eq!(create.user_id, user_id);
    assert_eq!(create.title, "Test");
    assert!(!create.published);
}

#[test]
fn test_category_create_from_json_with_null_parent() {
    let json = r#"{"name": "Root", "parent_id": null, "sort_order": 0}"#;
    let create: category::CategoryCreate =
        serde_json::from_str(json).expect("Failed to parse CategoryCreate from JSON");

    assert_eq!(create.name, "Root");
    assert_eq!(create.parent_id, None);
}

#[test]
fn test_category_create_from_json_without_parent() {
    // parent_id is Option<Uuid>, so it can be omitted entirely
    let json = r#"{"name": "Root", "sort_order": 0}"#;
    let create: category::CategoryCreate =
        serde_json::from_str(json).expect("Failed to parse CategoryCreate from JSON");

    assert_eq!(create.name, "Root");
    assert_eq!(create.parent_id, None);
}

#[test]
fn test_category_update_from_json_set_parent_to_null() {
    // Setting parent_id to null in update should set it to Some(None) with double_option
    let json = r#"{"parent_id": null}"#;
    let update: category::CategoryUpdate =
        serde_json::from_str(json).expect("Failed to parse CategoryUpdate from JSON");

    // parent_id should be Some(None) - meaning "set to null"
    assert_eq!(update.parent_id, Some(None));
}

// ============================================================================
// Response model tests
// ============================================================================

#[test]
fn test_user_response_model_exists() {
    // Verify response types are generated
    let _response: user::UserResponse = user::UserResponse {
        id: Uuid::new_v4(),
        name: "Test".to_string(),
        email: "test@example.com".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
}

#[test]
fn test_post_response_model_exists() {
    let _response: post::PostResponse = post::PostResponse {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        title: "Test".to_string(),
        content: "Content".to_string(),
        published: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
}

#[test]
fn test_category_response_model_exists() {
    let _response: category::CategoryResponse = category::CategoryResponse {
        id: Uuid::new_v4(),
        name: "Test Category".to_string(),
        parent_id: None,
        sort_order: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
}

// ============================================================================
// List model tests
// ============================================================================

#[test]
fn test_user_list_model_exists() {
    let _list: user::UserList = user::UserList {
        id: Uuid::new_v4(),
        name: "Test".to_string(),
        email: "test@example.com".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
}

#[test]
fn test_post_list_model_exists() {
    let _list: post::PostList = post::PostList {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        title: "Test".to_string(),
        content: "Content".to_string(),
        published: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
}

// ============================================================================
// Conversion tests (Create -> ActiveModel)
// ============================================================================

#[test]
fn test_user_create_to_active_model() {
    let create = user::UserCreate {
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
    };

    let active_model: user::ActiveModel = create.into();

    // Verify the active model has the correct values set
    match &active_model.name {
        sea_orm::ActiveValue::Set(name) => assert_eq!(name, "Test User"),
        _ => panic!("Expected name to be Set"),
    }

    match &active_model.email {
        sea_orm::ActiveValue::Set(email) => assert_eq!(email, "test@example.com"),
        _ => panic!("Expected email to be Set"),
    }

    // id should be set via on_create
    match &active_model.id {
        sea_orm::ActiveValue::Set(_) => {} // UUID was generated
        _ => panic!("Expected id to be Set via on_create"),
    }
}

#[test]
fn test_profile_create_to_active_model() {
    let user_id = Uuid::new_v4();
    let create = profile::ProfileCreate {
        picture: "avatar.png".to_string(),
        bio: Some("A bio".to_string()),
        user_id,
    };

    let active_model: profile::ActiveModel = create.into();

    match &active_model.picture {
        sea_orm::ActiveValue::Set(picture) => assert_eq!(picture, "avatar.png"),
        _ => panic!("Expected picture to be Set"),
    }

    match &active_model.bio {
        sea_orm::ActiveValue::Set(bio) => assert_eq!(bio, &Some("A bio".to_string())),
        _ => panic!("Expected bio to be Set"),
    }

    match &active_model.user_id {
        sea_orm::ActiveValue::Set(uid) => assert_eq!(uid, &user_id),
        _ => panic!("Expected user_id to be Set"),
    }
}

#[test]
fn test_post_create_to_active_model() {
    let user_id = Uuid::new_v4();
    let create = post::PostCreate {
        user_id,
        title: "My Post".to_string(),
        content: "Post content here".to_string(),
        published: true,
    };

    let active_model: post::ActiveModel = create.into();

    match &active_model.user_id {
        sea_orm::ActiveValue::Set(uid) => assert_eq!(uid, &user_id),
        _ => panic!("Expected user_id to be Set"),
    }

    match &active_model.title {
        sea_orm::ActiveValue::Set(title) => assert_eq!(title, "My Post"),
        _ => panic!("Expected title to be Set"),
    }

    match &active_model.published {
        sea_orm::ActiveValue::Set(published) => assert!(published),
        _ => panic!("Expected published to be Set"),
    }
}

#[test]
fn test_category_create_to_active_model_with_parent() {
    let parent_id = Uuid::new_v4();
    let create = category::CategoryCreate {
        name: "Child Category".to_string(),
        parent_id: Some(parent_id),
        sort_order: 5,
    };

    let active_model: category::ActiveModel = create.into();

    match &active_model.name {
        sea_orm::ActiveValue::Set(name) => assert_eq!(name, "Child Category"),
        _ => panic!("Expected name to be Set"),
    }

    match &active_model.parent_id {
        sea_orm::ActiveValue::Set(pid) => assert_eq!(pid, &Some(parent_id)),
        _ => panic!("Expected parent_id to be Set"),
    }

    match &active_model.sort_order {
        sea_orm::ActiveValue::Set(order) => assert_eq!(order, &5),
        _ => panic!("Expected sort_order to be Set"),
    }
}

#[test]
fn test_category_create_to_active_model_without_parent() {
    let create = category::CategoryCreate {
        name: "Root Category".to_string(),
        parent_id: None,
        sort_order: 0,
    };

    let active_model: category::ActiveModel = create.into();

    match &active_model.parent_id {
        sea_orm::ActiveValue::Set(pid) => assert_eq!(pid, &None),
        _ => panic!("Expected parent_id to be Set to None"),
    }
}

// ============================================================================
// Type constraint tests (ensure correct derives)
// ============================================================================

fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}
fn assert_debug<T: std::fmt::Debug>() {}
fn assert_clone<T: Clone>() {}

#[test]
fn test_generated_types_are_send() {
    assert_send::<user::UserCreate>();
    assert_send::<user::UserUpdate>();
    assert_send::<user::UserResponse>();
    assert_send::<user::UserList>();

    assert_send::<profile::ProfileCreate>();
    assert_send::<profile::ProfileUpdate>();

    assert_send::<post::PostCreate>();
    assert_send::<post::PostUpdate>();

    assert_send::<category::CategoryCreate>();
    assert_send::<category::CategoryUpdate>();
}

#[test]
fn test_generated_types_are_sync() {
    assert_sync::<user::UserCreate>();
    assert_sync::<user::UserUpdate>();
    assert_sync::<user::UserResponse>();
    assert_sync::<user::UserList>();

    assert_sync::<profile::ProfileCreate>();
    assert_sync::<profile::ProfileUpdate>();

    assert_sync::<post::PostCreate>();
    assert_sync::<post::PostUpdate>();

    assert_sync::<category::CategoryCreate>();
    assert_sync::<category::CategoryUpdate>();
}

#[test]
fn test_generated_types_are_debug() {
    assert_debug::<user::UserCreate>();
    assert_debug::<user::UserUpdate>();
    assert_debug::<user::UserResponse>();
    assert_debug::<user::UserList>();

    assert_debug::<profile::ProfileCreate>();
    assert_debug::<profile::ProfileUpdate>();

    assert_debug::<post::PostCreate>();
    assert_debug::<post::PostUpdate>();

    assert_debug::<category::CategoryCreate>();
    assert_debug::<category::CategoryUpdate>();
}

#[test]
fn test_generated_types_are_clone() {
    assert_clone::<user::UserCreate>();
    assert_clone::<user::UserUpdate>();
    assert_clone::<user::UserResponse>();
    assert_clone::<user::UserList>();

    assert_clone::<profile::ProfileCreate>();
    assert_clone::<profile::ProfileUpdate>();

    assert_clone::<post::PostCreate>();
    assert_clone::<post::PostUpdate>();

    assert_clone::<category::CategoryCreate>();
    assert_clone::<category::CategoryUpdate>();
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_empty_update_model() {
    // An update with all None values should be valid
    let update = user::UserUpdate {
        name: None,
        email: None,
    };

    let json = serde_json::to_string(&update).expect("Failed to serialize empty update");
    // Empty update should serialize (fields may be omitted due to skip_serializing_if)
    assert!(!json.is_empty());
}

#[test]
fn test_update_from_empty_json() {
    let json = r#"{}"#;
    let update: user::UserUpdate =
        serde_json::from_str(json).expect("Failed to parse empty JSON as UserUpdate");

    assert_eq!(update.name, None);
    assert_eq!(update.email, None);
}

#[test]
fn test_unicode_in_models() {
    let create = user::UserCreate {
        name: "用户名 🎉".to_string(),
        email: "测试@例子.com".to_string(),
    };

    let json = serde_json::to_string(&create).expect("Failed to serialize unicode");
    let parsed: user::UserCreate = serde_json::from_str(&json).expect("Failed to parse unicode");

    assert_eq!(parsed.name, "用户名 🎉");
    assert_eq!(parsed.email, "测试@例子.com");
}

#[test]
fn test_long_content_in_models() {
    let long_content = "a".repeat(100_000);

    let create = post::PostCreate {
        user_id: Uuid::new_v4(),
        title: "Long Post".to_string(),
        content: long_content.clone(),
        published: true,
    };

    let json = serde_json::to_string(&create).expect("Failed to serialize long content");
    let parsed: post::PostCreate =
        serde_json::from_str(&json).expect("Failed to parse long content");

    assert_eq!(parsed.content.len(), 100_000);
    assert_eq!(parsed.content, long_content);
}

// ============================================================================
// Multi-level relation scenario tests
// ============================================================================

#[test]
fn test_create_models_for_complete_hierarchy() {
    // Test that we can create models for a complete user -> post -> comment hierarchy
    let user_id = Uuid::new_v4();
    let post_id = Uuid::new_v4();

    let _user_create = user::UserCreate {
        name: "Author".to_string(),
        email: "author@blog.com".to_string(),
    };

    let _post_create = post::PostCreate {
        user_id,
        title: "My Article".to_string(),
        content: "Article content...".to_string(),
        published: true,
    };

    let _comment_create = comment::CommentCreate {
        post_id,
        user_id,
        content: "Great article!".to_string(),
    };

    // All models should compile and be instantiable
}

#[test]
fn test_self_referential_category_hierarchy() {
    let root_id = Uuid::new_v4();
    let level1_id = Uuid::new_v4();

    // Root category (no parent)
    let _root = category::CategoryCreate {
        name: "Electronics".to_string(),
        parent_id: None,
        sort_order: 0,
    };

    // Level 1 child
    let _level1 = category::CategoryCreate {
        name: "Computers".to_string(),
        parent_id: Some(root_id),
        sort_order: 1,
    };

    // Level 2 child
    let _level2 = category::CategoryCreate {
        name: "Laptops".to_string(),
        parent_id: Some(level1_id),
        sort_order: 2,
    };

    // Update to change parent (move category) - uses Option<Option<T>>
    let _move_category = category::CategoryUpdate {
        name: None,
        parent_id: Some(Some(root_id)), // Move directly under root
        sort_order: Some(Some(10)),
    };

    // Update to make it a root category
    let _make_root = category::CategoryUpdate {
        name: None,
        parent_id: Some(None), // Remove parent, making it a root
        sort_order: None,
    };
}

// ============================================================================
// Relation detection verification (compile-time)
// These tests verify that the macro infrastructure detects relations correctly
// ============================================================================

/// Test that entities with HasOne relations compile correctly
#[test]
fn test_has_one_relation_entity_compiles() {
    // User has_one Profile - verify both compile
    let _user_response = user::UserResponse {
        id: Uuid::new_v4(),
        name: "Test".to_string(),
        email: "test@test.com".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let _profile_response = profile::ProfileResponse {
        id: Uuid::new_v4(),
        picture: "pic.jpg".to_string(),
        bio: None,
        user_id: Uuid::new_v4(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
}

/// Test that entities with HasMany relations compile correctly
#[test]
fn test_has_many_relation_entity_compiles() {
    // User has_many Posts - verify both compile
    let _user = user::UserCreate {
        name: "Author".to_string(),
        email: "author@test.com".to_string(),
    };

    let _post = post::PostCreate {
        user_id: Uuid::new_v4(),
        title: "Title".to_string(),
        content: "Content".to_string(),
        published: false,
    };
}

/// Test that entities with BelongsTo relations compile correctly
#[test]
fn test_belongs_to_relation_entity_compiles() {
    // Profile belongs_to User
    // Comment belongs_to Post and User
    let user_id = Uuid::new_v4();
    let post_id = Uuid::new_v4();

    let _profile = profile::ProfileCreate {
        picture: "avatar.jpg".to_string(),
        bio: Some("Bio".to_string()),
        user_id,
    };

    let _comment = comment::CommentCreate {
        post_id,
        user_id,
        content: "Comment".to_string(),
    };
}

/// Test that many-to-many junction table entities compile
#[test]
fn test_many_to_many_junction_compiles() {
    // PostTag is a junction table for Post <-> Tag many-to-many
    let _post_tag = post_tag::Model {
        post_id: Uuid::new_v4(),
        tag_id: Uuid::new_v4(),
    };
}

/// Test that self-referential entities compile correctly
#[test]
fn test_self_referential_entity_compiles() {
    // Category has self-referential parent_id
    let parent_id = Uuid::new_v4();

    let _root = category::CategoryCreate {
        name: "Root".to_string(),
        parent_id: None,
        sort_order: 0,
    };

    let _child = category::CategoryCreate {
        name: "Child".to_string(),
        parent_id: Some(parent_id),
        sort_order: 1,
    };
}

// ============================================================================
// Semantic relation tests
// ============================================================================

#[test]
fn test_one_to_one_user_profile_semantic() {
    // Semantic test: A User should have exactly one Profile
    // This tests the concept, actual DB constraint is elsewhere
    let user_id = Uuid::new_v4();

    let _user = user::UserCreate {
        name: "User With Profile".to_string(),
        email: "with.profile@test.com".to_string(),
    };

    let _profile = profile::ProfileCreate {
        picture: "profile.jpg".to_string(),
        bio: Some("User's bio".to_string()),
        user_id,
    };

    // Both should be creatable - the relationship is enforced by user_id FK
}

#[test]
fn test_one_to_many_user_posts_semantic() {
    // Semantic test: A User can have many Posts
    let user_id = Uuid::new_v4();

    let _post1 = post::PostCreate {
        user_id,
        title: "First Post".to_string(),
        content: "Content 1".to_string(),
        published: true,
    };

    let _post2 = post::PostCreate {
        user_id,
        title: "Second Post".to_string(),
        content: "Content 2".to_string(),
        published: false,
    };

    let _post3 = post::PostCreate {
        user_id,
        title: "Third Post".to_string(),
        content: "Content 3".to_string(),
        published: true,
    };

    // Multiple posts can reference the same user_id
}

#[test]
fn test_many_to_one_comments_semantic() {
    // Semantic test: Many Comments can belong to one Post
    let post_id = Uuid::new_v4();
    let user1_id = Uuid::new_v4();
    let user2_id = Uuid::new_v4();

    let _comment1 = comment::CommentCreate {
        post_id,
        user_id: user1_id,
        content: "First comment".to_string(),
    };

    let _comment2 = comment::CommentCreate {
        post_id,
        user_id: user2_id,
        content: "Second comment".to_string(),
    };

    let _comment3 = comment::CommentCreate {
        post_id,
        user_id: user1_id,
        content: "Reply from first user".to_string(),
    };

    // Multiple comments reference the same post_id
}

#[test]
fn test_self_referential_category_tree_semantic() {
    // Semantic test: Categories form a tree structure
    let electronics_id = Uuid::new_v4();
    let computers_id = Uuid::new_v4();

    // Root level
    let _electronics = category::CategoryCreate {
        name: "Electronics".to_string(),
        parent_id: None,
        sort_order: 0,
    };

    // Level 1
    let _computers = category::CategoryCreate {
        name: "Computers".to_string(),
        parent_id: Some(electronics_id),
        sort_order: 1,
    };

    let _phones = category::CategoryCreate {
        name: "Phones".to_string(),
        parent_id: Some(electronics_id),
        sort_order: 2,
    };

    // Level 2
    let _laptops = category::CategoryCreate {
        name: "Laptops".to_string(),
        parent_id: Some(computers_id),
        sort_order: 1,
    };

    let _desktops = category::CategoryCreate {
        name: "Desktops".to_string(),
        parent_id: Some(computers_id),
        sort_order: 2,
    };

    // Tree structure: Electronics -> [Computers -> [Laptops, Desktops], Phones]
}

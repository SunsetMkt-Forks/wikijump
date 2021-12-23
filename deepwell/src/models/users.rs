//! SeaORM Entity. Generated by sea-orm-codegen 0.4.1

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub username: String,
    #[sea_orm(unique)]
    pub slug: String,
    pub username_changes: i16,
    #[sea_orm(unique)]
    pub email: String,
    pub email_verified_at: Option<DateTime>,
    pub password: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub multi_factor_secret: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub multi_factor_recovery_codes: Option<String>,
    pub remember_token: Option<String>,
    pub language: Option<String>,
    pub karma_points: i32,
    pub karma_level: i16,
    pub real_name: Option<String>,
    pub pronouns: Option<String>,
    pub dob: Option<Date>,
    pub bio: Option<String>,
    pub about_page: Option<String>,
    pub avatar_path: Option<String>,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
    pub deleted_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No RelationDef")
    }
}

impl ActiveModelBehavior for ActiveModel {}
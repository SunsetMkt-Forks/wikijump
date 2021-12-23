//! SeaORM Entity. Generated by sea-orm-codegen 0.4.1

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "file")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub file_id: i64,
    pub page_id: Option<i32>,
    pub site_id: Option<i32>,
    pub filename: Option<String>,
    pub mimetype: Option<String>,
    pub description: Option<String>,
    pub description_short: Option<String>,
    pub comment: Option<String>,
    pub size: Option<i32>,
    pub date_added: Option<DateTime>,
    pub user_id: Option<i32>,
    pub user_string: Option<String>,
    pub has_resized: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::page::Entity",
        from = "Column::PageId",
        to = "super::page::Column::PageId",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Page,
    #[sea_orm(
        belongs_to = "super::site::Entity",
        from = "Column::SiteId",
        to = "super::site::Column::SiteId",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Site,
}

impl Related<super::page::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Page.def()
    }
}

impl Related<super::site::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Site.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
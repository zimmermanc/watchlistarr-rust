use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub year: Option<i32>,
    pub item_type: ItemType,
    pub guid: Option<String>,
    pub imdb_id: Option<String>,
    pub tmdb_id: Option<i32>,
    pub tvdb_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ItemType {
    Movie,
    Show,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchlistItem {
    pub item: Item,
    pub added_at: DateTime<Utc>,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityProfile {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootFolder {
    pub id: i32,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: i32,
    pub label: String,
}
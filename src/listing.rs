use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize)]
pub struct Listing {
    pub entries: Vec<ListingPost>,
}

#[derive(Serialize)]
pub struct ListingPost {
    pub slug: String,
    pub desc: Option<String>,

    pub author: Option<String>,
    pub date: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
}

use serde::{Deserialize};
use chrono::{DateTime, Local};

use crate::listing::ListingPost;

#[derive(Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EntryStatus {
    Ongoing,
    Unspecified,
}

impl Default for EntryStatus {
    fn default() -> Self {
        Self::Unspecified
    }
}

#[derive(Deserialize)]
pub struct Frontmatter {
    #[serde(default)]
    pub status: EntryStatus,

    pub desc: Option<String>,
    pub author: Option<String>,
    pub date: DateTime<Local>,
}

impl Frontmatter {
    pub fn into_post(self, slug: String) -> ListingPost {
        ListingPost {
            slug,
            desc: self.desc,
            date: self.date.clone().into(),
            author: self.author,
            // TODO: impl
            last_modified: self.date.into(),
        }
    }
}

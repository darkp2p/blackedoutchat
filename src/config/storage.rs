use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Deserialize, Serialize)]
pub struct Storages {
    #[serde(rename = "storage")]
    pub storages: Vec<Storage>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Storage {
    Sqlite { path: PathBuf },
}

impl super::ConfigTrait for Storages {
    fn name() -> &'static str {
        "storages"
    }
}

impl Default for Storages {
    fn default() -> Self {
        Storages {
            storages: vec![Storage::Sqlite {
                path: "data/sqlite.db".parse().unwrap(),
            }],
        }
    }
}

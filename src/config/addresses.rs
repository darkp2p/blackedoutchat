use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct Addresses {
    #[serde(rename = "address")]
    pub addresses: Vec<Address>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Address {
    pub name: String,
    pub color: [u8; 3],
}

impl super::ConfigTrait for Addresses {
    fn name() -> &'static str {
        "addresses"
    }
}

impl Default for Addresses {
    fn default() -> Self {
        Addresses {
            addresses: vec![Address {
                name: "default".to_string(),
                color: [255, 255, 255],
            }],
        }
    }
}

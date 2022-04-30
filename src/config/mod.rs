pub mod addresses;
pub mod clients;
pub mod storage;

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs::create_dir_all;
use std::fs::read_to_string;
use std::fs::write;
use std::io::ErrorKind;
use std::path::PathBuf;

pub use self::addresses::*;
pub use self::clients::*;
pub use self::storage::*;

pub struct Config {
    pub addresses: Addresses,
    pub clients: Clients,
    pub storage: Storages,
}

impl Config {
    pub fn load() -> Self {
        Config {
            addresses: Addresses::load(),
            clients: Clients::load(),
            storage: Storages::load(),
        }
    }
}

pub trait ConfigTrait
where
    Self: Default + DeserializeOwned + Serialize,
{
    fn name() -> &'static str;

    fn path() -> PathBuf {
        PathBuf::new().join("data").join("config").join(
            Self::name()
                .parse::<PathBuf>()
                .unwrap()
                .with_extension("toml"),
        )
    }

    fn load() -> Self {
        let path = Self::path();

        create_dir_all(path.parent().unwrap()).unwrap();

        match read_to_string(&path).map(|x| {
            toml::from_str(x.as_str()).expect(&format!(
                "Failed to deserialize config `{}.toml`",
                Self::name()
            ))
        }) {
            Ok(n) => n,
            Err(e) => match e.kind() {
                ErrorKind::NotFound => {
                    let default = Self::default();
                    write(path, toml::to_string_pretty(&default).unwrap()).unwrap();
                    default
                }
                _ => {
                    panic!("Failed to read config `{}.toml`: {:?}", Self::name(), e);
                }
            },
        }
    }

    fn test_serialize() {
        println!("{}", toml::to_string_pretty(&Self::default()).unwrap());
    }
}

#[test]
fn test_serialize_addresses() {
    Addresses::test_serialize();
}

#[test]
fn test_serialize_clients() {
    Clients::test_serialize();
}

#[test]
fn test_serialize_storages() {
    Storages::test_serialize();
}

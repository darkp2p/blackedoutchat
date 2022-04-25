use std::{collections::HashMap, fs, path::PathBuf};

use ed25519_dalek::SecretKey;

use crate::{
    config::Config,
    error::{BlackedoutError, Result},
};

pub struct Onion {
    pub name: String,
    pub hostname: [u8; 56],
    pub secret_key: SecretKey,
}

pub fn get_onion_data(config: &Config) -> Result<HashMap<String, Onion>> {
    config
        .addresses
        .addresses
        .iter()
        .map(|addr| {
            let root = PathBuf::new().join("data").join(&addr.name);

            fs::read_to_string(root.join("hostname"))
                .and_then(|hostname| {
                    fs::read(root.join("hs_ed25519_secret_key")).map(|secret| (hostname, secret))
                })
                .map_err(Into::into)
                .and_then(|(hostname, secret)| {
                    if secret.len() < 64 {
                        return Err(BlackedoutError::TorBadSecretKey {
                            address: addr.name.clone(),
                        });
                    }

                    Ok((
                        addr.name.clone(),
                        Onion {
                            name: addr.name.clone(),
                            hostname: hostname.as_bytes().try_into().map_err(|_| {
                                BlackedoutError::TorBadHostname {
                                    address: addr.name.clone(),
                                }
                            })?,
                            secret_key: SecretKey::from_bytes(&secret[secret.len() - 64..])
                                .map_err(|_| BlackedoutError::TorBadSecretKey {
                                    address: addr.name.clone(),
                                })?,
                        },
                    ))
                })
        })
        .collect::<Result<HashMap<_, _>>>()
}

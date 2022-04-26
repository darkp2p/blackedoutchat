use std::{collections::HashMap, fs, path::PathBuf};

use ed25519_dalek::{ExpandedSecretKey, PublicKey};

use crate::{
    config::Config,
    error::{BlackedoutError, Result},
};

pub struct Onion {
    pub name: String,
    pub hostname: [u8; 56],
    pub public_key: PublicKey,
    pub secret_key: ExpandedSecretKey,
}

pub fn get_onion_data(config: &Config) -> Result<HashMap<[u8; 32], Onion>> {
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

                    let secret_key = ExpandedSecretKey::from_bytes(&secret[secret.len() - 64..])
                        .map_err(|_| BlackedoutError::TorBadSecretKey {
                            address: addr.name.clone(),
                        })?;

                    let public_key = PublicKey::from(&secret_key);

                    Ok((
                        *public_key.as_bytes(),
                        Onion {
                            name: addr.name.clone(),
                            hostname: hostname.as_bytes().try_into().map_err(|_| {
                                BlackedoutError::TorBadHostname {
                                    address: addr.name.clone(),
                                }
                            })?,
                            public_key,
                            secret_key,
                        },
                    ))
                })
        })
        .collect::<Result<HashMap<_, _>>>()
}

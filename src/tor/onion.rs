use std::{collections::HashMap, fs, path::PathBuf};

use ed25519_dalek::{ExpandedSecretKey, PublicKey as Ed25519PubKey};

use crate::{
    config::Config,
    error::{BlackedoutError, Result},
    types::{PublicKey, FULL_ADDRESS_LENGTH},
};

pub struct Onion {
    pub name: String,
    pub public_key: PublicKey,
    pub secret_key: ExpandedSecretKey,
}

pub fn get_onion_data(config: &Config) -> Result<HashMap<PublicKey, Onion>> {
    config
        .addresses
        .addresses
        .iter()
        .map(|addr| {
            let root = PathBuf::new()
                .join("data")
                .join("incoming")
                .join(&addr.name);

            fs::read_to_string(root.join("hostname"))
                .and_then(|hostname| {
                    fs::read(root.join("hs_ed25519_secret_key")).map(|secret| (hostname, secret))
                })
                .map_err(Into::into)
                .and_then(|(hostname, secret)| {
                    if secret.len() < 64 {
                        return Err(BlackedoutError::BadSecretKey);
                    }

                    if hostname.trim().len() != FULL_ADDRESS_LENGTH {
                        return Err(BlackedoutError::BadHostname);
                    }

                    let secret_key = ExpandedSecretKey::from_bytes(&secret[secret.len() - 64..])
                        .map_err(|_| BlackedoutError::BadSecretKey)?;

                    let public_key = PublicKey::from_onion_address(&hostname)?;

                    (Ed25519PubKey::from(&secret_key).as_bytes() == public_key.as_bytes())
                        .then(|| ())
                        .ok_or(BlackedoutError::BadHostname)?;

                    Ok((
                        public_key,
                        Onion {
                            name: addr.name.clone(),
                            public_key,
                            secret_key,
                        },
                    ))
                })
        })
        .collect::<Result<HashMap<_, _>>>()
}

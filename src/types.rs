use data_encoding::{BASE32, BASE64};
use ed25519_dalek::{ExpandedSecretKey, PublicKey as Ed25519PubKey, Signature, Verifier};
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize,
};
use sha3::{Digest, Sha3_256};

use crate::error::{BlackedoutError, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct PublicKey([u8; 32]);

pub const FULL_ADDRESS_LENGTH: usize = 62;
pub const ENCODED_ADDRESS_LENGTH: usize = 56;

impl PublicKey {
    pub fn to_onion_address(&self) -> String {
        let mut sha256 = Sha3_256::new();
        sha256.update(b".onion checksum");
        sha256.update(&self.0);
        sha256.update(b"\x03");

        let mut input = Vec::new();
        input.extend_from_slice(&self.0);
        input.extend_from_slice(&sha256.finalize().as_slice()[..2]);
        input.extend_from_slice(b"\x03");

        let mut output = BASE32.encode(&input);
        output.push_str(".onion");
        output.to_lowercase()
    }

    pub fn from_onion_address(addr: &str) -> Result<Self> {
        let addr = addr.trim();

        if addr.len() != FULL_ADDRESS_LENGTH {
            return Err(BlackedoutError::BadHostname);
        }

        Ok(PublicKey(
            *Ed25519PubKey::from_bytes(
                &BASE32.decode(
                    addr[..ENCODED_ADDRESS_LENGTH]
                        .to_ascii_uppercase()
                        .as_bytes(),
                )?[..32],
            )
            .map_err(|_| BlackedoutError::BadPublicKey)?
            .as_bytes(),
        ))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Ed25519PubKey::from_bytes(bytes)
            .map(|x| PublicKey(*x.as_bytes()))
            .map_err(|_| BlackedoutError::BadPublicKey)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn sign(&self, token: &[u8], secret_key: &ExpandedSecretKey) -> Signature {
        secret_key.sign(&token, &Ed25519PubKey::from_bytes(&self.0).unwrap())
    }

    pub fn verify(&self, token: &[u8], signature: &Signature) -> Result<()> {
        Ed25519PubKey::from_bytes(&self.0)
            .unwrap()
            .verify(&token, &signature)
            .map_err(|_| BlackedoutError::SignatureVerificationFailed)
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StrVisitor;

        impl<'de> Visitor<'de> for StrVisitor {
            type Value = PublicKey;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid ed25519 public key")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                BASE64
                    .decode(value.as_bytes())
                    .map_err(|err| de::Error::custom(err.to_string()))
                    .and_then(|bytes| {
                        PublicKey::from_bytes(&bytes)
                            .map_err(|_| de::Error::custom("failed to deserialize public key"))
                    })
            }
        }

        deserializer.deserialize_str(StrVisitor)
    }
}

#[test]
fn onion_address_conversion() {
    [
        "pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd.onion",
        "sp3k262uwy4r2k3ycr5awluarykdpag6a7y33jxop4cs2lu5uz5sseqd.onion",
        "xa4r2iadxm55fbnqgwwi5mymqdcofiu3w6rpbtqn7b2dyn7mgwj64jyd.onion",
    ]
    .iter()
    .for_each(|addr| {
        assert_eq!(
            *addr,
            PublicKey::from_onion_address(addr)
                .unwrap()
                .to_onion_address()
                .as_str()
        );
    });
}

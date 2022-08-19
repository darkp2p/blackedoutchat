use pqcrypto_kyber::kyber102490s;
use pqcrypto_traits::kem::{Ciphertext, PublicKey, SharedSecret};
use sha3::{Digest, Sha3_256};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::error::Result;

macro_rules! handshake_a {
    ($algorithm:ident, $stream:ident, $secrets:ident) => {
        let mut buf = [0u8; $algorithm::public_key_bytes()];
        $stream.read_exact(&mut buf).await?;

        let pk = PublicKey::from_bytes(&buf)?;
        let (sk, ct) = $algorithm::encapsulate(&pk);
        $secrets.push(sk.as_bytes().to_vec());
        $stream.write_all(ct.as_bytes()).await?;
    };
}

macro_rules! handshake_b {
    ($algorithm:ident, $stream:ident, $secrets:ident) => {
        let (pk, sk) = $algorithm::keypair();

        let mut buf = [0u8; $algorithm::ciphertext_bytes()];
        $stream.write_all(pk.as_bytes()).await?;
        $stream.read_exact(&mut buf).await?;

        let ct = Ciphertext::from_bytes(&buf)?;
        let sk = $algorithm::decapsulate(&ct, &sk);
        $secrets.push(sk.as_bytes().to_vec());
    };
}

pub async fn handshake<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    alice: bool,
) -> Result<[u8; 32]> {
    let mut secrets = Vec::new();

    if alice {
        handshake_a!(kyber102490s, stream, secrets);
    } else {
        handshake_b!(kyber102490s, stream, secrets);
    }

    let mut sha = Sha3_256::new();

    for secret in secrets.iter() {
        sha.update(secret);
    }

    let mut key = [0u8; 32];
    key.clone_from_slice(sha.finalize().as_slice());
    Ok(key)
}

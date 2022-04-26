use std::{
    pin::Pin,
    task::{Context, Poll},
};

use aes_gcm::{
    aead::{generic_array::GenericArray, AeadInPlace, NewAead},
    Aes256Gcm,
};

use bytes::Bytes;
use futures::{ready, Sink, Stream};
use rand::RngCore;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use crate::connections::model::BlackPacket;
use crate::crypto::handshake;
use crate::error::{BlackedoutError, Result};

pub struct SecureStream<S: AsyncRead + AsyncWrite + Unpin> {
    inner: Framed<S, LengthDelimitedCodec>,
    cipher: Aes256Gcm,
}

impl<S> SecureStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn new(mut inner: S, alice: bool) -> Result<Self> {
        let key = handshake(&mut inner, alice).await?;
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let inner = Framed::new(inner, LengthDelimitedCodec::new());

        Ok(SecureStream { inner, cipher })
    }
}

impl<S> Stream for SecureStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    type Item = Result<BlackPacket>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut bytes = match ready!(Pin::new(&mut self.inner).poll_next(cx)) {
            Some(n) => match n {
                Ok(n) => n,
                Err(e) => return Poll::Ready(Some(Err(e.into()))),
            },
            None => return Poll::Ready(None),
        };

        if bytes.len() < 28 {
            return Poll::Ready(Some(Err(BlackedoutError::AesBadLength)));
        }

        let (nonce, rest) = bytes.split_at_mut(12);
        let (tag, buffer) = rest.split_at_mut(16);

        match self.cipher.decrypt_in_place_detached(
            GenericArray::from_slice(nonce),
            b"",
            buffer,
            GenericArray::from_slice(tag),
        ) {
            Ok(_) => {}
            Err(_) => return Poll::Ready(Some(Err(BlackedoutError::AesBadTag))),
        }

        match bson::from_slice::<BlackPacket>(buffer) {
            Ok(n) => Poll::Ready(Some(Ok(n))),
            Err(e) => Poll::Ready(Some(Err(e.into()))),
        }
    }
}

impl<S> Sink<BlackPacket> for SecureStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    type Error = BlackedoutError;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.inner).poll_ready(cx).map_err(Into::into)
    }

    fn start_send(mut self: Pin<&mut Self>, item: BlackPacket) -> Result<()> {
        let mut ciphertext = vec![0u8; 28];
        ciphertext.append(&mut bson::to_vec(&item).unwrap());

        let (nonce, rest) = ciphertext.split_at_mut(12);
        let (tag, buffer) = rest.split_at_mut(16);

        rand::thread_rng().fill_bytes(nonce);

        tag.clone_from_slice(
            self.cipher
                .encrypt_in_place_detached(GenericArray::from_slice(nonce), b"", buffer)
                .map_err(|_| BlackedoutError::AesEncryptionError)?
                .as_slice(),
        );

        Pin::new(&mut self.inner)
            .start_send(Bytes::from(ciphertext))
            .map_err(Into::into)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx).map_err(Into::into)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.inner).poll_close(cx).map_err(Into::into)
    }
}

use serde::{ser::SerializeStruct, Serialize, Serializer};
use strum::Display;

#[derive(Debug, Display)]
#[strum(serialize_all = "snake_case")]
pub enum BlackedoutError {
    AesBadLength,
    AesBadTag,
    AesEncryptionError,
    AxumError(axum::Error),
    BadHostname,
    BadPublicKey,
    BadSecretKey,
    BadSignature,
    HostPublicKeyDoesNotExist,
    PeerPublicKeyDoesNotExist,
    Hyper(hyper::Error),
    SocksError(tokio_socks::Error),
    SignatureVerificationFailed,
    Base32Error(data_encoding::DecodeError),
    BsonError(bson::de::Error),
    ConnectionClosed,
    TorShutdown(Box<BlackedoutError>),
    Io(std::io::Error),
    PqCrypto(pqcrypto_traits::Error),
    WrongPacketType(String),
    Unexpected,
}

pub type Result<T> = std::result::Result<T, BlackedoutError>;

impl Serialize for BlackedoutError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut error = serializer.serialize_struct("BlackedoutError", 2)?;
        error.serialize_field("error_kind", &self.to_string())?;

        error.end()
    }
}

macro_rules! impl_from {
    ($error:ty, $kind:ident) => {
        impl From<$error> for BlackedoutError {
            fn from(e: $error) -> Self {
                BlackedoutError::$kind(e)
            }
        }
    };
}

impl_from!(axum::Error, AxumError);
impl_from!(bson::de::Error, BsonError);
impl_from!(data_encoding::DecodeError, Base32Error);
impl_from!(hyper::Error, Hyper);
impl_from!(tokio_socks::Error, SocksError);
impl_from!(std::io::Error, Io);
impl_from!(pqcrypto_traits::Error, PqCrypto);

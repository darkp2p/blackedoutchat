#[derive(Debug)]
pub enum BlackedoutError {
    AesBadLength,
    AesBadTag,
    AesEncryptionError,
    BadHostname,
    BadPublicKey,
    BadSignature,
    SocksError(tokio_socks::Error),
    SignatureVerificationFailed,
    Base32Error(data_encoding::DecodeError),
    BsonError(bson::de::Error),
    ConnectionClosed,
    TorBadHostname { address: String },
    TorBadSecretKey { address: String },
    TorShutdown(Box<BlackedoutError>),
    Io(std::io::Error),
    PqCrypto(pqcrypto::traits::Error),
    WrongPacketType { description: String },
}

pub type Result<T> = std::result::Result<T, BlackedoutError>;

impl From<bson::de::Error> for BlackedoutError {
    fn from(e: bson::de::Error) -> Self {
        BlackedoutError::BsonError(e)
    }
}

impl From<tokio_socks::Error> for BlackedoutError {
    fn from(e: tokio_socks::Error) -> Self {
        BlackedoutError::SocksError(e)
    }
}

impl From<std::io::Error> for BlackedoutError {
    fn from(e: std::io::Error) -> Self {
        BlackedoutError::Io(e)
    }
}

impl From<pqcrypto::traits::Error> for BlackedoutError {
    fn from(e: pqcrypto::traits::Error) -> Self {
        BlackedoutError::PqCrypto(e)
    }
}

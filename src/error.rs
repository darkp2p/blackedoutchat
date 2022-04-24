#[derive(Debug)]
pub enum BlackedoutError {
    AesBadLength,
    AesBadTag,
    AesEncryptionError,
    BsonError(bson::de::Error),
    TorShutdown(Box<BlackedoutError>),
    Io(std::io::Error),
    PqCrypto(pqcrypto::traits::Error),
}

pub type Result<T> = std::result::Result<T, BlackedoutError>;

impl From<bson::de::Error> for BlackedoutError {
    fn from(e: bson::de::Error) -> Self {
        BlackedoutError::BsonError(e)
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

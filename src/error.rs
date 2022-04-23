#[derive(Debug)]
pub enum BlackedoutError {
    TorShutdown(Box<BlackedoutError>),
    Io(std::io::Error),
}

pub type Result<T> = std::result::Result<T, BlackedoutError>;

impl From<std::io::Error> for BlackedoutError {
    fn from(e: std::io::Error) -> Self {
        BlackedoutError::Io(e)
    }
}

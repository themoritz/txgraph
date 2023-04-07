use bitcoin_explorer::parser::errors::OpError;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    RocksDB { value: rocksdb::Error },
    Blockchain { value: OpError },
    Serde { value: serde_cbor::Error },
    Bitcoin { value: bitcoin_hashes::Error },
    Hyper { value: hyper::Error },
    Tokio { value: tokio::task::JoinError },
    NotYetIndexed,
}

impl From<rocksdb::Error> for Error {
    fn from(value: rocksdb::Error) -> Self {
        Error::RocksDB { value }
    }
}

impl From<OpError> for Error {
    fn from(value: OpError) -> Self {
        Error::Blockchain { value }
    }
}

impl From<serde_cbor::Error> for Error {
    fn from(value: serde_cbor::Error) -> Self {
        Error::Serde { value }
    }
}

impl From<bitcoin_hashes::Error> for Error {
    fn from(value: bitcoin_hashes::Error) -> Self {
        Error::Bitcoin { value }
    }
}

impl From<hyper::Error> for Error {
    fn from(value: hyper::Error) -> Self {
        Error::Hyper { value }
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(value: tokio::task::JoinError) -> Self {
        Error::Tokio { value }
    }
}

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("")]
    NoSuchInterface,
    #[error("parse error")]
    ParseError,
    #[error("unexpected icmp packet with type `{0:?}` and code `{1:?}`")]
    UnexpectedIcmpPacket(pnet::packet::icmp::IcmpType, pnet::packet::icmp::IcmpCode),
    #[error("network error: {0}")]
    NetworkError(#[from] std::io::Error), // thus io::Error can implicitly `into` NetworkError
}

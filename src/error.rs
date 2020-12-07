use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot find interface `{0}`")]
    NoSuchInterface(String),
    #[error("parse error")]
    ParseError,
    #[error("unexpected icmp packet with source port `{0}`, expected `{1}`")]
    UnexpectedIcmpSrcPort(u16, u16),
    #[error("unexpected icmp packet with type `{0:?}` and code `{1:?}`")]
    UnexpectedIcmpType(pnet::packet::icmp::IcmpType, pnet::packet::icmp::IcmpCode),
    #[error("network error: {0}")]
    NetworkError(#[from] std::io::Error), // thus io::Error can implicitly `into` NetworkError
}

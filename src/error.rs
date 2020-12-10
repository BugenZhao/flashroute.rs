use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot find interface `{0}`")]
    NoSuchInterface(String),
    #[error("parse error at stage {0}")]
    ParseError(u8),
    #[error("unexpected icmp packet with source port `{0}`, expected `{1}`")]
    UnexpectedIcmpSrcPort(u16, u16),
    #[error("invalid distance with initial_ttl `{0}` and dst_ttl `{1}`")]
    InvalidDistance(u8, u8),
    #[error("unexpected icmp packet with type `{0:?}` and code `{1:?}`")]
    UnexpectedIcmpType(pnet::packet::icmp::IcmpType, pnet::packet::icmp::IcmpCode),
    #[error("")]
    BadGrainOrNet(u8, ipnet::Ipv4Net),
    #[error("")]
    InvalidIpv4Addr(String),
    #[error("")]
    CannotResolveTargets(String),
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error), // thus io::Error can implicitly `into` IoError
}

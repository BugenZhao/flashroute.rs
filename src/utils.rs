use std::{
    net::IpAddr,
    time::{SystemTime, UNIX_EPOCH},
};

use pnet::datalink::NetworkInterface;

use crate::error::*;

pub fn get_interface_ipv4_addr(ni: &NetworkInterface) -> Option<IpAddr> {
    ni.ips
        .clone()
        .into_iter()
        .filter(|ip| ip.is_ipv4())
        .next()
        .map(|net| net.ip())
}

pub fn get_interface(name: &str) -> Result<NetworkInterface> {
    let interfaces = pnet::datalink::interfaces();

    if name.is_empty() {
        interfaces
            .into_iter()
            .filter(|ni| ni.is_up() && !ni.is_loopback() && !ni.ips.is_empty())
            .next()
            .ok_or(Error::NoSuchInterface(name.to_owned()))
    } else {
        interfaces
            .into_iter()
            .filter(|ni| ni.name == name)
            .next()
            .ok_or(Error::NoSuchInterface(name.to_owned()))
    }
}

pub fn get_timestamp_ms_u16() -> u16 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u16
}

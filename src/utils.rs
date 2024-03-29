use std::{
    net::{IpAddr, Ipv4Addr},
    time::{SystemTime, UNIX_EPOCH},
};

use pnet::datalink::NetworkInterface;

use crate::error::*;

pub fn get_interface_ipv4_addr(ni: &NetworkInterface) -> Option<Ipv4Addr> {
    for ip in ni.ips.iter().map(|net| net.ip()) {
        if let IpAddr::V4(ipv4) = ip {
            return Some(ipv4);
        }
    }
    None
}

pub fn get_interface(name: &str) -> Result<NetworkInterface> {
    let interfaces = pnet::datalink::interfaces();

    if name.is_empty() {
        interfaces
            .into_iter()
            .filter(|ni| ni.is_up() && !ni.is_loopback() && get_interface_ipv4_addr(ni).is_some())
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

pub fn timestamp_ms_u16() -> u16 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u16
}

pub fn ip_checksum(addr: Ipv4Addr, salt: u16) -> u16 {
    pnet::util::checksum(&addr.octets(), 0) + salt
}

pub fn ensure_su() {
    if sudo::check() == sudo::RunningAs::User {
        log::warn!(
            "Listening on ICMP socket requires superuser permission. \
             {} will restart with sudo.",
            env!("CARGO_PKG_NAME")
        );
        sudo::escalate_if_needed().unwrap();
    }
}

pub trait GlobalIpv4Ext {
    fn is_bz_global(&self) -> bool;
}

impl GlobalIpv4Ext for Ipv4Addr {
    fn is_bz_global(&self) -> bool {
        // check if this address is 192.0.0.9 or 192.0.0.10. These addresses are the only two
        // globally routable addresses in the 192.0.0.0/24 range.
        if u32::from_be_bytes(self.octets()) == 0xc0000009
            || u32::from_be_bytes(self.octets()) == 0xc000000a
        {
            return true;
        }
        !self.is_private()
            && !self.is_loopback()
            && !self.is_link_local()
            && !self.is_broadcast()
            && !self.is_documentation()
            && !(self.octets()[0] == 100 && (self.octets()[1] & 0b1100_0000 == 0b0100_0000))
            && !(self.octets()[0] == 192 && self.octets()[1] == 0 && self.octets()[2] == 0)
            && !(self.octets()[0] & 240 == 240 && !self.is_broadcast())
            && !(self.octets()[0] == 198 && (self.octets()[1] & 0xfe) == 18)
            // Make sure the address is not in 0.0.0.0/8
            && self.octets()[0] != 0
    }
}

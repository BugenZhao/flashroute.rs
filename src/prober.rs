use crate::error::*;
use pnet::packet::{icmp::*, ip::IpNextHeaderProtocols, ipv4::*, udp::*, Packet};
use std::net::Ipv4Addr;

#[derive(Default)]
pub struct ProbeDebugResult {}

pub struct ProbeResult {
    destination: Ipv4Addr,
    responder: Ipv4Addr,
    distance: u8,
    from_destination: bool,
    debug: ProbeDebugResult,
}

#[derive(Copy, Clone)]
pub enum ProbePhase {
    Pre = 0,
    Main = 1,
}

pub type ProbeCallback = fn(result: ProbeResult) -> ();

pub struct Prober {
    callback: ProbeCallback,
    phase: ProbePhase,
    dst_port: u16,
    payload_msg: String,
    encode_timestamp: bool,
    checksum_salt: u16,
}

impl Prober {
    const IPV4_HEADER_LENGTH: u16 = 20;
    const ICMP_HEADER_LENGTH: u16 = 8;

    pub fn new(
        callback: ProbeCallback,
        phase: ProbePhase,
        dst_port: u16,
        payload_msg: String,
        encode_timestamp: bool,
        checksum_salt: u16,
    ) -> Self {
        Self {
            callback,
            phase,
            dst_port,
            payload_msg,
            encode_timestamp,
            checksum_salt,
        }
    }
}

pub type ProbeUnit = (Ipv4Addr, u8);

impl Prober {
    pub fn pack(&self, destination: ProbeUnit, source_ip: Ipv4Addr) -> Ipv4Packet {
        let (dst_ip, ttl) = destination;
        let timestamp = crate::utils::timestamp_ms_u16();
        let expect_total_size = {
            let mut size = 128;
            if self.encode_timestamp {
                size |= ((timestamp >> 10) & 0x3F) << 1;
            }
            size
        };
        let expect_udp_size = expect_total_size - Self::IPV4_HEADER_LENGTH;

        let mut udp_packet = MutableUdpPacket::owned(vec![0u8; expect_udp_size as usize]).unwrap();
        udp_packet.set_source(crate::utils::ip_checksum(dst_ip, self.checksum_salt)); // TODO: is this ok?
        udp_packet.set_destination(self.dst_port);
        udp_packet.set_length(expect_udp_size);
        udp_packet.set_payload(self.payload_msg.as_bytes());

        let ip_id = {
            let mut id = (ttl as u16 & 0x1F) | ((self.phase as u16 & 0x1) << 5);
            if self.encode_timestamp {
                id |= (timestamp & 0x3FF) << 6;
            }
            id
        };

        let mut ip_packet =
            MutableIpv4Packet::owned(vec![0u8; expect_total_size as usize]).unwrap();
        ip_packet.set_version(4);
        ip_packet.set_header_length((Self::IPV4_HEADER_LENGTH >> 2) as u8);
        ip_packet.set_destination(dst_ip);
        ip_packet.set_source(source_ip);
        ip_packet.set_next_level_protocol(IpNextHeaderProtocols::Udp);
        ip_packet.set_ttl(ttl);
        ip_packet.set_identification(ip_id);
        ip_packet.set_total_length(expect_total_size);

        ip_packet.set_payload(udp_packet.packet());

        // TODO: is it ok to ignore checksums?

        return ip_packet.consume_to_immutable();
    }

    pub fn parse(&self, packet: Box<dyn Packet>) -> Result<ProbeResult> {
        let ip_packet = Ipv4Packet::new(packet.packet()).ok_or(Error::ParseError)?;
        let icmp_packet = IcmpPacket::new(ip_packet.payload()).ok_or(Error::ParseError)?;
        let res_ip_packet = Ipv4Packet::new(icmp_packet.payload()).ok_or(Error::ParseError)?;
        let res_udp_packet = UdpPacket::new(res_ip_packet.payload()).ok_or(Error::ParseError)?;

        let destination = res_ip_packet.get_destination();
        let src_port = res_udp_packet.get_source();
        let expected_src_port = crate::utils::ip_checksum(destination, self.checksum_salt);
        if src_port != expected_src_port {
            return Err(Error::UnexpectedIcmpSrcPort(src_port, expected_src_port));
        }

        let initial_ttl = {
            let ttl = res_ip_packet.get_identification() & 0x1f;
            if ttl == 0 {
                32
            } else {
                ttl as u8
            }
        };
        let dst_ttl = res_ip_packet.get_ttl();

        let icmp_type = icmp_packet.get_icmp_type();
        let icmp_code = icmp_packet.get_icmp_code();

        let (distance, from_destination) = {
            if icmp_type == IcmpTypes::DestinationUnreachable && [1, 2, 3].contains(&icmp_code.0) {
                (initial_ttl - dst_ttl + 1, true)
            } else if icmp_type == IcmpTypes::TimeExceeded {
                (initial_ttl, false)
            } else {
                return Err(Error::UnexpectedIcmpType(icmp_type, icmp_code));
            }
        };

        // TODO: extract more data for debug use

        let result = ProbeResult {
            destination,
            responder: ip_packet.get_source(),
            distance,
            from_destination,
            debug: ProbeDebugResult {},
        };

        Ok(result)
    }

    pub fn run_callback(&self, probe_result: ProbeResult) {
        (self.callback)(probe_result);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    lazy_static! {
        static ref IP1: Ipv4Addr = "1.2.3.4".parse().unwrap();
        static ref IP2: Ipv4Addr = "4.3.2.1".parse().unwrap();
        // static ref PACKET: &'static [u8] = include_bytes!("../res/packet.bin");
    }

    #[test]
    fn test_pack() {
        let prober = Prober::new(|_| {}, ProbePhase::Pre, 33434, "hello".to_owned(), true, 0);
        let packet = prober.pack((*IP1, 32), *IP2);
        println!("{:#?}", packet);
    }

    // TODO: add more realistic tests
    // TODO: add tests for parsing
}

use crate::error::*;
use crate::OPT;
use pnet::packet::{icmp::*, ip::IpNextHeaderProtocols, ipv4::*, udp::*, Packet};
use std::net::Ipv4Addr;

#[derive(Default, Debug)]
pub struct ProbeDebugResult {
    pub rtt: u16,
}

#[derive(Debug)]
pub struct ProbeResult {
    pub destination: Ipv4Addr,
    pub responder: Ipv4Addr,
    pub distance: u8,
    pub from_destination: bool,
    pub debug: ProbeDebugResult,
}

#[derive(Copy, Clone, Debug)]
pub enum ProbePhase {
    Pre = 0,
    Main = 1,
}

#[derive(Debug)]
pub struct Prober {
    pub phase: ProbePhase,
    encode_timestamp: bool,
}

impl Prober {
    const IPV4_HEADER_LENGTH: u16 = 20;
    const ICMP_HEADER_LENGTH: u16 = 8;

    pub fn new(phase: ProbePhase, encode_timestamp: bool) -> Self {
        Self {
            phase,
            encode_timestamp,
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
        udp_packet.set_source(crate::utils::ip_checksum(dst_ip, OPT.salt)); // TODO: is this ok?
        udp_packet.set_destination(OPT.dst_port);
        udp_packet.set_length(expect_udp_size);
        udp_packet.set_payload(OPT.payload_message.as_bytes());

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

    pub fn parse(&self, packet: &[u8], ignore_port: bool) -> Result<ProbeResult> {
        // currently there's a bug in pnet, that ip total length has incorrect endianness on apple devices
        // thus, we can't...
        //  - construct res_ip_packet from ip_packet.payload()[ICMP_HDR_LEN..]
        //  - directly decode timestamp from res_ip_packet.get_total_length()

        let ip_packet = Ipv4Packet::new(packet).ok_or(Error::ParseError(1))?;
        let icmp_packet = IcmpPacket::new(ip_packet.payload()).ok_or(Error::ParseError(2))?;
        let res_ip_packet = Ipv4Packet::new(
            &ip_packet.packet()[(Self::IPV4_HEADER_LENGTH + Self::ICMP_HEADER_LENGTH) as usize..],
        )
        .ok_or(Error::ParseError(3))?;
        let res_udp_packet = UdpPacket::new(res_ip_packet.payload()).ok_or(Error::ParseError(4))?;

        let destination = res_ip_packet.get_destination();
        let src_port = res_udp_packet.get_source();
        let expected_src_port = crate::utils::ip_checksum(destination, OPT.salt);
        if src_port != expected_src_port && !ignore_port {
            return Err(Error::UnexpectedIcmpSrcPort(src_port, expected_src_port));
        }

        // log::trace!("{:#?}", ip_packet);
        // log::trace!("{:#?}", icmp_packet);
        // log::trace!("{:#?}", res_ip_packet);
        // log::trace!("{:#?}", res_udp_packet);

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
                if initial_ttl < dst_ttl {
                    return Err(Error::InvalidDistance(initial_ttl, dst_ttl));
                }
                (initial_ttl - dst_ttl + 1, true)
            } else if icmp_type == IcmpTypes::TimeExceeded {
                (initial_ttl, false)
            } else {
                return Err(Error::UnexpectedIcmpType(icmp_type, icmp_code));
            }
        };

        let rtt = if self.encode_timestamp {
            let send = if cfg!(target_vendor = "apple") {
                // byte order fix
                ((res_ip_packet.get_identification() >> 6) & 0x3FF)
                    | (((res_ip_packet.get_total_length().to_be() >> 1) & 0x3F) << 10)
            } else {
                ((res_ip_packet.get_identification() >> 6) & 0x3FF)
                    | (((res_ip_packet.get_total_length() >> 1) & 0x3F) << 10)
            } as u32;
            let recv = crate::utils::timestamp_ms_u16() as u32;
            log::trace!("send: 0x{:x}, recv: 0x{:x}", send, recv);

            if recv >= send {
                recv - send
            } else {
                recv + u16::MAX as u32 - send
            }
        } else {
            0
        } as u16;

        let result = ProbeResult {
            destination,
            responder: ip_packet.get_source(),
            distance,
            from_destination,
            debug: ProbeDebugResult { rtt },
        };

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! packet {
        ($bin:literal) => {
            Ipv4Packet::owned(include_bytes!($bin)[14..].to_vec()).unwrap()
        };
    }

    lazy_static! {
        static ref IP1: Ipv4Addr = "1.2.3.4".parse().unwrap();
        static ref IP2: Ipv4Addr = "4.3.2.1".parse().unwrap();
        static ref TLE_WITHOUT_DATA: Ipv4Packet<'static> =
            packet!("../res/frame_tle_without_data.bin");
        static ref TLE_WITH_DATA: Ipv4Packet<'static> = packet!("../res/frame_tle_with_data.bin");
        static ref UNR_WITH_DATA: Ipv4Packet<'static> =
            packet!("../res/frame_unreachable_with_data.bin");
    }

    #[test]
    fn test_pack() {
        let prober = Prober::new(ProbePhase::Pre, true);
        let packet = prober.pack((*IP1, 32), *IP2);
        println!("{:#?}", packet);
    }

    #[test]
    fn test_parse() {
        let prober = Prober::new(ProbePhase::Pre, true);
        {
            let result = prober.parse(TLE_WITH_DATA.packet(), true).unwrap();
            println!("{:#?}", result);
            assert_eq!(result.destination.to_string(), "59.78.31.75");
            assert_eq!(result.responder.to_string(), "59.78.37.254");
            assert_eq!(result.distance, 2);
            assert_eq!(result.from_destination, false);
        }
        {
            let result = prober.parse(TLE_WITHOUT_DATA.packet(), true).unwrap();
            println!("{:#?}", result);
            assert_eq!(result.destination.to_string(), "59.78.173.84");
            assert_eq!(result.responder.to_string(), "101.4.135.214");
            assert_eq!(result.distance, 11);
            assert_eq!(result.from_destination, false);
        }
        {
            let result = prober.parse(UNR_WITH_DATA.packet(), true).unwrap();
            println!("{:#?}", result);
            assert_eq!(result.destination.to_string(), "59.78.17.126");
            assert_eq!(result.responder.to_string(), "59.78.17.126");
            assert_eq!(result.distance, 6);
            assert_eq!(result.from_destination, true);
        }
    }
}
